//! Request validation helpers for SQL entrypoints.

use pgrx::PgSqlErrorCode;

const MAX_TYPE_LEN: usize = 255;
const MAX_ID_LEN: usize = 1000;

fn ensure_non_empty(field: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{} must not be empty", field));
    }
    Ok(())
}

pub(crate) fn validate_expand_args(
    object_type: &str,
    object_id: &str,
    relation: &str,
) -> Result<(), String> {
    ensure_non_empty("object_type", object_type)?;
    ensure_non_empty("object_id", object_id)?;
    ensure_non_empty("relation", relation)?;

    ensure_max_len("object_type", object_type, MAX_TYPE_LEN)?;
    ensure_max_len("relation", relation, MAX_TYPE_LEN)?;
    ensure_max_len("object_id", object_id, MAX_ID_LEN)?;

    ensure_identifier_chars("object_type", object_type)?;
    ensure_identifier_chars("relation", relation)?;
    Ok(())
}

pub(crate) fn validate_continuation_token(token: Option<&str>) -> Result<(), String> {
    if let Some(t) = token {
        ensure_max_len("continuation_token", t, MAX_ID_LEN)?;
    }
    Ok(())
}

pub(crate) fn validate_read_changes_args(object_type: &str, page_size: i32) -> Result<(), String> {
    ensure_non_empty("object_type", object_type)?;
    ensure_max_len("object_type", object_type, MAX_TYPE_LEN)?;
    ensure_identifier_chars("object_type", object_type)?;
    validate_page_size(page_size)
}

fn ensure_max_len(field: &str, value: &str, max: usize) -> Result<(), String> {
    if value.len() > max {
        return Err(format!("{} exceeds max length {}", field, max));
    }
    Ok(())
}

fn ensure_identifier_chars(field: &str, value: &str) -> Result<(), String> {
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == ':' || c == '#')
    {
        return Err(format!("{} contains invalid characters", field));
    }
    Ok(())
}

pub(crate) fn validate_check_args(
    object_type: &str,
    object_id: &str,
    relation: &str,
    subject_type: &str,
    subject_id: &str,
) -> Result<(), String> {
    ensure_non_empty("object_type", object_type)?;
    ensure_non_empty("object_id", object_id)?;
    ensure_non_empty("relation", relation)?;
    ensure_non_empty("subject_type", subject_type)?;
    ensure_non_empty("subject_id", subject_id)?;

    ensure_max_len("object_type", object_type, MAX_TYPE_LEN)?;
    ensure_max_len("relation", relation, MAX_TYPE_LEN)?;
    ensure_max_len("subject_type", subject_type, MAX_TYPE_LEN)?;
    ensure_max_len("object_id", object_id, MAX_ID_LEN)?;
    ensure_max_len("subject_id", subject_id, MAX_ID_LEN)?;

    ensure_identifier_chars("object_type", object_type)?;
    ensure_identifier_chars("relation", relation)?;
    ensure_identifier_chars("subject_type", subject_type)?;
    Ok(())
}

pub(crate) fn validate_page_size(page_size: i32) -> Result<(), String> {
    if !(1..=1000).contains(&page_size) {
        return Err("page_size must be between 1 and 1000".to_string());
    }
    Ok(())
}

pub(crate) fn raise_invalid_param(message: &str) -> ! {
    // Record validation error metric before raising
    crate::metrics::record_error("invalid_parameter", "validation");

    pgrx::ereport!(
        pgrx::PgLogLevel::ERROR,
        PgSqlErrorCode::ERRCODE_INVALID_PARAMETER_VALUE,
        message
    );
    // After ereport, control flow should not continue
    unreachable!("ereport should have terminated execution")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_check_args_ok() {
        let result = validate_check_args("document", "doc1", "viewer", "user", "alice");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_check_args_empty_type() {
        let result = validate_check_args("", "doc1", "viewer", "user", "alice");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("object_type must not be empty")
        );
    }

    #[test]
    fn test_validate_check_args_too_long() {
        let long_type = "a".repeat(300);
        let result = validate_check_args(&long_type, "doc1", "viewer", "user", "alice");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("exceeds max length"));
    }

    #[test]
    fn test_validate_check_args_invalid_chars() {
        let result = validate_check_args("doc@ument!", "doc1", "viewer", "user", "alice");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid characters"));
    }

    #[test]
    fn test_validate_page_size_boundaries() {
        assert!(validate_page_size(0).is_err());
        assert!(validate_page_size(1).is_ok());
        assert!(validate_page_size(500).is_ok());
        assert!(validate_page_size(1000).is_ok());
        assert!(validate_page_size(1001).is_err());
    }

    #[test]
    fn test_validate_continuation_token_too_long() {
        let long_token = "a".repeat(1100);
        let result = validate_continuation_token(Some(&long_token));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("exceeds max length"));
    }
}
