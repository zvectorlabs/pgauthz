// Validation unit tests for pgauthz functionality
// 
// Unit tests for:
// - Argument validation
// - Error handling
// - Input validation

use pgrx::prelude::*;
use crate::{create_test_model, create_test_tuple, DOCUMENT_MODEL, pgauthz_add_relation, pgauthz_write_tuples};

// ============================================================================
// Argument Validation Tests
// ============================================================================

#[pg_test]
fn test_check_and_expand_argument_validation() {
    create_test_model(DOCUMENT_MODEL);

    let bad_check = std::panic::catch_unwind(|| {
        let _ = Spi::run("SELECT pgauthz_check('', 'doc1', 'viewer', 'user', 'alice')").unwrap();
    });
    assert!(bad_check.is_err(), "empty object_type should fail validation");

    let bad_expand = std::panic::catch_unwind(|| {
        let _ = Spi::run("SELECT pgauthz_expand('document', '', 'viewer')").unwrap();
    });
    assert!(bad_expand.is_err(), "empty object_id should fail validation");
}

#[pg_test]
fn test_page_size_validation_on_list_apis() {
    create_test_model(DOCUMENT_MODEL);

    // Test just one invalid page size to avoid stack overflow
    let list_objects_q = "SELECT pgauthz_list_objects('user', 'alice', 'viewer', 'document', 0, NULL)";
    let objects_err = std::panic::catch_unwind(|| {
        let _ = Spi::run(list_objects_q).unwrap();
    });
    assert!(objects_err.is_err(), "invalid list_objects page_size should fail");

    let list_subjects_q = "SELECT pgauthz_list_subjects('document', 'doc1', 'viewer', 'user', 0, NULL)";
    let subjects_err = std::panic::catch_unwind(|| {
        let _ = Spi::run(list_subjects_q).unwrap();
    });
    assert!(subjects_err.is_err(), "invalid list_subjects page_size should fail");
}

#[pg_test]
fn test_continuation_token_length_validation() {
    create_test_model(DOCUMENT_MODEL);

    let long_token = "a".repeat(1500);
    let q = format!(
        "SELECT pgauthz_list_policies(10, {})",
        spi::quote_literal(&long_token)
    );

    let result = std::panic::catch_unwind(|| {
        let _ = Spi::run(&q).unwrap();
    });
    assert!(result.is_err(), "overly long continuation_token should fail validation");
}

// ============================================================================
// Tuple Validation Tests
// ============================================================================

#[pg_test]
fn test_tuple_invalid_shape_is_rejected() {
    create_test_model(DOCUMENT_MODEL);

    let invalid = std::panic::catch_unwind(|| {
        pgauthz_add_relation("photo", "1", "viewer", "user", "alice", None);
    });
    assert!(invalid.is_err(), "invalid object_type should be rejected");
}

#[pg_test]
fn test_system_recovers_after_failed_write() {
    create_test_model(DOCUMENT_MODEL);

    // First add_relation should succeed
    pgauthz_add_relation("document", "1", "viewer", "user", "alice", None);

    // add_relation with invalid object_type should fail
    let result = std::panic::catch_unwind(|| {
        pgauthz_add_relation("photo", "1", "viewer", "user", "alice", None);
    });
    assert!(result.is_err(), "add_relation with invalid tuple should fail");

    // System should still work for valid adds
    pgauthz_add_relation("document", "2", "viewer", "user", "bob", None);

    // Verify both good tuples are there
    let alice_q = "SELECT COUNT(*)::bigint FROM pgauthz_read_relationship('document', '1', 'viewer', 'user', 'alice')";
    let bob_q = "SELECT COUNT(*)::bigint FROM pgauthz_read_relationship('document', '2', 'viewer', 'user', 'bob')";

    let alice_count: i64 = Spi::get_one(alice_q).unwrap().unwrap();
    let bob_count: i64 = Spi::get_one(bob_q).unwrap().unwrap();
    assert_eq!(alice_count, 1);
    assert_eq!(bob_count, 1);
}
