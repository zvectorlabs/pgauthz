//! Integration test: CREATE EXTENSION pgauthz and pgauthz_check stub.
//!
//! Requires: `cargo pgrx init` and Docker (for testcontainers).
//! Run: `cargo test --test integration_pgauthz` (with Docker)

/// Phase 0: Verify authz-core types work (no Postgres required).
#[test]
fn test_authz_core_types() {
    use authz_core::resolver::{CheckResult, ResolveCheckRequest};
    assert_eq!(CheckResult::Allowed, CheckResult::Allowed);
    let _req = ResolveCheckRequest::new(
        "document".into(),
        "1".into(),
        "viewer".into(),
        "user".into(),
        "alice".into(),
    );
}

/// Full E2E requires: cargo pgrx init, then cargo pgx run pg16.
/// In psql: CREATE EXTENSION pgauthz; SELECT pgauthz_check('','','','','','','');
/// This test is a placeholder; run manually for now.
#[test]
#[ignore = "Requires cargo pgrx init and Postgres 16"]
fn test_pgauthz_extension_e2e() {
    // Would use testcontainers + pgx to load extension and run check.
    // For Phase 0, this is documented in BUILD.md.
}
