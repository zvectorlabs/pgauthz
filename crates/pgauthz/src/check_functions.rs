//! Check resolution SQL functions for pgauthz extension.

use crate::cache;
use crate::errors::raise_authz_error;
use crate::guc;
use crate::validation::{raise_invalid_param, validate_check_args, validate_expand_args};
use authz_core::core_resolver::{CheckStrategy, CoreResolver};
use authz_core::dispatcher::{Dispatcher, LocalDispatcher};
use authz_core::policy_provider::StaticPolicyProvider;
use authz_core::resolver::{CheckResult, ResolveCheckRequest};
use authz_core::traits::RevisionReader;
use authz_datastore_pgx::PostgresDatastore;
use pgrx::prelude::*;
use std::collections::HashMap;

/// Internal implementation shared by both check entry points.
fn do_check(
    object_type: &str,
    object_id: &str,
    relation: &str,
    subject_type: &str,
    subject_id: &str,
    context: HashMap<String, serde_json::Value>,
) -> bool {
    let _span = tracing::info_span!(
        "pgauthz_check",
        authz.object_type = object_type,
        authz.object_id = object_id,
        authz.relation = relation,
        authz.subject_type = subject_type,
        authz.subject_id = subject_id,
        authz.has_context = !context.is_empty(),
        authz.result = tracing::field::Empty,
    )
    .entered();
    let start = std::time::Instant::now();

    if let Err(e) = validate_check_args(object_type, object_id, relation, subject_type, subject_id)
    {
        raise_invalid_param(&e);
    }

    // 1. Load latest model/type-system (cached)
    let ds = PostgresDatastore::new();
    let type_system = cache::load_typesystem_cached(&ds).unwrap_or_else(|e| raise_authz_error(&e));

    // 2. Read and quantize revision for cache keys
    let raw_revision =
        pollster::block_on(ds.read_latest_revision()).unwrap_or_else(|_| "0".to_string());
    let quantum_secs = guc::get_revision_quantization_secs();
    let revision = cache::quantize_revision(&raw_revision, quantum_secs);

    // Debug: Log what we're looking for
    pgrx::info!(
        "[DEBUG] pgauthz_check looking for {}#{} on {}",
        object_type,
        relation,
        object_id
    );
    if let Some(type_def) = type_system.get_type(object_type) {
        pgrx::info!(
            "[DEBUG]   Type '{}' has {} relations, {} permissions",
            object_type,
            type_def.relations.len(),
            type_def.permissions.len()
        );
        pgrx::info!(
            "[DEBUG]   Relations: {:?}",
            type_def
                .relations
                .iter()
                .map(|r| &r.name)
                .collect::<Vec<_>>()
        );
        pgrx::info!(
            "[DEBUG]   Permissions: {:?}",
            type_def
                .permissions
                .iter()
                .map(|p| &p.name)
                .collect::<Vec<_>>()
        );

        if let Some(_rel) = type_system.get_relation(object_type, relation) {
            pgrx::info!("[DEBUG]   FOUND relation/permission '{}'", relation);
        } else {
            pgrx::info!("[DEBUG]   NOT FOUND: relation/permission '{}'", relation);
        }
    } else {
        pgrx::info!("[DEBUG]   Type '{}' NOT FOUND in type system", object_type);
    }

    // 3. Get check strategy from GUC
    let strategy = match guc::get_check_strategy() {
        guc::CheckStrategy::Batch => CheckStrategy::Batch,
        guc::CheckStrategy::Parallel => CheckStrategy::Parallel,
    };

    // 4. Create CoreResolver with StaticPolicyProvider (single-tenant: one global policy)
    let policy_provider = StaticPolicyProvider::from_arc(type_system);
    let resolver = CoreResolver::new(ds, policy_provider)
        .with_strategy(strategy)
        .with_result_cache(cache::get_result_cache())
        .with_tuple_cache(cache::get_tuple_cache());

    // 5. Create LocalDispatcher
    let dispatcher = LocalDispatcher::new(resolver);

    // 6. Build ResolveCheckRequest with revision
    let mut request = ResolveCheckRequest::new(
        object_type.to_string(),
        object_id.to_string(),
        relation.to_string(),
        subject_type.to_string(),
        subject_id.to_string(),
    );
    request.context = context;
    request.at_revision = revision;

    // Keep a handle to the shared metadata counters so we can read them after dispatch
    let metadata = request.metadata.clone();

    // 7. Call dispatcher.dispatch_check()
    let result = pollster::block_on(dispatcher.dispatch_check(request))
        .unwrap_or_else(|e| raise_authz_error(&e));

    // 8. Record metrics
    let duration = start.elapsed().as_secs_f64();
    let is_allowed = result == CheckResult::Allowed;
    let result_str = if is_allowed { "allowed" } else { "denied" };
    _span.record("authz.result", result_str);
    crate::metrics::record_check(duration, result_str, object_type, relation);

    // 9. Record resolution metrics (depth reached, dispatch count, datastore queries)
    crate::metrics::record_resolution(
        metadata.get_max_depth_reached() as u64,
        metadata.get_dispatch_count() as u64,
        metadata.get_datastore_queries() as u64,
    );

    // 10. Return result == CheckResult::Allowed
    is_allowed
}

/// Check if subject has relation on resource.
#[pg_extern]
fn pgauthz_check(
    object_type: &str,
    object_id: &str,
    relation: &str,
    subject_type: &str,
    subject_id: &str,
) -> bool {
    do_check(
        object_type,
        object_id,
        relation,
        subject_type,
        subject_id,
        HashMap::new(),
    )
}

/// Check with optional JSON context for CEL condition evaluation.
#[pg_extern]
fn pgauthz_check_with_context(
    object_type: &str,
    object_id: &str,
    relation: &str,
    subject_type: &str,
    subject_id: &str,
    context_json: &str,
) -> bool {
    let context: HashMap<String, serde_json::Value> = serde_json::from_str(context_json)
        .unwrap_or_else(|e| {
            raise_invalid_param(&format!("invalid context JSON: {}", e));
        });
    do_check(
        object_type,
        object_id,
        relation,
        subject_type,
        subject_id,
        context,
    )
}

/// Expand permission tree for debugging.
#[pg_extern]
fn pgauthz_expand(object_type: &str, object_id: &str, relation: &str) -> String {
    let _span = tracing::info_span!(
        "pgauthz_expand",
        authz.object_type = object_type,
        authz.object_id = object_id,
        authz.relation = relation,
    )
    .entered();
    if let Err(e) = validate_expand_args(object_type, object_id, relation) {
        raise_invalid_param(&e);
    }

    // Load model/type-system (cached)
    let ds = PostgresDatastore::new();
    let type_system = match cache::load_typesystem_cached(&ds) {
        Ok(ts) => ts,
        Err(e) => return format!(r#"{{"error": "{}"}}"#, e.to_string()),
    };

    // Get the relation definition
    match type_system.get_relation(object_type, relation) {
        Some(rel_def) => {
            format!(
                r#"{{"object_type": "{}", "object_id": "{}", "relation": "{}", "expression": "{:?}"}}"#,
                object_type, object_id, relation, rel_def.expression
            )
        }
        None => {
            format!(
                r#"{{"error": "Relation '{}' not found on type '{}'"}}"#,
                relation, object_type
            )
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_check_functions_compile() {
        // Just ensure the functions compile
        assert!(true);
    }
}
