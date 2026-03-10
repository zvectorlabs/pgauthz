//! PostgreSQL error mapping helpers.

use authz_core::error::AuthzError;
use pgrx::{PgLogLevel, PgSqlErrorCode, ereport};

/// Map an AuthzError to the appropriate PostgreSQL SQLSTATE code and raise via ereport.
pub(crate) fn raise_authz_error(err: &AuthzError) -> ! {
    let (error_type, code, msg) = match err {
        AuthzError::Validation { .. } => (
            "validation",
            PgSqlErrorCode::ERRCODE_INVALID_PARAMETER_VALUE, // 22023
            err.to_string(),
        ),
        AuthzError::ModelParse(_) => (
            "model_parse",
            PgSqlErrorCode::ERRCODE_DATA_EXCEPTION, // 22000
            err.to_string(),
        ),
        AuthzError::ModelValidation(_) => (
            "model_validation",
            PgSqlErrorCode::ERRCODE_CHECK_VIOLATION, // 23514
            err.to_string(),
        ),
        AuthzError::ModelNotFound => (
            "model_not_found",
            PgSqlErrorCode::ERRCODE_NO_DATA_FOUND, // 02000
            err.to_string(),
        ),
        AuthzError::RelationshipValidation(_) => (
            "relationship_validation",
            PgSqlErrorCode::ERRCODE_CHECK_VIOLATION, // 23514
            err.to_string(),
        ),
        AuthzError::RelationNotFound { .. } => (
            "relation_not_found",
            PgSqlErrorCode::ERRCODE_UNDEFINED_OBJECT, // 42704
            err.to_string(),
        ),
        AuthzError::MaxDepthExceeded => (
            "max_depth_exceeded",
            PgSqlErrorCode::ERRCODE_PROGRAM_LIMIT_EXCEEDED, // 54000
            err.to_string(),
        ),
        AuthzError::Datastore(_) => (
            "datastore",
            PgSqlErrorCode::ERRCODE_EXTERNAL_ROUTINE_EXCEPTION, // 38000
            err.to_string(),
        ),
        AuthzError::ResolutionError(_) | AuthzError::CachePoisoned | AuthzError::Internal(_) => (
            "internal",
            PgSqlErrorCode::ERRCODE_INTERNAL_ERROR, // XX000
            err.to_string(),
        ),
    };

    // Record error metric before raising
    crate::metrics::record_error(error_type, "authz_operation");

    ereport!(PgLogLevel::ERROR, code, msg);
    unreachable!("ereport should have terminated execution")
}
