use authz_core::core_resolver::CoreResolver;
use authz_core::dispatcher::{Dispatcher, LocalDispatcher};
use authz_core::error::AuthzError;
use authz_core::model_parser::parse_dsl;
use authz_core::policy_provider::StaticPolicyProvider;
use authz_core::resolver::{CheckResult, ResolveCheckRequest};
use authz_core::traits::{Tuple, TupleFilter, TupleReader};
use authz_core::type_system::TypeSystem;

// Mock datastore for E2E testing
#[derive(Clone)]
struct MockDatastore {
    tuples: Vec<Tuple>,
}

#[async_trait::async_trait]
impl TupleReader for MockDatastore {
    async fn read_tuples(&self, filter: &TupleFilter) -> Result<Vec<Tuple>, AuthzError> {
        let mut results = self.tuples.clone();

        if let Some(ref obj_type) = filter.object_type {
            results.retain(|t| &t.object_type == obj_type);
        }
        if let Some(ref obj_id) = filter.object_id {
            results.retain(|t| &t.object_id == obj_id);
        }
        if let Some(ref rel) = filter.relation {
            results.retain(|t| &t.relation == rel);
        }
        if let Some(ref subj_type) = filter.subject_type {
            results.retain(|t| &t.subject_type == subj_type);
        }
        if let Some(ref subj_id) = filter.subject_id {
            results.retain(|t| &t.subject_id == subj_id);
        }

        Ok(results)
    }

    async fn read_user_tuple(
        &self,
        object_type: &str,
        object_id: &str,
        relation: &str,
        subject_type: &str,
        subject_id: &str,
    ) -> Result<Option<Tuple>, AuthzError> {
        Ok(self
            .tuples
            .iter()
            .find(|t| {
                t.object_type == object_type
                    && t.object_id == object_id
                    && t.relation == relation
                    && t.subject_type == subject_type
                    && t.subject_id == subject_id
            })
            .cloned())
    }

    async fn read_userset_tuples(
        &self,
        object_type: &str,
        object_id: &str,
        relation: &str,
    ) -> Result<Vec<Tuple>, AuthzError> {
        Ok(self
            .tuples
            .iter()
            .filter(|t| {
                t.object_type == object_type && t.object_id == object_id && t.relation == relation
            })
            .cloned()
            .collect())
    }

    async fn read_starting_with_user(
        &self,
        subject_type: &str,
        subject_id: &str,
    ) -> Result<Vec<Tuple>, AuthzError> {
        Ok(self
            .tuples
            .iter()
            .filter(|t| t.subject_type == subject_type && t.subject_id == subject_id)
            .cloned()
            .collect())
    }

    async fn read_user_tuple_batch(
        &self,
        object_type: &str,
        object_id: &str,
        relations: &[String],
        subject_type: &str,
        subject_id: &str,
    ) -> Result<Option<Tuple>, AuthzError> {
        Ok(self
            .tuples
            .iter()
            .find(|t| {
                t.object_type == object_type
                    && t.object_id == object_id
                    && relations.contains(&t.relation)
                    && t.subject_type == subject_type
                    && t.subject_id == subject_id
            })
            .cloned())
    }
}

#[tokio::test]
async fn test_e2e_full_stack() {
    // Define a comprehensive authorization model
    let dsl = r#"
        type user {}
        type group {
            relations
                define member: [user]
        }
        type folder {
            relations
                define viewer: [user | group#member]
        }
        type document {
            relations
                define parent: [folder]
                define owner: [user]
                define editor: [user]
                define viewer: [user | group#member] + editor + owner
                define can_view: viewer
                define can_edit: editor + owner
                define can_delete: owner - editor
        }
    "#;

    let model = parse_dsl(dsl).expect("Failed to parse DSL");
    let ts = TypeSystem::new(model);

    // Setup test data
    let tuples = vec![
        // alice owns doc1
        Tuple {
            object_type: "document".to_string(),
            object_id: "doc1".to_string(),
            relation: "owner".to_string(),
            subject_type: "user".to_string(),
            subject_id: "alice".to_string(),
            condition: None,
        },
        // bob is editor of doc1
        Tuple {
            object_type: "document".to_string(),
            object_id: "doc1".to_string(),
            relation: "editor".to_string(),
            subject_type: "user".to_string(),
            subject_id: "bob".to_string(),
            condition: None,
        },
        // doc2 has parent folder1
        Tuple {
            object_type: "document".to_string(),
            object_id: "doc2".to_string(),
            relation: "parent".to_string(),
            subject_type: "folder".to_string(),
            subject_id: "folder1".to_string(),
            condition: None,
        },
        // charlie is viewer of folder1
        Tuple {
            object_type: "folder".to_string(),
            object_id: "folder1".to_string(),
            relation: "viewer".to_string(),
            subject_type: "user".to_string(),
            subject_id: "charlie".to_string(),
            condition: None,
        },
        // eng group has member dave
        Tuple {
            object_type: "group".to_string(),
            object_id: "eng".to_string(),
            relation: "member".to_string(),
            subject_type: "user".to_string(),
            subject_id: "dave".to_string(),
            condition: None,
        },
        // doc3 has viewer eng#member
        Tuple {
            object_type: "document".to_string(),
            object_id: "doc3".to_string(),
            relation: "viewer".to_string(),
            subject_type: "group".to_string(),
            subject_id: "eng".to_string(),
            condition: None,
        },
    ];

    let datastore = MockDatastore { tuples };
    let resolver = CoreResolver::new(datastore, StaticPolicyProvider::new(ts));
    let dispatcher = LocalDispatcher::new(resolver);

    // Test 1: Direct relation - alice owns doc1
    let result = dispatcher
        .dispatch_check(ResolveCheckRequest::new(
            "document".into(),
            "doc1".into(),
            "owner".into(),
            "user".into(),
            "alice".into(),
        ))
        .await
        .unwrap();
    assert_eq!(result, CheckResult::Allowed, "alice should own doc1");

    // Test 2: Computed userset - alice can_view doc1 (via owner)
    let result = dispatcher
        .dispatch_check(ResolveCheckRequest::new(
            "document".into(),
            "doc1".into(),
            "can_view".into(),
            "user".into(),
            "alice".into(),
        ))
        .await
        .unwrap();
    assert_eq!(
        result,
        CheckResult::Allowed,
        "alice should can_view doc1 via owner"
    );

    // Test 3: Union - bob can_view doc1 (via editor)
    let result = dispatcher
        .dispatch_check(ResolveCheckRequest::new(
            "document".into(),
            "doc1".into(),
            "can_view".into(),
            "user".into(),
            "bob".into(),
        ))
        .await
        .unwrap();
    assert_eq!(
        result,
        CheckResult::Allowed,
        "bob should can_view doc1 via editor"
    );

    // Test 4: Intersection (can_edit requires editor or owner)
    let result = dispatcher
        .dispatch_check(ResolveCheckRequest::new(
            "document".into(),
            "doc1".into(),
            "can_edit".into(),
            "user".into(),
            "bob".into(),
        ))
        .await
        .unwrap();
    assert_eq!(result, CheckResult::Allowed, "bob should can_edit doc1");

    // Test 5: Exclusion - alice can_delete (owner but not editor)
    let result = dispatcher
        .dispatch_check(ResolveCheckRequest::new(
            "document".into(),
            "doc1".into(),
            "can_delete".into(),
            "user".into(),
            "alice".into(),
        ))
        .await
        .unwrap();
    assert_eq!(result, CheckResult::Allowed, "alice should can_delete doc1");

    // Test 6: Exclusion denied - bob cannot delete (is editor)
    let result = dispatcher
        .dispatch_check(ResolveCheckRequest::new(
            "document".into(),
            "doc1".into(),
            "can_delete".into(),
            "user".into(),
            "bob".into(),
        ))
        .await
        .unwrap();
    assert_eq!(
        result,
        CheckResult::Denied,
        "bob should not can_delete doc1"
    );

    // Test 7: Userset expansion - dave can view doc3 via group membership
    let result = dispatcher
        .dispatch_check(ResolveCheckRequest::new(
            "document".into(),
            "doc3".into(),
            "viewer".into(),
            "user".into(),
            "dave".into(),
        ))
        .await
        .unwrap();
    assert_eq!(
        result,
        CheckResult::Allowed,
        "dave should view doc3 via eng group"
    );

    // Test 8: No permission
    let result = dispatcher
        .dispatch_check(ResolveCheckRequest::new(
            "document".into(),
            "doc1".into(),
            "owner".into(),
            "user".into(),
            "charlie".into(),
        ))
        .await
        .unwrap();
    assert_eq!(result, CheckResult::Denied, "charlie should not own doc1");
}
