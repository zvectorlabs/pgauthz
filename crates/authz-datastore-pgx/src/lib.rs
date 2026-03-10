//! SPI-based datastore implementation for pgauthz extension.

use async_trait::async_trait;
use authz_core::error::AuthzError;
use authz_core::tenant_schema::{ChangelogEntry, ChangelogReader};
use authz_core::traits::{
    AuthorizationPolicy, ModelReader, ModelWriter, Pagination, RevisionReader, Tuple, TupleFilter,
    TupleReader, TupleWriter,
};

/// SPI-based datastore for global model.
#[derive(Debug, Clone, Default)]
pub struct PostgresDatastore;

impl PostgresDatastore {
    pub fn new() -> Self {
        Self
    }
}

// ---------------------------------------------------------------------------
// Stub implementations when pgx feature is disabled (for CI)
// ---------------------------------------------------------------------------

#[cfg(not(feature = "pgx"))]
fn stub_err() -> AuthzError {
    AuthzError::Internal("authz-datastore-pgx built without pgx feature".into())
}

#[cfg(not(feature = "pgx"))]
#[async_trait]
impl ModelReader for PostgresDatastore {
    async fn read_authorization_policy(
        &self,
        _id: &str,
    ) -> Result<Option<AuthorizationPolicy>, AuthzError> {
        Err(stub_err())
    }
    async fn read_latest_authorization_policy(
        &self,
    ) -> Result<Option<AuthorizationPolicy>, AuthzError> {
        Err(stub_err())
    }
    async fn list_authorization_policies(
        &self,
        _pagination: &Pagination,
    ) -> Result<Vec<AuthorizationPolicy>, AuthzError> {
        Err(stub_err())
    }
}

#[cfg(not(feature = "pgx"))]
#[async_trait]
impl ModelWriter for PostgresDatastore {
    async fn write_authorization_policy(
        &self,
        _policy: &AuthorizationPolicy,
    ) -> Result<String, AuthzError> {
        Err(stub_err())
    }
}

#[cfg(not(feature = "pgx"))]
#[async_trait]
impl TupleReader for PostgresDatastore {
    async fn read_tuples(&self, _filter: &TupleFilter) -> Result<Vec<Tuple>, AuthzError> {
        Err(stub_err())
    }
    async fn read_user_tuple(
        &self,
        _object_type: &str,
        _object_id: &str,
        _relation: &str,
        _subject_type: &str,
        _subject_id: &str,
    ) -> Result<Option<Tuple>, AuthzError> {
        Err(stub_err())
    }
    async fn read_userset_tuples(
        &self,
        _object_type: &str,
        _object_id: &str,
        _relation: &str,
    ) -> Result<Vec<Tuple>, AuthzError> {
        Err(stub_err())
    }
    async fn read_starting_with_user(
        &self,
        _subject_type: &str,
        _subject_id: &str,
    ) -> Result<Vec<Tuple>, AuthzError> {
        Err(stub_err())
    }
    async fn read_user_tuple_batch(
        &self,
        _object_type: &str,
        _object_id: &str,
        _relations: &[String],
        _subject_type: &str,
        _subject_id: &str,
    ) -> Result<Option<Tuple>, AuthzError> {
        Err(stub_err())
    }
}

#[cfg(not(feature = "pgx"))]
#[async_trait]
impl TupleWriter for PostgresDatastore {
    async fn write_tuples(
        &self,
        _writes: &[Tuple],
        _deletes: &[Tuple],
    ) -> Result<String, AuthzError> {
        Err(stub_err())
    }
}

#[cfg(not(feature = "pgx"))]
#[async_trait]
impl ChangelogReader for PostgresDatastore {
    async fn read_changes(
        &self,
        _object_type: &str,
        _after_ulid: Option<&str>,
        _page_size: usize,
    ) -> Result<Vec<ChangelogEntry>, AuthzError> {
        Err(stub_err())
    }
}

#[cfg(not(feature = "pgx"))]
#[async_trait]
impl RevisionReader for PostgresDatastore {
    async fn read_latest_revision(&self) -> Result<String, AuthzError> {
        Err(stub_err())
    }
}

// ---------------------------------------------------------------------------
// Real implementations when pgx feature is enabled
// ---------------------------------------------------------------------------

#[cfg(feature = "pgx")]
mod pgx_impl {
    use super::*;
    use authz_core;
    use authz_core::model_parser;
    use authz_core::model_validator;
    use pgrx::prelude::*;
    use pgrx::spi::{self, SpiError};

    fn q(s: &str) -> String {
        spi::quote_literal(s)
    }

    fn to_err(e: SpiError) -> AuthzError {
        AuthzError::Datastore(e.to_string())
    }

    #[async_trait]
    impl ModelReader for PostgresDatastore {
        async fn read_authorization_policy(
            &self,
            id: &str,
        ) -> Result<Option<AuthorizationPolicy>, AuthzError> {
            let sql = format!(
                "SELECT id::text AS id, definition FROM authz.authorization_policy WHERE id = {}",
                q(id),
            );
            let result = Spi::connect_mut(|client| {
                let table = client.update(&sql, Some(1), &[]).map_err(to_err)?;
                if table.is_empty() {
                    return Ok(None);
                }
                let row = table.first();
                let id: Option<String> = row.get(1).map_err(to_err)?;
                let definition: Option<String> = row.get(2).map_err(to_err)?;

                if let Some(ref def) = definition {
                    pgrx::info!(
                        "[DEBUG] Read model {} from DB, parsing...",
                        id.as_ref().unwrap_or(&"unknown".to_string())
                    );
                    if let Ok(parsed) = authz_core::model_parser::parse_dsl(def) {
                        pgrx::info!("[DEBUG]   Parsed {} types", parsed.type_defs.len());
                        for type_def in &parsed.type_defs {
                            pgrx::info!(
                                "[DEBUG]     Type: {} ({} relations, {} permissions)",
                                type_def.name,
                                type_def.relations.len(),
                                type_def.permissions.len()
                            );
                        }
                    }
                }

                Ok(id.and_then(|id| {
                    definition.map(|definition| AuthorizationPolicy { id, definition })
                }))
            });
            result
        }

        async fn read_latest_authorization_policy(
            &self,
        ) -> Result<Option<AuthorizationPolicy>, AuthzError> {
            let sql = "SELECT id::text AS id, definition FROM authz.authorization_policy ORDER BY id DESC LIMIT 1".to_string();
            let result = Spi::connect_mut(|client| {
                let table = client.update(&sql, Some(1), &[]).map_err(to_err)?;
                if table.is_empty() {
                    return Ok(None);
                }
                let row = table.first();
                let id: Option<String> = row.get(1).map_err(to_err)?;
                let definition: Option<String> = row.get(2).map_err(to_err)?;
                Ok(id.and_then(|id| {
                    definition.map(|definition| AuthorizationPolicy { id, definition })
                }))
            });
            result
        }

        async fn list_authorization_policies(
            &self,
            pagination: &Pagination,
        ) -> Result<Vec<AuthorizationPolicy>, AuthzError> {
            let limit = if pagination.page_size == 0 {
                100
            } else {
                pagination.page_size
            };
            let offset = pagination
                .continuation_token
                .as_deref()
                .and_then(|t| t.parse::<usize>().ok())
                .unwrap_or(0);
            let sql = format!(
                "SELECT id::text AS id, definition FROM authz.authorization_policy ORDER BY created_at LIMIT {} OFFSET {}",
                limit, offset,
            );
            let result = Spi::connect_mut(|client| {
                let table = client.update(&sql, None, &[]).map_err(to_err)?;
                let mut out = Vec::new();
                for htup in table {
                    let id: Option<String> = htup.get_by_name("id").map_err(to_err)?;
                    let definition: Option<String> =
                        htup.get_by_name("definition").map_err(to_err)?;
                    if let (Some(id), Some(definition)) = (id, definition) {
                        out.push(AuthorizationPolicy { id, definition });
                    }
                }
                Ok(out)
            });
            result
        }
    }

    #[async_trait]
    impl ModelWriter for PostgresDatastore {
        async fn write_authorization_policy(
            &self,
            policy: &AuthorizationPolicy,
        ) -> Result<String, AuthzError> {
            // Validate the policy definition using the parser
            let parsed_model = model_parser::parse_dsl(&policy.definition)
                .map_err(|e| AuthzError::ModelParse(format!("{e}")))?;

            pgrx::info!(
                "[DEBUG] Parsed model with {} types",
                parsed_model.type_defs.len()
            );
            for type_def in &parsed_model.type_defs {
                pgrx::info!("[DEBUG]   Type: {}", type_def.name);
                pgrx::info!("[DEBUG]     Relations: {}", type_def.relations.len());
                for rel in &type_def.relations {
                    pgrx::info!("[DEBUG]       - {}", rel.name);
                }
                pgrx::info!("[DEBUG]     Permissions: {}", type_def.permissions.len());
                for perm in &type_def.permissions {
                    pgrx::info!("[DEBUG]       - {} = {:?}", perm.name, perm.expression);
                }
            }

            // Run semantic model validation.
            if let Err(validation_errors) = model_validator::validate_model(&parsed_model) {
                let details = validation_errors
                    .into_iter()
                    .map(|e| e.message)
                    .collect::<Vec<_>>()
                    .join("; ");
                return Err(AuthzError::ModelValidation(details));
            }

            // Compile all conditions on write to reject invalid CEL syntax early.
            for condition in &parsed_model.condition_defs {
                authz_core::cel::compile(&condition.expression).map_err(|e| {
                    AuthzError::ModelValidation(format!(
                        "Invalid condition '{}': {}",
                        condition.name, e
                    ))
                })?;
            }

            // Generate ULID if empty
            let id = if policy.id.is_empty() {
                ulid::Ulid::new().to_string()
            } else {
                policy.id.clone()
            };
            let sql = format!(
                "INSERT INTO authz.authorization_policy (id, definition) VALUES ({}, {}) ON CONFLICT (id) DO UPDATE SET definition = EXCLUDED.definition",
                q(&id),
                q(&policy.definition),
            );
            Spi::run(&sql).map_err(to_err)?;
            Ok(id)
        }
    }

    fn tuple_from_row(
        object_type: String,
        object_id: String,
        relation: String,
        subject_type: String,
        subject_id: String,
        condition: Option<String>,
    ) -> Tuple {
        Tuple {
            object_type,
            object_id,
            relation,
            subject_type,
            subject_id,
            condition,
        }
    }

    #[async_trait]
    impl TupleReader for PostgresDatastore {
        async fn read_tuples(&self, filter: &TupleFilter) -> Result<Vec<Tuple>, AuthzError> {
            let mut conditions = vec![];
            if let Some(ref v) = filter.object_type {
                conditions.push(format!("object_type = {}", q(v)));
            }
            if let Some(ref v) = filter.object_id {
                conditions.push(format!("object_id = {}", q(v)));
            }
            if let Some(ref v) = filter.relation {
                conditions.push(format!("relation = {}", q(v)));
            }
            if let Some(ref v) = filter.subject_type {
                conditions.push(format!("subject_type = {}", q(v)));
            }
            if let Some(ref v) = filter.subject_id {
                conditions.push(format!("subject_id = {}", q(v)));
            }
            let where_clause = conditions.join(" AND ");
            let sql = if where_clause.is_empty() {
                "SELECT object_type, object_id, relation, subject_type, subject_id, condition FROM authz.tuple".to_string()
            } else {
                format!(
                    "SELECT object_type, object_id, relation, subject_type, subject_id, condition FROM authz.tuple WHERE {}",
                    where_clause,
                )
            };
            let result = Spi::connect_mut(|client| {
                let table = client.update(&sql, None, &[]).map_err(to_err)?;
                let mut out = Vec::new();
                for htup in table {
                    let ot: Option<String> = htup.get_by_name("object_type").map_err(to_err)?;
                    let oid: Option<String> = htup.get_by_name("object_id").map_err(to_err)?;
                    let rel: Option<String> = htup.get_by_name("relation").map_err(to_err)?;
                    let st: Option<String> = htup.get_by_name("subject_type").map_err(to_err)?;
                    let sid: Option<String> = htup.get_by_name("subject_id").map_err(to_err)?;
                    let cond: Option<String> = htup.get_by_name("condition").map_err(to_err)?;
                    if let (Some(ot), Some(oid), Some(rel), Some(st), Some(sid)) =
                        (ot, oid, rel, st, sid)
                    {
                        out.push(tuple_from_row(ot, oid, rel, st, sid, cond));
                    }
                }
                Ok(out)
            });
            result
        }

        async fn read_user_tuple(
            &self,
            object_type: &str,
            object_id: &str,
            relation: &str,
            subject_type: &str,
            subject_id: &str,
        ) -> Result<Option<Tuple>, AuthzError> {
            let sql = format!(
                "SELECT object_type, object_id, relation, subject_type, subject_id, condition FROM authz.tuple WHERE object_type = {} AND object_id = {} AND relation = {} AND subject_type = {} AND subject_id = {}",
                q(object_type),
                q(object_id),
                q(relation),
                q(subject_type),
                q(subject_id),
            );
            let result = Spi::connect_mut(|client| {
                let table = client.update(&sql, Some(1), &[]).map_err(to_err)?;
                if table.is_empty() {
                    return Ok(None);
                }
                let row = table.first();
                let ot: Option<String> = row.get(1).map_err(to_err)?;
                let oid: Option<String> = row.get(2).map_err(to_err)?;
                let rel: Option<String> = row.get(3).map_err(to_err)?;
                let st: Option<String> = row.get(4).map_err(to_err)?;
                let sid: Option<String> = row.get(5).map_err(to_err)?;
                let cond: Option<String> = row.get(6).map_err(to_err)?;
                Ok(ot.and_then(|ot| {
                    oid.and_then(|oid| {
                        rel.and_then(|rel| {
                            st.and_then(|st| {
                                sid.map(|sid| tuple_from_row(ot, oid, rel, st, sid, cond))
                            })
                        })
                    })
                }))
            });
            result
        }

        async fn read_userset_tuples(
            &self,
            object_type: &str,
            object_id: &str,
            relation: &str,
        ) -> Result<Vec<Tuple>, AuthzError> {
            let sql = format!(
                "SELECT object_type, object_id, relation, subject_type, subject_id, condition FROM authz.tuple WHERE object_type = {} AND object_id = {} AND relation = {}",
                q(object_type),
                q(object_id),
                q(relation),
            );
            let result = Spi::connect_mut(|client| {
                let table = client.update(&sql, None, &[]).map_err(to_err)?;
                let mut out = Vec::new();
                for htup in table {
                    let ot: Option<String> = htup.get_by_name("object_type").map_err(to_err)?;
                    let oid: Option<String> = htup.get_by_name("object_id").map_err(to_err)?;
                    let rel: Option<String> = htup.get_by_name("relation").map_err(to_err)?;
                    let st: Option<String> = htup.get_by_name("subject_type").map_err(to_err)?;
                    let sid: Option<String> = htup.get_by_name("subject_id").map_err(to_err)?;
                    let cond: Option<String> = htup.get_by_name("condition").map_err(to_err)?;
                    if let (Some(ot), Some(oid), Some(rel), Some(st), Some(sid)) =
                        (ot, oid, rel, st, sid)
                    {
                        out.push(tuple_from_row(ot, oid, rel, st, sid, cond));
                    }
                }
                Ok(out)
            });
            result
        }

        async fn read_starting_with_user(
            &self,
            subject_type: &str,
            subject_id: &str,
        ) -> Result<Vec<Tuple>, AuthzError> {
            let sql = format!(
                "SELECT object_type, object_id, relation, subject_type, subject_id, condition FROM authz.tuple WHERE subject_type = {} AND subject_id = {}",
                q(subject_type),
                q(subject_id),
            );
            let result = Spi::connect_mut(|client| {
                let table = client.update(&sql, None, &[]).map_err(to_err)?;
                let mut out = Vec::new();
                for htup in table {
                    let ot: Option<String> = htup.get_by_name("object_type").map_err(to_err)?;
                    let oid: Option<String> = htup.get_by_name("object_id").map_err(to_err)?;
                    let rel: Option<String> = htup.get_by_name("relation").map_err(to_err)?;
                    let st: Option<String> = htup.get_by_name("subject_type").map_err(to_err)?;
                    let sid: Option<String> = htup.get_by_name("subject_id").map_err(to_err)?;
                    let cond: Option<String> = htup.get_by_name("condition").map_err(to_err)?;
                    if let (Some(ot), Some(oid), Some(rel), Some(st), Some(sid)) =
                        (ot, oid, rel, st, sid)
                    {
                        out.push(tuple_from_row(ot, oid, rel, st, sid, cond));
                    }
                }
                Ok(out)
            });
            result
        }

        async fn read_user_tuple_batch(
            &self,
            object_type: &str,
            object_id: &str,
            relations: &[String],
            subject_type: &str,
            subject_id: &str,
        ) -> Result<Option<Tuple>, AuthzError> {
            if relations.is_empty() {
                return Ok(None);
            }

            // Build IN clause for relations
            let relations_quoted: Vec<String> = relations.iter().map(|r| q(r)).collect();
            let relations_in = relations_quoted.join(", ");

            let sql = format!(
                "SELECT object_type, object_id, relation, subject_type, subject_id, condition FROM authz.tuple WHERE object_type = {} AND object_id = {} AND relation IN ({}) AND subject_type = {} AND subject_id = {} LIMIT 1",
                q(object_type),
                q(object_id),
                relations_in,
                q(subject_type),
                q(subject_id),
            );

            let result = Spi::connect_mut(|client| {
                let table = client.update(&sql, Some(1), &[]).map_err(to_err)?;
                if table.is_empty() {
                    return Ok(None);
                }
                let row = table.first();
                let ot: Option<String> = row.get(1).map_err(to_err)?;
                let oid: Option<String> = row.get(2).map_err(to_err)?;
                let rel: Option<String> = row.get(3).map_err(to_err)?;
                let st: Option<String> = row.get(4).map_err(to_err)?;
                let sid: Option<String> = row.get(5).map_err(to_err)?;
                let cond: Option<String> = row.get(6).map_err(to_err)?;
                Ok(ot.and_then(|ot| {
                    oid.and_then(|oid| {
                        rel.and_then(|rel| {
                            st.and_then(|st| {
                                sid.map(|sid| tuple_from_row(ot, oid, rel, st, sid, cond))
                            })
                        })
                    })
                }))
            });
            result
        }
    }

    // Helper function to extract allowed types from a relation expression
    fn extract_allowed_types(expr: &authz_core::model_ast::RelationExpr) -> Vec<&str> {
        use authz_core::model_ast::{AssignableTarget, RelationExpr};

        match expr {
            RelationExpr::DirectAssignment(targets) => targets
                .iter()
                .filter_map(|t| match t {
                    AssignableTarget::Type(type_name) => Some(type_name.as_str()),
                    AssignableTarget::Userset { type_name, .. } => Some(type_name.as_str()),
                    _ => None,
                })
                .collect(),
            RelationExpr::Union(exprs) => exprs.iter().flat_map(extract_allowed_types).collect(),
            RelationExpr::Intersection(exprs) => {
                exprs.iter().flat_map(extract_allowed_types).collect()
            }
            RelationExpr::Exclusion { base, .. } => extract_allowed_types(base),
            _ => Vec::new(), // ComputedUserset and TupleToUserset don't define direct types
        }
    }

    #[async_trait]
    impl TupleWriter for PostgresDatastore {
        async fn write_tuples(
            &self,
            writes: &[Tuple],
            deletes: &[Tuple],
        ) -> Result<String, AuthzError> {
            // Validate tuples against the authorization model schema
            if !writes.is_empty() {
                let model_opt = self.read_latest_authorization_policy().await?;
                if let Some(model) = model_opt {
                    let ast = model_parser::parse_dsl(&model.definition)
                        .map_err(|e| AuthzError::ModelParse(format!("{e}")))?;

                    for tuple in writes {
                        // Find the type definition for this object_type
                        let type_def = ast
                            .type_defs
                            .iter()
                            .find(|t| t.name == tuple.object_type)
                            .ok_or_else(|| {
                            AuthzError::RelationshipValidation(format!(
                                "object_type '{}' not defined in authorization model",
                                tuple.object_type
                            ))
                        })?;

                        // Check if the relation exists in this type
                        let relation_def = type_def.relations.iter()
                            .find(|r| r.name == tuple.relation)
                            .ok_or_else(|| AuthzError::RelationshipValidation(format!(
                                "relation '{}' not defined for type '{}' in authorization model",
                                tuple.relation,
                                tuple.object_type
                            )))?;

                        // Validate subject_type against the relation's assignable targets
                        let allowed_types = extract_allowed_types(&relation_def.expression);
                        if !allowed_types.is_empty()
                            && !allowed_types.contains(&tuple.subject_type.as_str())
                        {
                            return Err(AuthzError::RelationshipValidation(format!(
                                "subject_type '{}' not allowed for relation '{}' on type '{}'. Allowed types: {:?}",
                                tuple.subject_type,
                                tuple.relation,
                                tuple.object_type,
                                allowed_types
                            )));
                        }
                    }
                }
                // If no model exists yet, allow writes (bootstrap case)
            }

            let revision_id = ulid::Ulid::new().to_string();
            Spi::connect_mut(|client| {
                for t in deletes {
                    let sql = format!(
                        "DELETE FROM authz.tuple WHERE object_type = {} AND object_id = {} AND relation = {} AND subject_type = {} AND subject_id = {}",
                        q(&t.object_type),
                        q(&t.object_id),
                        q(&t.relation),
                        q(&t.subject_type),
                        q(&t.subject_id),
                    );
                    client.update(&sql, None, &[]).map_err(to_err)?;
                    let ulid = ulid::Ulid::new().to_string();
                    let sql = format!(
                        "INSERT INTO authz.changelog (object_type, object_id, relation, subject_type, subject_id, operation, ulid) VALUES ({}, {}, {}, {}, {}, 'delete', {})",
                        q(&t.object_type),
                        q(&t.object_id),
                        q(&t.relation),
                        q(&t.subject_type),
                        q(&t.subject_id),
                        q(&ulid),
                    );
                    client.update(&sql, None, &[]).map_err(to_err)?;
                }
                for t in writes {
                    let cond = t
                        .condition
                        .as_ref()
                        .map(|c| q(c))
                        .unwrap_or_else(|| "NULL".to_string());
                    let sql = format!(
                        "INSERT INTO authz.tuple (object_type, object_id, relation, subject_type, subject_id, condition) VALUES ({}, {}, {}, {}, {}, {}) ON CONFLICT (object_type, object_id, relation, subject_type, subject_id) DO UPDATE SET condition = EXCLUDED.condition",
                        q(&t.object_type),
                        q(&t.object_id),
                        q(&t.relation),
                        q(&t.subject_type),
                        q(&t.subject_id),
                        cond,
                    );
                    client.update(&sql, None, &[]).map_err(to_err)?;
                    let ulid = ulid::Ulid::new().to_string();
                    let sql = format!(
                        "INSERT INTO authz.changelog (object_type, object_id, relation, subject_type, subject_id, operation, ulid) VALUES ({}, {}, {}, {}, {}, 'write', {})",
                        q(&t.object_type),
                        q(&t.object_id),
                        q(&t.relation),
                        q(&t.subject_type),
                        q(&t.subject_id),
                        q(&ulid),
                    );
                    client.update(&sql, None, &[]).map_err(to_err)?;
                }
                let sql = format!(
                    "INSERT INTO authz.revision (revision_id) VALUES ({})",
                    q(&revision_id),
                );
                client.update(&sql, None, &[]).map_err(to_err)?;
                Ok::<(), AuthzError>(())
            })?;
            Ok(revision_id)
        }
    }

    #[async_trait]
    impl ChangelogReader for PostgresDatastore {
        async fn read_changes(
            &self,
            object_type: &str,
            after_ulid: Option<&str>,
            page_size: usize,
        ) -> Result<Vec<ChangelogEntry>, AuthzError> {
            let limit = page_size.max(1).min(100);
            let ulid_cond = after_ulid
                .map(|u| format!(" AND ulid > {}", q(u)))
                .unwrap_or_default();
            let sql = format!(
                "SELECT object_type, object_id, relation, subject_type, subject_id, operation, ulid FROM authz.changelog WHERE object_type = {} {} ORDER BY ulid LIMIT {}",
                q(object_type),
                ulid_cond,
                limit,
            );
            let result = Spi::connect_mut(|client| {
                let table = client.update(&sql, None, &[]).map_err(to_err)?;
                let mut out = Vec::new();
                for htup in table {
                    let ot: Option<String> = htup.get_by_name("object_type").map_err(to_err)?;
                    let oid: Option<String> = htup.get_by_name("object_id").map_err(to_err)?;
                    let rel: Option<String> = htup.get_by_name("relation").map_err(to_err)?;
                    let st: Option<String> = htup.get_by_name("subject_type").map_err(to_err)?;
                    let sid: Option<String> = htup.get_by_name("subject_id").map_err(to_err)?;
                    let op: Option<String> = htup.get_by_name("operation").map_err(to_err)?;
                    let ulid: Option<String> = htup.get_by_name("ulid").map_err(to_err)?;
                    if let (
                        Some(ot),
                        Some(oid),
                        Some(rel),
                        Some(st),
                        Some(sid),
                        Some(op),
                        Some(ulid),
                    ) = (ot, oid, rel, st, sid, op, ulid)
                    {
                        out.push(ChangelogEntry {
                            object_type: ot,
                            object_id: oid,
                            relation: rel,
                            subject_type: st,
                            subject_id: sid,
                            operation: op,
                            ulid,
                        });
                    }
                }
                Ok(out)
            });
            result
        }
    }

    #[async_trait]
    impl RevisionReader for PostgresDatastore {
        async fn read_latest_revision(&self) -> Result<String, AuthzError> {
            let sql = "SELECT revision_id FROM authz.revision ORDER BY created_at DESC LIMIT 1";
            let result = Spi::connect_mut(|client| {
                let table = client.update(sql, None, &[]).map_err(to_err)?;
                for htup in table {
                    let revision_id: Option<String> =
                        htup.get_by_name("revision_id").map_err(to_err)?;
                    return Ok(revision_id.unwrap_or_else(|| "0".to_string()));
                }
                // No rows found (bootstrap case)
                Ok("0".to_string())
            });
            result
        }
    }
}
