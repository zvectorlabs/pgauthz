use std::env;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use postgres::{Client, NoTls};
use uuid::Uuid;

#[derive(Clone)]
struct TupleRow {
    object_type: &'static str,
    object_id: &'static str,
    relation: &'static str,
    subject_type: &'static str,
    subject_id: &'static str,
}

fn connect() -> Client {
    let dsn = env::var("PGAUTHZ_BENCH_DSN")
        .unwrap_or_else(|_| "host=127.0.0.1 port=28816 user=postgres dbname=postgres".to_string());

    let mut client = Client::connect(&dsn, NoTls)
        .unwrap_or_else(|e| panic!("failed to connect using PGAUTHZ_BENCH_DSN='{}': {}", dsn, e));

    client
        .batch_execute("CREATE EXTENSION IF NOT EXISTS pgauthz;")
        .unwrap_or_else(|e| panic!("failed to ensure pgauthz extension exists: {}", e));

    client
}

fn setup_case(client: &mut Client, model_dsl: &str, tuples: &[TupleRow]) -> (Uuid, Uuid) {
    let tenant_id = Uuid::new_v4();
    let schema_id = Uuid::new_v4();
    let model_id = Uuid::new_v4().to_string();

    client
        .execute(
            "INSERT INTO authz.tenant (id, name, status) VALUES ($1::uuid, $2, 'active')",
            &[
                &tenant_id,
                &format!("bench-{}", &tenant_id.to_string()[..8]),
            ],
        )
        .expect("insert tenant failed");

    client
        .execute(
            "INSERT INTO authz.authz_schema (id, tenant_id, name, status) VALUES ($1::uuid, $2::uuid, $3, 'active')",
            &[&schema_id, &tenant_id, &format!("bench-schema-{}", &schema_id.to_string()[..8])],
        )
        .expect("insert schema failed");

    client
        .execute(
            "INSERT INTO authz.authorization_policy (id, tenant_id, schema_id, definition) VALUES ($1, $2::uuid, $3::uuid, $4)",
            &[&model_id, &tenant_id, &schema_id, &model_dsl],
        )
        .expect("insert model failed");

    for tuple in tuples {
        client
            .execute(
                "INSERT INTO authz.tuple (tenant_id, schema_id, object_type, object_id, relation, subject_type, subject_id, condition)
                 VALUES ($1::uuid, $2::uuid, $3, $4, $5, $6, $7, NULL)",
                &[
                    &tenant_id,
                    &schema_id,
                    &tuple.object_type,
                    &tuple.object_id,
                    &tuple.relation,
                    &tuple.subject_type,
                    &tuple.subject_id,
                ],
            )
            .expect("insert tuple failed");
    }

    (tenant_id, schema_id)
}

fn cleanup_case(client: &mut Client, tenant_id: &Uuid, schema_id: &Uuid) {
    client
        .execute(
            "DELETE FROM authz.tuple WHERE tenant_id = $1::uuid AND schema_id = $2::uuid",
            &[&tenant_id, &schema_id],
        )
        .expect("cleanup tuples failed");

    client
        .execute(
            "DELETE FROM authz.authorization_policy WHERE tenant_id = $1::uuid AND schema_id = $2::uuid",
            &[&tenant_id, &schema_id],
        )
        .expect("cleanup models failed");

    client
        .execute(
            "DELETE FROM authz.authz_schema WHERE tenant_id = $1::uuid AND id = $2::uuid",
            &[&tenant_id, &schema_id],
        )
        .expect("cleanup schema failed");

    client
        .execute(
            "DELETE FROM authz.tenant WHERE id = $1::uuid",
            &[&tenant_id],
        )
        .expect("cleanup tenant failed");
}

fn bench_check_shallow_pg(c: &mut Criterion) {
    let dsl = "type document { relations define viewer: [user] }";
    let tuples = [TupleRow {
        object_type: "document",
        object_id: "doc1",
        relation: "viewer",
        subject_type: "user",
        subject_id: "alice",
    }];

    let mut client = connect();
    let (tenant_id, schema_id) = setup_case(&mut client, dsl, &tuples);
    let tenant_id_s = tenant_id.to_string();
    let schema_id_s = schema_id.to_string();

    c.bench_function("check_shallow_pg", |b| {
        b.iter(|| {
            let allowed: bool = client
                .query_one(
                    "SELECT pgauthz_check($1, $2, $3, $4, $5, $6, $7)",
                    &[
                        &tenant_id_s,
                        &schema_id_s,
                        &"document",
                        &"doc1",
                        &"viewer",
                        &"user",
                        &"alice",
                    ],
                )
                .expect("pgauthz_check failed")
                .get(0);
            black_box(allowed)
        })
    });

    cleanup_case(&mut client, &tenant_id, &schema_id);
}

fn bench_check_deep_pg(c: &mut Criterion) {
    let dsl = "type folder { relations define viewer: [user] define parent: [folder] } type document { relations define parent: [folder] define viewer: viewer from parent }";
    let tuples = [
        TupleRow {
            object_type: "document",
            object_id: "doc1",
            relation: "parent",
            subject_type: "folder",
            subject_id: "b",
        },
        TupleRow {
            object_type: "folder",
            object_id: "b",
            relation: "parent",
            subject_type: "folder",
            subject_id: "a",
        },
        TupleRow {
            object_type: "folder",
            object_id: "a",
            relation: "parent",
            subject_type: "folder",
            subject_id: "root",
        },
        TupleRow {
            object_type: "folder",
            object_id: "root",
            relation: "viewer",
            subject_type: "user",
            subject_id: "alice",
        },
    ];

    let mut client = connect();
    let (tenant_id, schema_id) = setup_case(&mut client, dsl, &tuples);
    let tenant_id_s = tenant_id.to_string();
    let schema_id_s = schema_id.to_string();

    c.bench_function("check_deep_ttu_pg", |b| {
        b.iter(|| {
            let allowed: bool = client
                .query_one(
                    "SELECT pgauthz_check($1, $2, $3, $4, $5, $6, $7)",
                    &[
                        &tenant_id_s,
                        &schema_id_s,
                        &"document",
                        &"doc1",
                        &"viewer",
                        &"user",
                        &"alice",
                    ],
                )
                .expect("pgauthz_check failed")
                .get(0);
            black_box(allowed)
        })
    });

    cleanup_case(&mut client, &tenant_id, &schema_id);
}

fn bench_check_union_fanout_pg(c: &mut Criterion) {
    let dsl = "type document { relations define r1: [user] define r2: [user] define r3: [user] define r4: [user] define r5: [user] define r6: [user] define r7: [user] define r8: [user] define r9: [user] define r10: [user] define viewer: r1 or r2 or r3 or r4 or r5 or r6 or r7 or r8 or r9 or r10 }";
    let tuples = [TupleRow {
        object_type: "document",
        object_id: "doc1",
        relation: "r10",
        subject_type: "user",
        subject_id: "alice",
    }];

    let mut client = connect();
    let (tenant_id, schema_id) = setup_case(&mut client, dsl, &tuples);
    let tenant_id_s = tenant_id.to_string();
    let schema_id_s = schema_id.to_string();

    c.bench_function("check_union_fanout_pg", |b| {
        b.iter(|| {
            let allowed: bool = client
                .query_one(
                    "SELECT pgauthz_check($1, $2, $3, $4, $5, $6, $7)",
                    &[
                        &tenant_id_s,
                        &schema_id_s,
                        &"document",
                        &"doc1",
                        &"viewer",
                        &"user",
                        &"alice",
                    ],
                )
                .expect("pgauthz_check failed")
                .get(0);
            black_box(allowed)
        })
    });

    cleanup_case(&mut client, &tenant_id, &schema_id);
}

fn bench_check_union_fanout_batch_pg(c: &mut Criterion) {
    let dsl = "type document { relations define r1: [user] define r2: [user] define r3: [user] define r4: [user] define r5: [user] define r6: [user] define r7: [user] define r8: [user] define r9: [user] define r10: [user] define viewer: r1 or r2 or r3 or r4 or r5 or r6 or r7 or r8 or r9 or r10 }";
    let tuples = [TupleRow {
        object_type: "document",
        object_id: "doc1",
        relation: "r10",
        subject_type: "user",
        subject_id: "alice",
    }];

    let mut client = connect();
    let (tenant_id, schema_id) = setup_case(&mut client, dsl, &tuples);
    let tenant_id_s = tenant_id.to_string();
    let schema_id_s = schema_id.to_string();

    client
        .execute("SET authz.check_strategy = 'batch'", &[])
        .expect("Failed to set strategy");

    c.bench_function("check_union_fanout_batch_pg", |b| {
        b.iter(|| {
            let allowed: bool = client
                .query_one(
                    "SELECT pgauthz_check($1, $2, $3, $4, $5, $6, $7)",
                    &[
                        &tenant_id_s,
                        &schema_id_s,
                        &"document",
                        &"doc1",
                        &"viewer",
                        &"user",
                        &"alice",
                    ],
                )
                .expect("pgauthz_check failed")
                .get(0);
            black_box(allowed)
        })
    });

    cleanup_case(&mut client, &tenant_id, &schema_id);
}

fn bench_check_union_fanout_parallel_pg(c: &mut Criterion) {
    let dsl = "type document { relations define r1: [user] define r2: [user] define r3: [user] define r4: [user] define r5: [user] define r6: [user] define r7: [user] define r8: [user] define r9: [user] define r10: [user] define viewer: r1 or r2 or r3 or r4 or r5 or r6 or r7 or r8 or r9 or r10 }";
    let tuples = [TupleRow {
        object_type: "document",
        object_id: "doc1",
        relation: "r10",
        subject_type: "user",
        subject_id: "alice",
    }];

    let mut client = connect();
    let (tenant_id, schema_id) = setup_case(&mut client, dsl, &tuples);
    let tenant_id_s = tenant_id.to_string();
    let schema_id_s = schema_id.to_string();

    client
        .execute("SET authz.check_strategy = 'parallel'", &[])
        .expect("Failed to set strategy");

    c.bench_function("check_union_fanout_parallel_pg", |b| {
        b.iter(|| {
            let allowed: bool = client
                .query_one(
                    "SELECT pgauthz_check($1, $2, $3, $4, $5, $6, $7)",
                    &[
                        &tenant_id_s,
                        &schema_id_s,
                        &"document",
                        &"doc1",
                        &"viewer",
                        &"user",
                        &"alice",
                    ],
                )
                .expect("pgauthz_check failed")
                .get(0);
            black_box(allowed)
        })
    });

    cleanup_case(&mut client, &tenant_id, &schema_id);
}

// Real-world scenario: Document collaboration with organizational access
fn bench_check_document_collaboration_pg(c: &mut Criterion) {
    let dsl = "type user {} type organization { relations define member: [user] relations define admin: [user] } type project { relations define owner: [user] relations define collaborator: [user] relations define viewer: [user, organization#member] or collaborator or owner } type document { relations define parent: [project] relations define editor: collaborator or owner relations define viewer: viewer from parent }";

    let tuples = [
        // Alice owns the project
        TupleRow {
            object_type: "project",
            object_id: "proj1",
            relation: "owner",
            subject_type: "user",
            subject_id: "alice",
        },
        // Bob is a collaborator on the project
        TupleRow {
            object_type: "project",
            object_id: "proj1",
            relation: "collaborator",
            subject_type: "user",
            subject_id: "bob",
        },
        // Charlie is a member of TechOrg
        TupleRow {
            object_type: "organization",
            object_id: "techorg",
            relation: "member",
            subject_type: "user",
            subject_id: "charlie",
        },
        // TechOrg has viewer access to the project
        TupleRow {
            object_type: "project",
            object_id: "proj1",
            relation: "viewer",
            subject_type: "organization",
            subject_id: "techorg#member",
        },
        // Document belongs to project
        TupleRow {
            object_type: "document",
            object_id: "doc1",
            relation: "parent",
            subject_type: "project",
            subject_id: "proj1",
        },
    ];

    let mut client = connect();
    let (tenant_id, schema_id) = setup_case(&mut client, dsl, &tuples);
    let tenant_id_s = tenant_id.to_string();
    let schema_id_s = schema_id.to_string();

    // Test owner access (should be fastest - direct ownership)
    c.bench_function("document_collaboration_owner", |b| {
        b.iter(|| {
            let allowed: bool = client
                .query_one(
                    "SELECT pgauthz_check($1, $2, $3, $4, $5, $6, $7)",
                    &[
                        &tenant_id_s,
                        &schema_id_s,
                        &"document",
                        &"doc1",
                        &"editor",
                        &"user",
                        &"alice",
                    ],
                )
                .expect("pgauthz_check failed")
                .get(0);
            black_box(allowed)
        })
    });

    // Test collaborator access (direct relationship)
    c.bench_function("document_collaboration_collaborator", |b| {
        b.iter(|| {
            let allowed: bool = client
                .query_one(
                    "SELECT pgauthz_check($1, $2, $3, $4, $5, $6, $7)",
                    &[
                        &tenant_id_s,
                        &schema_id_s,
                        &"document",
                        &"doc1",
                        &"editor",
                        &"user",
                        &"bob",
                    ],
                )
                .expect("pgauthz_check failed")
                .get(0);
            black_box(allowed)
        })
    });

    // Test organizational access (complex nested relationship)
    c.bench_function("document_collaboration_org_member", |b| {
        b.iter(|| {
            let allowed: bool = client
                .query_one(
                    "SELECT pgauthz_check($1, $2, $3, $4, $5, $6, $7)",
                    &[
                        &tenant_id_s,
                        &schema_id_s,
                        &"document",
                        &"doc1",
                        &"viewer",
                        &"user",
                        &"charlie",
                    ],
                )
                .expect("pgauthz_check failed")
                .get(0);
            black_box(allowed)
        })
    });

    cleanup_case(&mut client, &tenant_id, &schema_id);
}

// Real-world scenario: Role-based access control with hierarchical roles
fn bench_check_role_hierarchy_pg(c: &mut Criterion) {
    let dsl = "type user {} type role { relations define member: [user] relations define inherits: [role] } type resource { relations define admin: [role#member] relations define editor: [role#member] relations define viewer: [role#member] }";

    let tuples = [
        // Role hierarchy
        TupleRow {
            object_type: "role",
            object_id: "admin",
            relation: "inherits",
            subject_type: "role",
            subject_id: "editor",
        },
        TupleRow {
            object_type: "role",
            object_id: "editor",
            relation: "inherits",
            subject_type: "role",
            subject_id: "viewer",
        },
        // User role assignments
        TupleRow {
            object_type: "role",
            object_id: "viewer",
            relation: "member",
            subject_type: "user",
            subject_id: "alice",
        },
        TupleRow {
            object_type: "role",
            object_id: "editor",
            relation: "member",
            subject_type: "user",
            subject_id: "bob",
        },
        TupleRow {
            object_type: "role",
            object_id: "admin",
            relation: "member",
            subject_type: "user",
            subject_id: "charlie",
        },
        // Resource permissions
        TupleRow {
            object_type: "resource",
            object_id: "res1",
            relation: "viewer",
            subject_type: "role",
            subject_id: "viewer#member",
        },
        TupleRow {
            object_type: "resource",
            object_id: "res1",
            relation: "editor",
            subject_type: "role",
            subject_id: "editor#member",
        },
        TupleRow {
            object_type: "resource",
            object_id: "res1",
            relation: "admin",
            subject_type: "role",
            subject_id: "admin#member",
        },
    ];

    let mut client = connect();
    let (tenant_id, schema_id) = setup_case(&mut client, dsl, &tuples);
    let tenant_id_s = tenant_id.to_string();
    let schema_id_s = schema_id.to_string();

    // Test direct role assignment (baseline)
    c.bench_function("role_hierarchy_direct_viewer", |b| {
        b.iter(|| {
            let allowed: bool = client
                .query_one(
                    "SELECT pgauthz_check($1, $2, $3, $4, $5, $6, $7)",
                    &[
                        &tenant_id_s,
                        &schema_id_s,
                        &"resource",
                        &"res1",
                        &"viewer",
                        &"user",
                        &"alice",
                    ],
                )
                .expect("pgauthz_check failed")
                .get(0);
            black_box(allowed)
        })
    });

    // Test role inheritance (2 levels deep)
    c.bench_function("role_hierarchy_inheritance_editor", |b| {
        b.iter(|| {
            let allowed: bool = client
                .query_one(
                    "SELECT pgauthz_check($1, $2, $3, $4, $5, $6, $7)",
                    &[
                        &tenant_id_s,
                        &schema_id_s,
                        &"resource",
                        &"res1",
                        &"viewer",
                        &"user",
                        &"bob", // Bob is editor, inherits viewer rights
                    ],
                )
                .expect("pgauthz_check failed")
                .get(0);
            black_box(allowed)
        })
    });

    // Test deep role inheritance (3 levels)
    c.bench_function("role_hierarchy_deep_inheritance", |b| {
        b.iter(|| {
            let allowed: bool = client
                .query_one(
                    "SELECT pgauthz_check($1, $2, $3, $4, $5, $6, $7)",
                    &[
                        &tenant_id_s,
                        &schema_id_s,
                        &"resource",
                        &"res1",
                        &"viewer",
                        &"user",
                        &"charlie", // Charlie is admin, inherits editor and viewer rights
                    ],
                )
                .expect("pgauthz_check failed")
                .get(0);
            black_box(allowed)
        })
    });

    cleanup_case(&mut client, &tenant_id, &schema_id);
}

// Real-world scenario: Multi-tenant SaaS with customer data isolation
fn bench_check_multi_tenant_pg(c: &mut Criterion) {
    let dsl = "type user {} type customer { relations define admin: [user] relations define member: [user] } type workspace { relations define owner: [customer#admin] relations define member: [customer#member] } type project { relations define parent: [workspace] relations define owner: [workspace#owner] relations define member: [workspace#member] } type document { relations define parent: [project] relations define owner: [project#owner] relations define viewer: [project#member] or owner }";

    let tuples = [
        // Customer setup
        TupleRow {
            object_type: "customer",
            object_id: "cust1",
            relation: "admin",
            subject_type: "user",
            subject_id: "alice",
        },
        TupleRow {
            object_type: "customer",
            object_id: "cust1",
            relation: "member",
            subject_type: "user",
            subject_id: "bob",
        },
        // Workspace ownership
        TupleRow {
            object_type: "workspace",
            object_id: "ws1",
            relation: "owner",
            subject_type: "customer",
            subject_id: "cust1#admin",
        },
        TupleRow {
            object_type: "workspace",
            object_id: "ws1",
            relation: "member",
            subject_type: "customer",
            subject_id: "cust1#member",
        },
        // Project in workspace
        TupleRow {
            object_type: "project",
            object_id: "proj1",
            relation: "parent",
            subject_type: "workspace",
            subject_id: "ws1",
        },
        TupleRow {
            object_type: "project",
            object_id: "proj1",
            relation: "owner",
            subject_type: "workspace",
            subject_id: "ws1#owner",
        },
        TupleRow {
            object_type: "project",
            object_id: "proj1",
            relation: "member",
            subject_type: "workspace",
            subject_id: "ws1#member",
        },
        // Document in project
        TupleRow {
            object_type: "document",
            object_id: "doc1",
            relation: "parent",
            subject_type: "project",
            subject_id: "proj1",
        },
        TupleRow {
            object_type: "document",
            object_id: "doc1",
            relation: "owner",
            subject_type: "project",
            subject_id: "proj1#owner",
        },
    ];

    let mut client = connect();
    let (tenant_id, schema_id) = setup_case(&mut client, dsl, &tuples);
    let tenant_id_s = tenant_id.to_string();
    let schema_id_s = schema_id.to_string();

    // Test customer admin access (deep chain: customer -> workspace -> project -> document)
    c.bench_function("multi_tenant_admin_access", |b| {
        b.iter(|| {
            let allowed: bool = client
                .query_one(
                    "SELECT pgauthz_check($1, $2, $3, $4, $5, $6, $7)",
                    &[
                        &tenant_id_s,
                        &schema_id_s,
                        &"document",
                        &"doc1",
                        &"viewer",
                        &"user",
                        &"alice",
                    ],
                )
                .expect("pgauthz_check failed")
                .get(0);
            black_box(allowed)
        })
    });

    // Test customer member access
    c.bench_function("multi_tenant_member_access", |b| {
        b.iter(|| {
            let allowed: bool = client
                .query_one(
                    "SELECT pgauthz_check($1, $2, $3, $4, $5, $6, $7)",
                    &[
                        &tenant_id_s,
                        &schema_id_s,
                        &"document",
                        &"doc1",
                        &"viewer",
                        &"user",
                        &"bob",
                    ],
                )
                .expect("pgauthz_check failed")
                .get(0);
            black_box(allowed)
        })
    });

    cleanup_case(&mut client, &tenant_id, &schema_id);
}

// Real-world scenario: GitHub-like repository permissions
fn bench_check_github_repo_pg(c: &mut Criterion) {
    let dsl = "type user {} type organization { relations define member: [user] relations define owner: [user] } type team { relations define member: [user] relations define parent: [organization] } type repository { relations define owner: [user, organization#owner] relations define admin: [team#member] relations define write: [team#member] relations define read: [team#member] or write or admin or owner }";

    let tuples = [
        // Organization setup
        TupleRow {
            object_type: "organization",
            object_id: "techcorp",
            relation: "owner",
            subject_type: "user",
            subject_id: "alice",
        },
        TupleRow {
            object_type: "organization",
            object_id: "techcorp",
            relation: "member",
            subject_type: "user",
            subject_id: "bob",
        },
        // Teams
        TupleRow {
            object_type: "team",
            object_id: "dev-team",
            relation: "member",
            subject_type: "user",
            subject_id: "bob",
        },
        TupleRow {
            object_type: "team",
            object_id: "dev-team",
            relation: "parent",
            subject_type: "organization",
            subject_id: "techcorp",
        },
        TupleRow {
            object_type: "team",
            object_id: "ops-team",
            relation: "member",
            subject_type: "user",
            subject_id: "charlie",
        },
        TupleRow {
            object_type: "team",
            object_id: "ops-team",
            relation: "parent",
            subject_type: "organization",
            subject_id: "techcorp",
        },
        // Repository permissions
        TupleRow {
            object_type: "repository",
            object_id: "webapp",
            relation: "owner",
            subject_type: "organization",
            subject_id: "techcorp#owner",
        },
        TupleRow {
            object_type: "repository",
            object_id: "webapp",
            relation: "admin",
            subject_type: "team",
            subject_id: "dev-team#member",
        },
        TupleRow {
            object_type: "repository",
            object_id: "webapp",
            relation: "write",
            subject_type: "team",
            subject_id: "ops-team#member",
        },
    ];

    let mut client = connect();
    let (tenant_id, schema_id) = setup_case(&mut client, dsl, &tuples);
    let tenant_id_s = tenant_id.to_string();
    let schema_id_s = schema_id.to_string();

    // Test organization owner access
    c.bench_function("github_repo_org_owner", |b| {
        b.iter(|| {
            let allowed: bool = client
                .query_one(
                    "SELECT pgauthz_check($1, $2, $3, $4, $5, $6, $7)",
                    &[
                        &tenant_id_s,
                        &schema_id_s,
                        &"repository",
                        &"webapp",
                        &"read",
                        &"user",
                        &"alice",
                    ],
                )
                .expect("pgauthz_check failed")
                .get(0);
            black_box(allowed)
        })
    });

    // Test team admin access
    c.bench_function("github_repo_team_admin", |b| {
        b.iter(|| {
            let allowed: bool = client
                .query_one(
                    "SELECT pgauthz_check($1, $2, $3, $4, $5, $6, $7)",
                    &[
                        &tenant_id_s,
                        &schema_id_s,
                        &"repository",
                        &"webapp",
                        &"read",
                        &"user",
                        &"bob",
                    ],
                )
                .expect("pgauthz_check failed")
                .get(0);
            black_box(allowed)
        })
    });

    // Test team write access (union of permissions)
    c.bench_function("github_repo_team_write", |b| {
        b.iter(|| {
            let allowed: bool = client
                .query_one(
                    "SELECT pgauthz_check($1, $2, $3, $4, $5, $6, $7)",
                    &[
                        &tenant_id_s,
                        &schema_id_s,
                        &"repository",
                        &"webapp",
                        &"read",
                        &"user",
                        &"charlie",
                    ],
                )
                .expect("pgauthz_check failed")
                .get(0);
            black_box(allowed)
        })
    });

    cleanup_case(&mut client, &tenant_id, &schema_id);
}

criterion_group!(
    check_benches_pg,
    bench_check_shallow_pg,
    bench_check_deep_pg,
    bench_check_union_fanout_pg,
    bench_check_union_fanout_batch_pg,
    bench_check_union_fanout_parallel_pg,
    bench_check_document_collaboration_pg,
    bench_check_role_hierarchy_pg,
    bench_check_multi_tenant_pg,
    bench_check_github_repo_pg
);
criterion_main!(check_benches_pg);
