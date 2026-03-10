use async_trait::async_trait;
use authz_core::core_resolver::CoreResolver;
use authz_core::error::AuthzError;
use authz_core::model_parser::parse_dsl;
use authz_core::policy_provider::StaticPolicyProvider;
use authz_core::resolver::{CheckResolver, ResolveCheckRequest};
use authz_core::traits::{Tuple, TupleFilter, TupleReader};
use authz_core::type_system::TypeSystem;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

#[derive(Clone)]
struct BenchDatastore {
    tuples: Vec<Tuple>,
}

#[async_trait]
impl TupleReader for BenchDatastore {
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

fn bench_check_shallow(c: &mut Criterion) {
    let dsl = "type document { relations define viewer: [user] }";
    let model = parse_dsl(dsl).unwrap();
    let ts = TypeSystem::new(model);

    let tuples = vec![Tuple {
        object_type: "document".to_string(),
        object_id: "doc1".to_string(),
        relation: "viewer".to_string(),
        subject_type: "user".to_string(),
        subject_id: "alice".to_string(),
        condition: None,
    }];

    let datastore = BenchDatastore { tuples };
    let resolver = CoreResolver::new(datastore, StaticPolicyProvider::new(ts));

    c.bench_function("check_shallow", |b| {
        b.iter(|| {
            let request = ResolveCheckRequest::new(
                "document".into(),
                "doc1".into(),
                "viewer".into(),
                "user".into(),
                "alice".into(),
            );
            pollster::block_on(resolver.resolve_check(black_box(request)))
        })
    });
}

fn bench_check_deep(c: &mut Criterion) {
    let dsl = "type folder { relations define viewer: [user] } type document { relations define parent: [folder] define viewer: viewer from parent }";
    let model = parse_dsl(dsl).unwrap();
    let ts = TypeSystem::new(model);

    let tuples = vec![
        Tuple {
            object_type: "document".to_string(),
            object_id: "doc1".to_string(),
            relation: "parent".to_string(),
            subject_type: "folder".to_string(),
            subject_id: "b".to_string(),
            condition: None,
        },
        Tuple {
            object_type: "folder".to_string(),
            object_id: "b".to_string(),
            relation: "parent".to_string(),
            subject_type: "folder".to_string(),
            subject_id: "a".to_string(),
            condition: None,
        },
        Tuple {
            object_type: "folder".to_string(),
            object_id: "a".to_string(),
            relation: "parent".to_string(),
            subject_type: "folder".to_string(),
            subject_id: "root".to_string(),
            condition: None,
        },
        Tuple {
            object_type: "folder".to_string(),
            object_id: "root".to_string(),
            relation: "viewer".to_string(),
            subject_type: "user".to_string(),
            subject_id: "alice".to_string(),
            condition: None,
        },
    ];

    let datastore = BenchDatastore { tuples };
    let resolver = CoreResolver::new(datastore, StaticPolicyProvider::new(ts));

    c.bench_function("check_deep_ttu", |b| {
        b.iter(|| {
            let request = ResolveCheckRequest::new(
                "document".into(),
                "doc1".into(),
                "viewer".into(),
                "user".into(),
                "alice".into(),
            );
            pollster::block_on(resolver.resolve_check(black_box(request)))
        })
    });
}

fn bench_check_union_fanout(c: &mut Criterion) {
    let dsl = "type document { relations define r1: [user] define r2: [user] define r3: [user] define r4: [user] define r5: [user] define r6: [user] define r7: [user] define r8: [user] define r9: [user] define r10: [user] define viewer: r1 or r2 or r3 or r4 or r5 or r6 or r7 or r8 or r9 or r10 }";
    let model = parse_dsl(dsl).unwrap();
    let ts = TypeSystem::new(model);

    let tuples = vec![Tuple {
        object_type: "document".to_string(),
        object_id: "doc1".to_string(),
        relation: "r10".to_string(),
        subject_type: "user".to_string(),
        subject_id: "alice".to_string(),
        condition: None,
    }];

    let datastore = BenchDatastore { tuples };
    let resolver = CoreResolver::new(datastore, StaticPolicyProvider::new(ts));

    c.bench_function("check_union_fanout", |b| {
        b.iter(|| {
            let request = ResolveCheckRequest::new(
                "document".into(),
                "doc1".into(),
                "viewer".into(),
                "user".into(),
                "alice".into(),
            );
            pollster::block_on(resolver.resolve_check(black_box(request)))
        })
    });
}

criterion_group!(
    check_benches,
    bench_check_shallow,
    bench_check_deep,
    bench_check_union_fanout
);
criterion_main!(check_benches);
