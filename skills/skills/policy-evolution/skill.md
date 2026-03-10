# Policy Evolution Skill

Learn how to evolve authorization policies as your business needs change.

## Overview

pgauthz uses a **complete policy replacement** model. Each `pgauthz_define_policy()` call creates a new versioned policy. This skill teaches you how to safely evolve policies while maintaining existing relationships.

## Prerequisites

- pgauthz installed and running
- Basic SQL knowledge
- Understanding of authorization concepts (types, relations, permissions)

## Step 1: Understand the Policy Model

Each policy is:
- **Complete**: Contains all types, relations, permissions, and conditions
- **Immutable**: Once created, cannot be modified
- **Versioned**: Gets a unique ULID identifier
- **Replaceable**: New policy becomes "latest", old versions retained

```sql
-- Check current policy
SELECT * FROM pgauthz_read_latest_policy();

-- See all policy versions
SELECT * FROM pgauthz_list_policies();
```

## Step 2: Start with a Simple Policy

```sql
-- Initial policy: basic document access
SELECT pgauthz_define_policy('
  type user {}
  type document { 
    relations 
      define viewer: [user] 
  }
');

-- Add some relationships
SELECT pgauthz_add_relation('document', 'doc1', 'viewer', 'user', 'alice');
SELECT pgauthz_add_relation('document', 'doc2', 'viewer', 'user', 'bob');

-- Verify
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');
-- Returns: true
```

## Step 3: Evolve the Policy

When business needs change, redefine the **entire** policy:

```sql
-- Business need: add editor role
SELECT pgauthz_define_policy('
  type user {}
  type document { 
    relations 
      define viewer: [user]
      define editor: [user]    -- NEW
  }
');

-- Existing relationships still work!
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');
-- Returns: true

-- Now we can add editors
SELECT pgauthz_add_relation('document', 'doc1', 'editor', 'user', 'charlie');
```

## Step 4: Add New Types

```sql
-- Business need: organize documents in folders
SELECT pgauthz_define_policy('
  type user {}
  type folder {                          -- NEW TYPE
    relations
      define owner: [user]
      define viewer: [user]
  }
  type document { 
    relations 
      define parent: [folder]            -- NEW: folder reference
      define viewer: [user] | parent->viewer  -- NEW: inherit from folder
      define editor: [user]
  }
');

-- Create folder structure
SELECT pgauthz_add_relation('folder', 'folder1', 'owner', 'user', 'alice');
SELECT pgauthz_add_relation('folder', 'folder1', 'viewer', 'user', 'bob');

-- Link document to folder
SELECT pgauthz_add_relation('document', 'doc3', 'parent', 'folder', 'folder1');

-- Bob can now view doc3 through folder inheritance!
SELECT pgauthz_check('document', 'doc3', 'viewer', 'user', 'bob');
-- Returns: true
```

## Step 5: Add Conditions

```sql
-- Business need: time-based access control
SELECT pgauthz_define_policy('
  type user {}
  condition business_hours { hour >= 9 && hour <= 17 }  -- NEW
  type folder {
    relations
      define owner: [user]
      define viewer: [user]
  }
  type document { 
    relations 
      define parent: [folder]
      define viewer: [user] | parent->viewer
      define editor: [user]
      define restricted_viewer: [user with business_hours]  -- NEW
  }
');

-- Add time-restricted access
SELECT pgauthz_add_relation('document', 'doc4', 'restricted_viewer', 'user', 'dave', 'business_hours');

-- Check with context
SELECT pgauthz_check_with_context(
  'document', 'doc4', 'restricted_viewer', 'user', 'dave',
  '{"hour": 10}'::jsonb
);
-- Returns: true (within business hours)

SELECT pgauthz_check_with_context(
  'document', 'doc4', 'restricted_viewer', 'user', 'dave',
  '{"hour": 22}'::jsonb
);
-- Returns: false (outside business hours)
```

## Step 6: Verify Policy Evolution

```sql
-- See all policy versions
SELECT id, substring(definition, 1, 50) as preview 
FROM pgauthz_list_policies();

-- Get computed view of current policy
SELECT type_name, relation_name, relation_type 
FROM pgauthz_read_latest_policy_computed()
WHERE type_name != ''
ORDER BY type_name, relation_name;
```

## Common Patterns

### Pattern 1: Adding Permissions

```sql
SELECT pgauthz_define_policy('
  type user {}
  type document { 
    relations 
      define owner: [user]
      define editor: [user]
      define viewer: [user]
    permissions
      define can_delete: owner
      define can_edit: owner | editor
      define can_view: can_edit | viewer
  }
');
```

### Pattern 2: Organization Hierarchy

```sql
SELECT pgauthz_define_policy('
  type user {}
  type organization {
    relations
      define admin: [user]
      define member: [user]
  }
  type team {
    relations
      define org: [organization]
      define lead: [user] | org->admin
      define member: [user] | org->member
  }
  type project {
    relations
      define team: [team]
      define owner: [user] | team->lead
      define contributor: [user] | team->member
  }
');
```

### Pattern 3: Parameterized Conditions

```sql
SELECT pgauthz_define_policy('
  type user {}
  condition ip_whitelist(allowed_ips: list<string>, current_ip: string) { 
    current_ip in allowed_ips 
  }
  condition max_amount(limit: int, amount: int) {
    amount <= limit
  }
  type transaction {
    relations
      define approver: [user with ip_whitelist]
      define limited_approver: [user with max_amount]
  }
');
```

## Best Practices

1. **Version control policies** - Store in your codebase
2. **Test in staging first** - Validate before production
3. **Use migrations** - Apply through your migration system
4. **Document changes** - Keep a changelog of policy updates
5. **Monitor after changes** - Watch for authorization failures

## Troubleshooting

### Relationship validation fails after policy change

If you removed a type or relation that has existing relationships:

```sql
-- Check what relationships exist
SELECT * FROM pgauthz_read_relationships('old_type', NULL, NULL, NULL, NULL);

-- Migrate or delete before policy change
DELETE FROM authz.relationship_tuple WHERE object_type = 'old_type';
```

### Policy parse error

Check syntax:
- Types need `{}` even if empty
- Relations need `define` keyword
- Conditions need valid CEL expressions

### Check returns unexpected result

Debug with expand:
```sql
SELECT pgauthz_expand('document', 'doc1', 'viewer');
```

## Next Steps

- [API Reference](../../docs/api-reference.md) - Complete function reference
- [Conditions Guide](../../docs/examples/conditions.md) - Advanced condition patterns
- [Performance Guide](../../docs/performance.md) - Optimization tips
