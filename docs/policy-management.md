# Policy Management Guide

This guide covers how to define, evolve, and manage authorization policies in pgauthz.

## Overview

pgauthz uses a **complete policy replacement** model, similar to OpenFGA and SpiceDB. Each call to `pgauthz_define_policy()` creates a new versioned policy that becomes the active "latest" policy. Previous versions are retained for history and rollback purposes.

## Core Concepts

### Policy Versioning

- **ULID-based IDs**: Each policy gets a unique ULID (Universally Unique Lexicographically Sortable Identifier)
- **Immutable versions**: Once created, a policy version cannot be modified
- **Latest wins**: The most recently created policy is always the active one
- **History retained**: All previous versions remain in the database

### Policy Structure

A policy defines:
- **Types**: The object types in your system (e.g., `user`, `document`, `folder`)
- **Relations**: How subjects relate to objects (e.g., `viewer`, `editor`, `owner`)
- **Permissions**: Computed access based on relations (e.g., `can_edit = editor | owner`)
- **Conditions**: Runtime checks with CEL expressions (e.g., `business_hours`, `ip_whitelist`)

## Defining Policies

### Basic Policy

```sql
SELECT pgauthz_define_policy('
  type user {}
  type document { 
    relations 
      define viewer: [user] 
      define editor: [user]
  }
');
```

### Policy with Permissions

```sql
SELECT pgauthz_define_policy('
  type user {}
  type document { 
    relations 
      define owner: [user]
      define editor: [user]
      define viewer: [user]
    permissions
      define can_edit: owner | editor
      define can_view: can_edit | viewer
  }
');
```

### Policy with Conditions

```sql
SELECT pgauthz_define_policy('
  type user {}
  condition business_hours { hour >= 9 && hour <= 17 }
  condition ip_whitelist(allowed_ips: list<string>, current_ip: string) { 
    current_ip in allowed_ips 
  }
  type document { 
    relations 
      define viewer: [user with business_hours]
      define editor: [user with ip_whitelist]
  }
');
```

### Policy with Inheritance

```sql
SELECT pgauthz_define_policy('
  type user {}
  type organization {
    relations
      define admin: [user]
      define member: [user]
  }
  type folder {
    relations
      define org: [organization]
      define owner: [user] | org->admin
      define viewer: [user] | org->member
  }
  type document { 
    relations 
      define parent: [folder]
      define owner: [user] | parent->owner
      define viewer: [user] | parent->viewer
  }
');
```

## Evolving Policies

As your business needs change, you'll need to update your policy. The key principle is: **redefine the entire policy with your changes**.

### Adding New Relations

```sql
-- Original policy
SELECT pgauthz_define_policy('
  type user {}
  type document { 
    relations define viewer: [user] 
  }
');

-- Evolved policy: add editor relation
SELECT pgauthz_define_policy('
  type user {}
  type document { 
    relations 
      define viewer: [user]
      define editor: [user]   -- NEW
  }
');
```

### Adding New Types

```sql
-- Evolved policy: add folder type
SELECT pgauthz_define_policy('
  type user {}
  type folder {                -- NEW
    relations
      define owner: [user]
  }
  type document { 
    relations 
      define parent: [folder]  -- NEW
      define viewer: [user]
      define editor: [user]
  }
');
```

### Adding Conditions

```sql
-- Evolved policy: add time-based access
SELECT pgauthz_define_policy('
  type user {}
  condition office_hours { hour >= 9 && hour <= 18 }  -- NEW
  type document { 
    relations 
      define viewer: [user]
      define restricted_viewer: [user with office_hours]  -- NEW
      define editor: [user]
  }
');
```

## Managing Policy Versions

### List All Policies

```sql
SELECT * FROM pgauthz_list_policies();
-- Returns: id, definition for each policy version
```

### Read Specific Policy

```sql
SELECT * FROM pgauthz_read_policy('01ABC123DEF456...');
```

### Read Latest Policy

```sql
-- Raw definition
SELECT * FROM pgauthz_read_latest_policy();

-- Computed/parsed structure
SELECT * FROM pgauthz_read_latest_policy_computed();
```

### Computed Policy View

The computed view shows the parsed structure:

```sql
SELECT * FROM pgauthz_read_latest_policy_computed();
-- Returns:
--   policy_id: ULID of the policy
--   type_name: Name of the type (e.g., 'document')
--   relation_name: Name of the relation (e.g., 'viewer')
--   relation_type: 'relation', 'permission', or 'condition'
--   expression_json: JSON representation of the relation expression
--   condition_name: Name of condition (if applicable)
--   condition_params_json: JSON of condition parameters
--   condition_expression: CEL expression for condition
```

## Relationship Compatibility

When you evolve a policy:

### Compatible Changes (relationships persist)
- Adding new types
- Adding new relations to existing types
- Adding new permissions
- Adding new conditions

### Breaking Changes (may require migration)
- Removing types that have relationships
- Removing relations that have relationships
- Renaming types or relations
- Changing relation type constraints

### Migration Example

```sql
-- If you rename 'viewer' to 'reader', migrate relationships:
UPDATE authz.relationship_tuple 
SET relation = 'reader' 
WHERE relation = 'viewer';
```

## Best Practices

### 1. Version Control Policies

Store your policy definitions in your codebase:

```
policies/
├── v1_initial.sql
├── v2_add_folders.sql
├── v3_add_conditions.sql
└── current.sql  -- symlink to latest
```

### 2. Use Migrations

Apply policy changes through your migration system:

```sql
-- migrations/003_add_editor_role.sql
SELECT pgauthz_define_policy('
  type user {}
  type document { 
    relations 
      define viewer: [user]
      define editor: [user]
  }
');
```

### 3. Test Before Deploying

Validate policies in staging:

```sql
-- Test that policy parses correctly
SELECT pgauthz_define_policy('...');

-- Verify expected structure
SELECT * FROM pgauthz_read_latest_policy_computed();

-- Test authorization checks
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');
```

### 4. Use Conditions for Runtime Flexibility

Conditions allow runtime flexibility without policy changes:

```sql
-- Instead of hardcoding IP ranges in policy
condition ip_check(allowed: list<string>, current: string) { 
  current in allowed 
}

-- Pass different IPs at check time
SELECT pgauthz_check_with_context(
  'document', 'doc1', 'editor', 'user', 'alice',
  '{"allowed": ["10.0.0.1"], "current": "10.0.0.1"}'::jsonb
);
```

### 5. Monitor Policy Changes

Track policy evolution:

```sql
-- See all policy versions with timestamps
SELECT id, 
       substring(definition, 1, 100) as preview,
       created_at
FROM authz.authorization_policy 
ORDER BY created_at DESC;
```

## Rollback

To rollback to a previous policy version, you need to redefine the policy with the old definition:

```sql
-- Get the old policy definition
SELECT definition FROM pgauthz_read_policy('01OLDPOLICYID...');

-- Redefine with the old definition (creates a new version)
SELECT pgauthz_define_policy('<old definition here>');
```

## Troubleshooting

### Policy Parse Errors

```sql
-- Check for syntax errors
SELECT pgauthz_define_policy('invalid policy');
-- ERROR: Policy parse error (SQLSTATE 22000)
```

### Policy Validation Errors

```sql
-- Check for semantic errors (undefined types, cycles, etc.)
SELECT pgauthz_define_policy('
  type document { relations define viewer: [undefined_type] }
');
-- ERROR: Policy validation error (SQLSTATE 23514)
```

### Relationship Validation Errors

```sql
-- Adding relationship for undefined type/relation
SELECT pgauthz_add_relation('photo', 'p1', 'viewer', 'user', 'alice');
-- ERROR: Tuple validation error (SQLSTATE 23514)
```

## See Also

- [API Reference](api-reference.md) - Complete SQL function reference
- [Quickstart](quickstart.md) - Getting started guide
- [Examples](examples/) - Real-world policy examples
