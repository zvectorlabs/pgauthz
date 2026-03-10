//! List operations for finding objects and subjects with permissions.

use crate::cache;
use crate::errors::raise_authz_error;
use crate::guc;
use crate::validation::{
    raise_invalid_param, validate_check_args, validate_continuation_token, validate_page_size,
};
use authz_core::core_resolver::{CheckStrategy, CoreResolver};
use authz_core::dispatcher::{Dispatcher, LocalDispatcher};
use authz_core::model_ast::RelationExpr;
use authz_core::policy_provider::StaticPolicyProvider;
use authz_core::resolver::{CheckResult, ResolveCheckRequest};
use authz_core::traits::{RevisionReader, TupleFilter, TupleReader};
use authz_core::type_system::TypeSystem;
use authz_datastore_pgx::PostgresDatastore;
use pgrx::prelude::*;

/// Extract relation names from permission expressions for list_subjects
fn extract_relation_names_from_exprs(
    exprs: &[RelationExpr],
    _type_system: &TypeSystem,
    _object_type: &str,
    relations: &mut Vec<String>,
) {
    for expr in exprs {
        match expr {
            RelationExpr::ComputedUserset(relation_name) => {
                relations.push(relation_name.clone());
            }
            RelationExpr::Union(nested_exprs) => {
                extract_relation_names_from_exprs(
                    nested_exprs,
                    _type_system,
                    _object_type,
                    relations,
                );
            }
            RelationExpr::Intersection(_) => {
                // Skip intersection for now - complex case
            }
            RelationExpr::Exclusion { base, .. } => {
                extract_relation_names_from_exprs(
                    &[base.as_ref().clone()],
                    _type_system,
                    _object_type,
                    relations,
                );
            }
            _ => {
                // Skip DirectAssignment and TupleToUserset for list_subjects
            }
        }
    }
}

/// List all objects of a given type that a subject has a specific relation to.
/// Returns object IDs that the subject can access.
#[pg_extern]
fn pgauthz_list_objects(
    subject_type: &str,
    subject_id: &str,
    relation: &str,
    object_type: &str,
    page_size: default!(i32, 100),
    continuation_token: default!(Option<String>, "NULL"),
) -> Vec<String> {
    let _span = tracing::info_span!(
        "pgauthz_list_objects",
        authz.subject_type = subject_type,
        authz.subject_id = subject_id,
        authz.relation = relation,
        authz.object_type = object_type,
        authz.page_size = page_size,
        authz.result_count = tracing::field::Empty,
    )
    .entered();
    let start = std::time::Instant::now();
    if let Err(e) = validate_check_args(
        object_type,
        "placeholder",
        relation,
        subject_type,
        subject_id,
    ) {
        raise_invalid_param(&e.replace("object_id", "object_id/continuation_token"));
    }
    if let Err(e) = validate_page_size(page_size) {
        raise_invalid_param(&e);
    }
    if let Err(e) = validate_continuation_token(continuation_token.as_deref()) {
        raise_invalid_param(&e);
    }

    // Load model/type-system (cached)
    let ds = PostgresDatastore::new();
    let type_system = cache::load_typesystem_cached(&ds).unwrap_or_else(|e| raise_authz_error(&e));

    // Read and quantize revision for cache keys
    let raw_revision =
        pollster::block_on(ds.read_latest_revision()).unwrap_or_else(|_| "0".to_string());
    let quantum_secs = guc::get_revision_quantization_secs();
    let revision = cache::quantize_revision(&raw_revision, quantum_secs);

    // Reuse check strategy configuration so ListObjects follows the same traversal behavior.
    let strategy = match guc::get_check_strategy() {
        guc::CheckStrategy::Batch => CheckStrategy::Batch,
        guc::CheckStrategy::Parallel => CheckStrategy::Parallel,
    };

    let resolver = CoreResolver::new(
        ds.clone(),
        StaticPolicyProvider::from_arc(type_system.clone()),
    )
    .with_strategy(strategy)
    .with_result_cache(cache::get_result_cache())
    .with_tuple_cache(cache::get_tuple_cache());
    let dispatcher = LocalDispatcher::new(resolver);

    // Candidate seeding: scan all tuples for the requested object_type, then verify each
    // candidate object via dispatch_check. This reuses full graph traversal semantics
    // (usersets, tuple-to-userset, setops) instead of brittle relation extraction.
    let filter = TupleFilter {
        object_type: Some(object_type.to_string()),
        object_id: None,
        relation: None,
        subject_type: None,
        subject_id: None,
    };

    let all_tuples =
        pollster::block_on(ds.read_tuples(&filter)).unwrap_or_else(|e| raise_authz_error(&e));

    pgrx::info!(
        "[DEBUG] list_objects candidate scan for {}#{}@{}:{} read {} tuples",
        object_type,
        relation,
        subject_type,
        subject_id,
        all_tuples.len()
    );
    pgrx::info!("[DEBUG] list_objects candidate tuples: {:?}", all_tuples);

    // Collect unique object IDs
    let mut object_ids: Vec<String> = all_tuples
        .iter()
        .map(|t| t.object_id.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    object_ids.sort();

    pgrx::info!(
        "[DEBUG] list_objects candidate object_ids: {:?}",
        object_ids
    );

    let userset_probe = TupleFilter {
        object_type: Some("group".to_string()),
        object_id: None,
        relation: Some("member".to_string()),
        subject_type: Some(subject_type.to_string()),
        subject_id: Some(subject_id.to_string()),
    };
    let userset_probe_tuples = pollster::block_on(ds.read_tuples(&userset_probe))
        .unwrap_or_else(|e| raise_authz_error(&e));
    pgrx::info!(
        "[DEBUG] list_objects userset probe group#member for {}:{} => {} tuples",
        subject_type,
        subject_id,
        userset_probe_tuples.len()
    );

    // Apply pagination: start after continuation token
    let start_idx = if let Some(ref token) = continuation_token {
        object_ids.binary_search(token).map(|i| i + 1).unwrap_or(0)
    } else {
        0
    };

    // Limit to page_size
    let end_idx = std::cmp::min(start_idx + page_size.max(1) as usize, object_ids.len());
    let paginated_ids = &object_ids[start_idx..end_idx];

    let mut accessible_objects = Vec::new();

    for object_id in paginated_ids {
        let mut request = ResolveCheckRequest::new(
            object_type.to_string(),
            object_id.clone(),
            relation.to_string(),
            subject_type.to_string(),
            subject_id.to_string(),
        );
        request.at_revision = revision.clone();

        let result = pollster::block_on(dispatcher.dispatch_check(request))
            .unwrap_or_else(|e| raise_authz_error(&e));

        pgrx::info!(
            "[DEBUG] list_objects check {}:{}#{}@{}:{} => {:?}",
            object_type,
            object_id,
            relation,
            subject_type,
            subject_id,
            result
        );

        if result == CheckResult::Allowed {
            accessible_objects.push(object_id.clone());
        }
    }

    let duration = start.elapsed().as_secs_f64();
    _span.record("authz.result_count", accessible_objects.len());
    crate::metrics::record_check(duration, "list_objects", object_type, relation);
    accessible_objects
}

/// List all subjects of a given type that have a specific relation to an object.
/// Returns subject IDs that have access to the object.
#[pg_extern]
fn pgauthz_list_subjects(
    object_type: &str,
    object_id: &str,
    relation: &str,
    subject_type: &str,
    page_size: default!(i32, 100),
    continuation_token: default!(Option<String>, "NULL"),
) -> Vec<String> {
    let _span = tracing::info_span!(
        "pgauthz_list_subjects",
        authz.object_type = object_type,
        authz.object_id = object_id,
        authz.relation = relation,
        authz.subject_type = subject_type,
        authz.page_size = page_size,
        authz.result_count = tracing::field::Empty,
    )
    .entered();
    let start = std::time::Instant::now();
    if let Err(e) = validate_check_args(
        object_type,
        object_id,
        relation,
        subject_type,
        "placeholder",
    ) {
        raise_invalid_param(&e.replace("subject_id", "subject_id/continuation_token"));
    }
    if let Err(e) = validate_page_size(page_size) {
        raise_invalid_param(&e);
    }
    if let Err(e) = validate_continuation_token(continuation_token.as_deref()) {
        raise_invalid_param(&e);
    }

    // Load model/type-system (cached)
    let ds = PostgresDatastore::new();
    let type_system = cache::load_typesystem_cached(&ds).unwrap_or_else(|e| raise_authz_error(&e));

    // Read and quantize revision for cache keys
    let raw_revision =
        pollster::block_on(ds.read_latest_revision()).unwrap_or_else(|_| "0".to_string());
    let quantum_secs = guc::get_revision_quantization_secs();
    let _revision = cache::quantize_revision(&raw_revision, quantum_secs);

    // Check if relation is a permission or relation
    let object_type_def = match type_system.get_type(object_type) {
        Some(def) => def,
        None => raise_invalid_param(&format!("Type '{}' not found", object_type)),
    };

    let relations_to_check = if let Some(permission) = object_type_def
        .permissions
        .iter()
        .find(|p| p.name == relation)
    {
        // It's a permission - extract relations from the permission expression
        let mut relations = Vec::new();
        extract_relation_names_from_exprs(
            std::slice::from_ref(&permission.expression),
            &type_system,
            object_type,
            &mut relations,
        );
        relations
    } else {
        // It's a direct relation
        vec![relation.to_string()]
    };

    pgrx::info!(
        "[DEBUG] list_subjects checking relations: {:?}",
        relations_to_check
    );

    // Read tuples for all relevant relations
    let mut all_tuples = Vec::new();
    for rel in &relations_to_check {
        let filter = TupleFilter {
            object_type: Some(object_type.to_string()),
            object_id: Some(object_id.to_string()),
            relation: Some(rel.clone()),
            subject_type: Some(subject_type.to_string()),
            subject_id: None,
        };

        let tuples =
            pollster::block_on(ds.read_tuples(&filter)).unwrap_or_else(|e| raise_authz_error(&e));
        all_tuples.extend(tuples);
    }

    let tuples = all_tuples;

    pgrx::info!(
        "[DEBUG] list_subjects candidate scan for {}:{}#{}@{} read {} tuples",
        object_type,
        object_id,
        relation,
        subject_type,
        tuples.len()
    );
    pgrx::info!("[DEBUG] list_subjects candidate tuples: {:?}", tuples);

    // Collect unique subject IDs directly from tuples
    let mut subject_ids: Vec<String> = tuples
        .iter()
        .map(|t| t.subject_id.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    subject_ids.sort();

    pgrx::info!(
        "[DEBUG] list_subjects direct tuples found subject_ids: {:?}",
        subject_ids
    );

    // Apply pagination: start after continuation token
    let start_idx = if let Some(ref token) = continuation_token {
        subject_ids.binary_search(token).map(|i| i + 1).unwrap_or(0)
    } else {
        0
    };

    // Limit to page_size
    let end_idx = std::cmp::min(start_idx + page_size.max(1) as usize, subject_ids.len());
    let paginated_ids = &subject_ids[start_idx..end_idx];

    let result = paginated_ids.to_vec();
    let duration = start.elapsed().as_secs_f64();
    _span.record("authz.result_count", result.len());
    crate::metrics::record_check(duration, "list_subjects", object_type, relation);
    result
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_list_functions_compile() {
        // Just ensure the functions compile
        assert!(true);
    }
}
