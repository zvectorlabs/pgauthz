# pgauthz

[Zanzibar](https://research.google/pubs/zanzibar-googles-consistent-global-authorization-system/)-style authorization as a PostgreSQL extension.

## Overview

pgauthz brings Google [Zanzibar](https://research.google/pubs/zanzibar-googles-consistent-global-authorization-system/)-style authorization directly into PostgreSQL, enabling fine-grained access control with relationship-based permissions. Built as a native PostgreSQL extension, it provides high-performance authorization checks with built-in caching, observability, and structured error handling.

## Key Features

- **[Zanzibar](https://research.google/pubs/zanzibar-googles-consistent-global-authorization-system/)-Style Authorization** - Relationship-based access control with computed permissions
- **PostgreSQL Native** - Runs directly in your database as an extension
- **High Performance** - Multi-level caching (L1 model, L2 result, L3 tuple)
- **Observability** - OpenTelemetry metrics and tracing support
- **Structured Errors** - PostgreSQL SQLSTATE error codes for all operations
- **Flexible Policies** - Support for conditions, wildcards, and complex permission hierarchies

## Quick Links

- **[Installation Guide](docs/installation.md)** - Get started with pgauthz
- **[Quick Start](docs/quickstart.md)** - 5-minute tutorial
- **[API Reference](docs/api-reference.md)** - Complete SQL function documentation
- **[Configuration](docs/configuration.md)** - GUC parameters and tuning
- **[Observability](docs/observability.md)** - Metrics and tracing setup
- **[Debugging](docs/debugging.md)** - Troubleshooting guide

## Documentation

Full documentation is available at **[docs.pgauthz.dev](https://docs.pgauthz.dev)**

## Quick Example

```sql
-- Create the extension
CREATE EXTENSION pgauthz;

-- Define an authorization policy
SELECT pgauthz_define_policy('
  type user {}
  type document {
    relations
      define viewer: [user]
      define editor: [user]
  }
');

-- Add a relation
SELECT pgauthz_add_relation('document', 'doc1', 'viewer', 'user', 'alice');

-- Check permission
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');
-- Returns: true
```

## Examples

### Nested Groups with Exclusions

Control access through group hierarchies with deny-list style exclusions — e.g. interns in a group can be blocked from accessing documents while regular members retain access.

```sql
SELECT pgauthz_define_policy('
  type user {}
  type group {
    relations
      define member: [user | group#member]
      define intern: [user]
      define allowed: [user]
    permissions
      define intern_but_not_allowed = intern - allowed
      define non_intern_member = member - intern_but_not_allowed
  }
  type document {
    relations
      define viewer: [user | group#non_intern_member]
    permissions
      define view = viewer
  }
');

-- alice is a regular member
SELECT pgauthz_add_relation('group', 'engineering', 'member', 'user', 'alice');
-- bob is an intern (not in allowed list)
SELECT pgauthz_add_relation('group', 'engineering', 'intern', 'user', 'bob');
-- grant the group access to a document
SELECT pgauthz_add_relation('document', 'doc1', 'viewer', 'group', 'engineering#non_intern_member');

SELECT pgauthz_check('document', 'doc1', 'view', 'user', 'alice'); -- true
SELECT pgauthz_check('document', 'doc1', 'view', 'user', 'bob');   -- false (intern, excluded)
```

### Conditional Access (Business Hours & IP Allowlist)

Attach runtime conditions to relations — permissions only resolve when context satisfies the condition expression.

```sql
SELECT pgauthz_define_policy('
  type user {}
  condition business_hours(hour: int, day: string) {
    hour >= 9 && hour <= 17 && day != "saturday" && day != "sunday"
  }
  condition ip_whitelist(allowed_ips: list<string>, current_ip: string) {
    current_ip in allowed_ips
  }
  type document {
    relations
      define viewer: [user with business_hours]
      define editor: [user with ip_whitelist]
    permissions
      define view = viewer + editor
  }
');

SELECT pgauthz_add_relation('document', 'doc1', 'viewer', 'user', 'alice', 'business_hours');
SELECT pgauthz_add_relation('document', 'doc1', 'editor', 'user', 'bob',   'ip_whitelist');

-- alice can view during business hours
SELECT pgauthz_check_with_context('document', 'doc1', 'view', 'user', 'alice',
  '{"hour": 10, "day": "monday"}');   -- true

-- alice cannot view outside business hours
SELECT pgauthz_check_with_context('document', 'doc1', 'view', 'user', 'alice',
  '{"hour": 22, "day": "monday"}');   -- false

-- bob can view from a whitelisted IP
SELECT pgauthz_check_with_context('document', 'doc1', 'view', 'user', 'bob',
  '{"allowed_ips": ["10.0.0.1"], "current_ip": "10.0.0.1"}');  -- true
```

### Public Access with Wildcards

Use `user:*` to grant access to all users — useful for public resources while keeping ownership restricted.

```sql
SELECT pgauthz_define_policy('
  type user {}
  type document {
    relations
      define viewer: [user | user:*]
      define owner:  [user]
    permissions
      define view   = viewer
      define delete = owner
  }
');

-- make doc1 publicly readable
SELECT pgauthz_add_relation('document', 'doc1', 'viewer', 'user', '*');
-- only alice can delete
SELECT pgauthz_add_relation('document', 'doc1', 'owner', 'user', 'alice');

SELECT pgauthz_check('document', 'doc1', 'view',   'user', 'anyone'); -- true
SELECT pgauthz_check('document', 'doc1', 'delete', 'user', 'anyone'); -- false
SELECT pgauthz_check('document', 'doc1', 'delete', 'user', 'alice');  -- true
```

### Permission Set Operations (Union, Intersection, Exclusion)

Compose permissions using `+` (union), `&` (intersection), and `-` (exclusion).

```sql
SELECT pgauthz_define_policy('
  type user {}
  type project {
    relations
      define member:   [user]
      define reviewer: [user]
      define lead:     [user]
      define blocked:  [user]
    permissions
      define can_review = reviewer + lead          -- union: reviewer OR lead
      define can_edit   = member & reviewer        -- intersection: must be both
      define can_merge  = lead - blocked           -- exclusion: lead but not blocked
  }
');

SELECT pgauthz_add_relation('project', 'proj1', 'member',   'user', 'alice');
SELECT pgauthz_add_relation('project', 'proj1', 'reviewer', 'user', 'alice');
SELECT pgauthz_add_relation('project', 'proj1', 'lead',     'user', 'bob');
SELECT pgauthz_add_relation('project', 'proj1', 'lead',     'user', 'charlie');
SELECT pgauthz_add_relation('project', 'proj1', 'blocked',  'user', 'charlie');

SELECT pgauthz_check('project', 'proj1', 'can_edit',   'user', 'alice');   -- true (member & reviewer)
SELECT pgauthz_check('project', 'proj1', 'can_edit',   'user', 'bob');     -- false (lead but not member)
SELECT pgauthz_check('project', 'proj1', 'can_merge',  'user', 'bob');     -- true (lead, not blocked)
SELECT pgauthz_check('project', 'proj1', 'can_merge',  'user', 'charlie'); -- false (lead but blocked)
```

### List All Objects a User Can Access

```sql
-- list all documents alice can view
SELECT * FROM pgauthz_list_objects('user', 'alice', 'view', 'document');

-- list all users who can edit doc1
SELECT * FROM pgauthz_list_subjects('document', 'doc1', 'edit', 'user');
```

## Core SQL Functions

- **Authorization Checks**: `pgauthz_check()`, `pgauthz_check_with_context()`, `pgauthz_expand()`
- **List Operations**: `pgauthz_list_objects()`, `pgauthz_list_subjects()`
- **Relation Management**: `pgauthz_add_relation()`, `pgauthz_write_relationships()`, `pgauthz_read_relationships()`
- **Policy Management**: `pgauthz_define_policy()`, `pgauthz_read_latest_policy()`, `pgauthz_list_policies()`
- **Change Tracking**: `pgauthz_read_changes()`

## Requirements

- PostgreSQL 16/17/18
- Linux or macOS

## License

This project is licensed under the Apache License 2.0. See [LICENSE](LICENSE) for details.

## Contributing

[Contributing guidelines to be added]

## Support

- Documentation: [docs](docs/)
- Issues: [GitHub Issues](https://github.com/zvectorlabs/pgauthz/issues)
