# Quick Start Guide

Get started with pgauthz in under 5 minutes. This guide walks you through installing the extension, defining your first authorization policy, and performing permission checks.

## Prerequisites

- PostgreSQL 16+ installed
- pgauthz extension installed (see [Installation Guide](installation.md))
- Access to a PostgreSQL database

## Step 1: Create the Extension

Connect to your PostgreSQL database and create the pgauthz extension:

```sql
CREATE EXTENSION pgauthz;
```

Verify the extension is loaded:

```sql
SELECT extname, extversion FROM pg_extension WHERE extname = 'pgauthz';
```

## Step 2: Define an Authorization Policy

Let's create a simple document management system with users who can view or edit documents.

```sql
SELECT pgauthz_define_policy('
  type user {}
  
  type document {
    relations
      define viewer: [user]
      define editor: [user]
      define owner: [user]
  }
');
```

This policy defines:
- A `user` type (representing people in your system)
- A `document` type with three relations:
  - `viewer` - users who can view the document
  - `editor` - users who can edit the document
  - `owner` - users who own the document

## Step 3: Add Relations

Now let's add some relationships between users and documents:

```sql
-- Alice owns doc1
SELECT pgauthz_add_relation('document', 'doc1', 'owner', 'user', 'alice');

-- Bob can edit doc1
SELECT pgauthz_add_relation('document', 'doc1', 'editor', 'user', 'bob');

-- Charlie can view doc1
SELECT pgauthz_add_relation('document', 'doc1', 'viewer', 'user', 'charlie');

-- Alice owns doc2
SELECT pgauthz_add_relation('document', 'doc2', 'owner', 'user', 'alice');
```

## Step 4: Check Permissions

Now we can check if users have specific permissions:

```sql
-- Check if Alice is an owner of doc1
SELECT pgauthz_check('document', 'doc1', 'owner', 'user', 'alice');
-- Returns: true

-- Check if Bob is an owner of doc1
SELECT pgauthz_check('document', 'doc1', 'owner', 'user', 'bob');
-- Returns: false (Bob is an editor, not an owner)

-- Check if Charlie is a viewer of doc1
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'charlie');
-- Returns: true
```

## Step 5: List Objects and Subjects

Find all documents a user can access:

```sql
-- List all documents Alice owns
SELECT * FROM pgauthz_list_objects('user', 'alice', 'owner', 'document');
-- Returns: doc1, doc2

-- List all documents Bob can edit
SELECT * FROM pgauthz_list_objects('user', 'bob', 'editor', 'document');
-- Returns: doc1
```

Find all users with access to a document:

```sql
-- List all users who can view doc1
SELECT * FROM pgauthz_list_subjects('document', 'doc1', 'viewer', 'user');
-- Returns: charlie
```

## Step 6: View Existing Relations

Query the relations you've created:

```sql
-- View all relations for doc1
SELECT * FROM pgauthz_read_relationships('document', 'doc1', NULL, NULL, NULL);

-- View all relations for a specific user
SELECT * FROM pgauthz_read_relationships(NULL, NULL, NULL, 'user', 'alice');
```

## Advanced Example: Hierarchical Permissions

Let's create a more complex policy with hierarchical permissions where owners can do everything editors can do, and editors can do everything viewers can do.

```sql
SELECT pgauthz_define_policy('
  type user {}
  
  type document {
    relations
      define viewer: [user]
      define editor: [user] | viewer
      define owner: [user] | editor
  }
');
```

Now add relations:

```sql
-- Alice is the owner
SELECT pgauthz_add_relation('document', 'doc3', 'owner', 'user', 'alice');

-- Check if Alice can edit (even though we only made her an owner)
SELECT pgauthz_check('document', 'doc3', 'editor', 'user', 'alice');
-- Returns: true (because owner includes editor)

-- Check if Alice can view
SELECT pgauthz_check('document', 'doc3', 'viewer', 'user', 'alice');
-- Returns: true (because owner includes editor includes viewer)
```

## Example: Conditional Permissions

Add context-based permissions using conditions:

```sql
SELECT pgauthz_define_policy('
  type user {}
  
  condition business_hours {
    hour >= 9 && hour <= 17
  }
  
  condition ip_whitelist(allowed_ips: list<string>, current_ip: string) {
    current_ip in allowed_ips
  }
  
  type document {
    relations
      define viewer: [user]
      define editor: [user with ip_whitelist]
      define restricted_viewer: [user with business_hours]
  }
');
```

Add a relation with a condition:

```sql
-- Bob can edit, but only from whitelisted IPs
SELECT pgauthz_add_relation('document', 'doc4', 'editor', 'user', 'bob', 'ip_whitelist');

-- Dave can view, but only during business hours
SELECT pgauthz_add_relation('document', 'doc4', 'restricted_viewer', 'user', 'dave', 'business_hours');
```

Check with context:

```sql
-- Check if Bob can edit from a whitelisted IP
SELECT pgauthz_check_with_context(
  'document', 
  'doc4', 
  'editor', 
  'user', 
  'bob',
  '{"allowed_ips": ["10.0.0.1", "10.0.0.2"], "current_ip": "10.0.0.1"}'::jsonb
);
-- Returns: true

-- Check if Bob can edit from a non-whitelisted IP
SELECT pgauthz_check_with_context(
  'document', 
  'doc4', 
  'editor', 
  'user', 
  'bob',
  '{"allowed_ips": ["10.0.0.1", "10.0.0.2"], "current_ip": "192.168.1.1"}'::jsonb
);
-- Returns: false

-- Check if Dave can view during business hours
SELECT pgauthz_check_with_context(
  'document', 
  'doc4', 
  'restricted_viewer', 
  'user', 
  'dave',
  '{"hour": 14}'::jsonb
);
-- Returns: true (14:00 is between 9 and 17)
```

## Debugging Permissions

Use `pgauthz_expand()` to see how permissions are computed:

```sql
SELECT pgauthz_expand('document', 'doc3', 'editor');
```

This shows the permission tree and helps debug complex permission hierarchies.

## Viewing Your Policy

View the current authorization policy:

```sql
-- Get the latest policy
SELECT * FROM pgauthz_read_latest_policy();

-- List all policies (with pagination)
SELECT * FROM pgauthz_list_policies(10, NULL);
```

## Next Steps

Now that you've learned the basics:

1. **[API Reference](api-reference.md)** - Explore all available functions
2. **[Configuration](configuration.md)** - Learn about caching and performance tuning
3. **[Examples](examples/basic-authorization.md)** - See real-world use cases
4. **[Observability](observability.md)** - Set up metrics and tracing
5. **[Performance](performance.md)** - Optimize for production workloads

## Common Patterns

### Pattern 1: Resource Hierarchies

```sql
type folder {
  relations
    define viewer: [user] | parent->viewer
    define parent: [folder]
}

type document {
  relations
    define viewer: [user] | parent->viewer
    define parent: [folder]
}
```

### Pattern 2: Group Membership

```sql
type group {
  relations
    define member: [user | group#member]
}

type document {
  relations
    define viewer: [user | group#member]
}
```

### Pattern 3: Exclusions

```sql
type document {
  relations
    define viewer: [user]
    define blocked: [user]
    define effective_viewer: viewer - blocked
}
```

## Troubleshooting

### Permission Check Returns Unexpected Result

Use `pgauthz_expand()` to debug:

```sql
SELECT pgauthz_expand('document', 'doc1', 'viewer');
```

### Policy Definition Fails

Check for syntax errors in your policy definition. Common issues:
- Missing brackets or braces
- Undefined types referenced in relations
- Invalid condition syntax

### Performance Issues

Enable caching in your configuration:

```sql
SET authz.model_cache_ttl_secs = 300;
SET authz.result_cache_ttl_secs = 60;
SET authz.tuple_cache_ttl_secs = 60;
```

See the [Performance Guide](performance.md) for more optimization tips.

## Getting Help

- **[Debugging Guide](debugging.md)** - Troubleshooting common issues
- **[GitHub Issues](https://github.com/zvectorlabs/pgauthz/issues)** - Report bugs
- **[GitHub Discussions](https://github.com/zvectorlabs/pgauthz/discussions)** - Ask questions
