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

## Core SQL Functions

- **Authorization Checks**: `pgauthz_check()`, `pgauthz_check_with_context()`, `pgauthz_expand()`
- **List Operations**: `pgauthz_list_objects()`, `pgauthz_list_subjects()`
- **Relation Management**: `pgauthz_add_relation()`, `pgauthz_read_tuples()`
- **Policy Management**: `pgauthz_define_policy()`, `pgauthz_read_model()`, `pgauthz_list_models()`
- **Change Tracking**: `pgauthz_read_changes()`

## Requirements

- PostgreSQL 16+
- Linux or macOS

## License

This project is licensed under the Apache License 2.0. See [LICENSE](LICENSE) for details.

## Contributing

[Contributing guidelines to be added]

## Support

- Documentation: [docs](docs/)
- Issues: [GitHub Issues](https://github.com/your-org/pgauthz/issues)
- Discussions: [GitHub Discussions](https://github.com/your-org/pgauthz/discussions)
