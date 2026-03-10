# pgauthz Documentation

Welcome to the pgauthz documentation. pgauthz is a PostgreSQL extension that implements Google Zanzibar-style authorization, bringing fine-grained, relationship-based access control directly into your database.

## What is pgauthz?

pgauthz enables you to define and enforce complex authorization policies using a relationship-based model. Instead of managing permissions through traditional role-based access control (RBAC), pgauthz allows you to express permissions as relationships between objects and subjects.

### Key Concepts

- **Objects** - Resources you want to protect (documents, files, folders, etc.)
- **Subjects** - Entities that can access objects (users, groups, services)
- **Relations** - Connections between objects and subjects (viewer, editor, owner)
- **Policies** - Rules that define how relations grant permissions

## Why pgauthz?

### PostgreSQL Native
Runs directly in your database as an extension - no external services required. Authorization checks happen in-process with microsecond latency.

### High Performance
Multi-level caching system:
- **L1 Cache** - Parsed policy models
- **L2 Cache** - Permission check results
- **L3 Cache** - Tuple query results

### Production Ready
- Structured error handling with PostgreSQL SQLSTATE codes
- OpenTelemetry metrics and tracing
- Comprehensive test coverage
- Battle-tested Zanzibar algorithm

### Flexible & Powerful
- Conditional permissions based on context (time, IP, custom attributes)
- Wildcard support for scalable permission management
- Complex permission hierarchies with unions, intersections, and exclusions
- Watch API for real-time permission changes

## Quick Example

```sql
-- Create extension
CREATE EXTENSION pgauthz;

-- Define a policy
SELECT pgauthz_define_policy('
  type user {}
  type document {
    relations
      define viewer: [user]
      define editor: [user]
      define owner: [user]
  }
');

-- Grant permissions
SELECT pgauthz_add_relation('document', 'doc1', 'owner', 'user', 'alice');
SELECT pgauthz_add_relation('document', 'doc1', 'viewer', 'user', 'bob');

-- Check permissions
SELECT pgauthz_check('document', 'doc1', 'owner', 'user', 'alice');
-- Returns: true

SELECT pgauthz_check('document', 'doc1', 'owner', 'user', 'bob');
-- Returns: false (bob is only a viewer)

-- List all documents alice can view
SELECT * FROM pgauthz_list_objects('user', 'alice', 'viewer', 'document');
```

## Getting Started

1. **[Installation](installation.md)** - Install pgauthz from binary packages
2. **[Quick Start](quickstart.md)** - 5-minute tutorial to get up and running
3. **[API Reference](api-reference.md)** - Complete SQL function documentation

## Core Documentation

### Setup & Configuration
- **[Installation](installation.md)** - Installing and verifying pgauthz
- **[Configuration](configuration.md)** - GUC parameters and tuning options

### Usage
- **[Quick Start](quickstart.md)** - Step-by-step tutorial
- **[API Reference](api-reference.md)** - All SQL functions with examples
- **[Examples](examples/basic-authorization.md)** - Real-world use cases

### Operations
- **[Observability](observability.md)** - Metrics, tracing, and monitoring
- **[Debugging](debugging.md)** - Troubleshooting and error handling
- **[Performance](performance.md)** - Optimization and tuning guide

## Use Cases

### Document Management
Control access to documents, folders, and files with hierarchical permissions.

### Multi-User Applications
Implement fine-grained permissions for SaaS applications with complex sharing models.

### API Authorization
Protect API endpoints with relationship-based access control.

### Resource Sharing
Enable flexible sharing patterns with viewer, editor, and owner roles.

## Architecture

pgauthz implements the Google Zanzibar authorization model:

1. **Policy Definition** - Define object types, relations, and permission rules
2. **Relation Storage** - Store relationships between objects and subjects
3. **Permission Resolution** - Compute permissions by traversing the relationship graph
4. **Caching** - Cache policies, results, and tuples for high performance

## Community & Support

- **Documentation**: [docs.pgauthz.dev](https://docs.pgauthz.dev)
- **GitHub**: [github.com/your-org/pgauthz](https://github.com/your-org/pgauthz)
- **Issues**: Report bugs and request features
- **Discussions**: Ask questions and share ideas

## Next Steps

- Follow the **[Quick Start](quickstart.md)** guide
- Explore the **[API Reference](api-reference.md)**
- Check out **[Examples](examples/basic-authorization.md)**
