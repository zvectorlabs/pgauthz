# API Reference

Complete reference for all pgauthz SQL functions.

## Authorization Checks

### pgauthz_check()

Check if a subject has a specific permission on an object.

**Signature:**
```sql
pgauthz_check(
    object_type text,
    object_id text,
    relation text,
    subject_type text,
    subject_id text
) RETURNS boolean
```

**Parameters:**
- `object_type` - Type of the object (e.g., 'document', 'folder')
- `object_id` - ID of the specific object
- `relation` - Relation to check (e.g., 'viewer', 'editor')
- `subject_type` - Type of the subject (e.g., 'user', 'group')
- `subject_id` - ID of the specific subject

**Returns:** `true` if the subject has the relation, `false` otherwise

**Example:**
```sql
-- Check if alice can view doc1
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');
```

**Error Codes:**
- `22023` - Invalid parameter (empty string, invalid format)
- `02000` - Policy not found
- `54000` - Max recursion depth exceeded

---

### pgauthz_check_with_context()

Check permission with additional context for evaluating conditions.

**Signature:**
```sql
pgauthz_check_with_context(
    object_type text,
    object_id text,
    relation text,
    subject_type text,
    subject_id text,
    context jsonb
) RETURNS boolean
```

**Parameters:**
- Same as `pgauthz_check()`, plus:
- `context` - JSONB object with context variables for condition evaluation

**Returns:** `true` if the subject has the relation given the context, `false` otherwise

**Example:**
```sql
-- Check with IP whitelist condition
SELECT pgauthz_check_with_context(
    'document',
    'doc1',
    'editor',
    'user',
    'alice',
    '{"allowed_ips": ["10.0.0.1"], "current_ip": "10.0.0.1"}'::jsonb
);

-- Check with time-based condition
SELECT pgauthz_check_with_context(
    'document',
    'doc1',
    'viewer',
    'user',
    'bob',
    '{"hour": 14}'::jsonb
);
```

---

### pgauthz_expand()

Debug function that shows the permission tree for a relation.

**Signature:**
```sql
pgauthz_expand(
    object_type text,
    object_id text,
    relation text
) RETURNS text
```

**Parameters:**
- `object_type` - Type of the object
- `object_id` - ID of the specific object
- `relation` - Relation to expand

**Returns:** Text representation of the permission tree

**Example:**
```sql
SELECT pgauthz_expand('document', 'doc1', 'viewer');
```

**Output:**
```
union(
  direct(user:alice),
  direct(user:bob),
  computed(editor) -> union(
    direct(user:charlie)
  )
)
```

---

## List Operations

### pgauthz_list_objects()

Find all objects of a given type that a subject has a specific relation to.

**Signature:**
```sql
pgauthz_list_objects(
    subject_type text,
    subject_id text,
    relation text,
    object_type text,
    page_size integer DEFAULT 100,
    continuation_token text DEFAULT NULL
) RETURNS SETOF text
```

**Parameters:**
- `subject_type` - Type of the subject
- `subject_id` - ID of the subject
- `relation` - Relation to check
- `object_type` - Type of objects to list
- `page_size` - Maximum number of results (1-1000)
- `continuation_token` - Token for pagination

**Returns:** Set of object IDs

**Example:**
```sql
-- List all documents alice can view
SELECT * FROM pgauthz_list_objects('user', 'alice', 'viewer', 'document');

-- With pagination
SELECT * FROM pgauthz_list_objects('user', 'alice', 'viewer', 'document', 50, NULL);
```

---

### pgauthz_list_subjects()

Find all subjects of a given type that have a specific relation to an object.

**Signature:**
```sql
pgauthz_list_subjects(
    object_type text,
    object_id text,
    relation text,
    subject_type text,
    page_size integer DEFAULT 100,
    continuation_token text DEFAULT NULL
) RETURNS SETOF text
```

**Parameters:**
- `object_type` - Type of the object
- `object_id` - ID of the object
- `relation` - Relation to check
- `subject_type` - Type of subjects to list
- `page_size` - Maximum number of results (1-1000)
- `continuation_token` - Token for pagination

**Returns:** Set of subject IDs

**Example:**
```sql
-- List all users who can view doc1
SELECT * FROM pgauthz_list_subjects('document', 'doc1', 'viewer', 'user');

-- With pagination
SELECT * FROM pgauthz_list_subjects('document', 'doc1', 'viewer', 'user', 50, NULL);
```

---

## Relation Management

### pgauthz_add_relation()

Add a single relation between an object and a subject.

**Signature:**
```sql
pgauthz_add_relation(
    object_type text,
    object_id text,
    relation text,
    subject_type text,
    subject_id text,
    condition text DEFAULT NULL
) RETURNS text
```

**Parameters:**
- `object_type` - Type of the object
- `object_id` - ID of the object
- `relation` - Relation name
- `subject_type` - Type of the subject
- `subject_id` - ID of the subject
- `condition` - Optional condition name

**Returns:** Revision ID of the write operation

**Example:**
```sql
-- Add a simple relation
SELECT pgauthz_add_relation('document', 'doc1', 'viewer', 'user', 'alice');

-- Add a relation with a condition
SELECT pgauthz_add_relation(
    'document',
    'doc1',
    'editor',
    'user',
    'bob',
    'ip_whitelist'
);
```

**Error Codes:**
- `22023` - Invalid parameter
- `23514` - Tuple validation failed (invalid object type, relation, etc.)

---

### pgauthz_read_tuples()

Read/query existing relations with optional filtering.

**Signature:**
```sql
pgauthz_read_tuples(
    object_type text DEFAULT NULL,
    object_id text DEFAULT NULL,
    relation text DEFAULT NULL,
    subject_type text DEFAULT NULL,
    subject_id text DEFAULT NULL
) RETURNS TABLE (
    object_type text,
    object_id text,
    relation text,
    subject_type text,
    subject_id text,
    condition text
)
```

**Parameters:**
All parameters are optional filters:
- `object_type` - Filter by object type
- `object_id` - Filter by object ID
- `relation` - Filter by relation
- `subject_type` - Filter by subject type
- `subject_id` - Filter by subject ID

**Returns:** Table of matching tuples

**Example:**
```sql
-- Get all relations for doc1
SELECT * FROM pgauthz_read_tuples('document', 'doc1', NULL, NULL, NULL);

-- Get all relations for user alice
SELECT * FROM pgauthz_read_tuples(NULL, NULL, NULL, 'user', 'alice');

-- Get all viewer relations
SELECT * FROM pgauthz_read_tuples(NULL, NULL, 'viewer', NULL, NULL);

-- Get all relations (no filters)
SELECT * FROM pgauthz_read_tuples(NULL, NULL, NULL, NULL, NULL);
```

---

## Policy Management

### pgauthz_define_policy()

Define or update an authorization policy.

**Signature:**
```sql
pgauthz_define_policy(
    definition text
) RETURNS text
```

**Parameters:**
- `definition` - Policy definition in pgauthz schema language

**Returns:** Model ID of the created policy

**Example:**
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

**Policy Language Syntax:**

```
type <type_name> {
  relations
    define <relation_name>: <relation_expr>
}

condition <condition_name>(<params>) {
  <condition_expr>
}
```

**Relation Expressions:**
- Direct assignment: `[user]`, `[user | group#member]`
- Union: `viewer | editor`
- Intersection: `viewer & editor`
- Exclusion: `viewer - blocked`
- Computed userset: `parent->viewer`
- Tuple to userset: `owner from parent`
- With condition: `[user with condition_name]`

**Error Codes:**
- `22000` - Policy parse error (syntax error)
- `23514` - Policy validation error (undefined types, cycles, etc.)

---

### pgauthz_read_model()

Read a specific authorization policy by ID.

**Signature:**
```sql
pgauthz_read_model(
    model_id text
) RETURNS TABLE (
    id text,
    definition text
)
```

**Parameters:**
- `model_id` - ID of the policy to read

**Returns:** Table with policy ID and definition

**Example:**
```sql
SELECT * FROM pgauthz_read_model('01HQZX...');
```

**Error Codes:**
- `02000` - Model not found

---

### pgauthz_read_latest_model()

Read the most recently defined authorization policy.

**Signature:**
```sql
pgauthz_read_latest_model() RETURNS TABLE (
    id text,
    definition text
)
```

**Returns:** Table with the latest policy ID and definition

**Example:**
```sql
SELECT * FROM pgauthz_read_latest_model();
```

**Error Codes:**
- `02000` - No models found

---

### pgauthz_list_models()

List all authorization policies with pagination.

**Signature:**
```sql
pgauthz_list_models(
    page_size integer DEFAULT 100,
    continuation_token text DEFAULT NULL
) RETURNS TABLE (
    id text,
    definition text
)
```

**Parameters:**
- `page_size` - Maximum number of results (1-1000)
- `continuation_token` - Token for pagination

**Returns:** Table of policy IDs and definitions

**Example:**
```sql
-- List first 10 policies
SELECT * FROM pgauthz_list_models(10, NULL);

-- Get next page
SELECT * FROM pgauthz_list_models(10, 'cursor_token_here');
```

---

## Change Tracking

### pgauthz_read_changes()

Read changelog entries for watching permission changes (Watch API).

**Signature:**
```sql
pgauthz_read_changes(
    object_type text,
    after_ulid text DEFAULT NULL,
    page_size integer DEFAULT 100
) RETURNS TABLE (
    object_type text,
    object_id text,
    relation text,
    subject_type text,
    subject_id text,
    operation text,
    ulid text
)
```

**Parameters:**
- `object_type` - Type of objects to watch
- `after_ulid` - ULID cursor for pagination (get changes after this point)
- `page_size` - Maximum number of results (1-1000)

**Returns:** Table of change entries

**Example:**
```sql
-- Get recent changes for documents
SELECT * FROM pgauthz_read_changes('document', NULL, 100);

-- Get changes after a specific point
SELECT * FROM pgauthz_read_changes('document', '01HQZX...', 100);

-- Watch for new changes (polling pattern)
WITH latest AS (
  SELECT MAX(ulid) as cursor
  FROM pgauthz_read_changes('document', NULL, 1)
)
SELECT * FROM pgauthz_read_changes('document', (SELECT cursor FROM latest), 100);
```

**Operation Types:**
- `WRITE` - Relation was added
- `DELETE` - Relation was removed

---

## Error Handling

All pgauthz functions use PostgreSQL SQLSTATE error codes:

| SQLSTATE | Error Type | Description |
|----------|------------|-------------|
| `22023` | Invalid Parameter | Empty or invalid input parameters |
| `22000` | Data Exception | Policy parsing errors |
| `23514` | Check Violation | Policy or tuple validation failures |
| `02000` | No Data Found | Policy or model not found |
| `42704` | Undefined Object | Relation not found in policy |
| `54000` | Program Limit | Max recursion depth exceeded |
| `38000` | External Routine | Datastore operation errors |
| `XX000` | Internal Error | Unexpected internal errors |

**Example Error Handling:**

```sql
DO $$
BEGIN
    PERFORM pgauthz_check('document', '', 'viewer', 'user', 'alice');
EXCEPTION
    WHEN SQLSTATE '22023' THEN
        RAISE NOTICE 'Invalid parameter: %', SQLERRM;
    WHEN SQLSTATE '02000' THEN
        RAISE NOTICE 'Policy not found: %', SQLERRM;
END $$;
```

---

## Performance Considerations

### Caching

All check operations are cached at multiple levels:
- **L1 Cache**: Parsed policy models (TTL configurable)
- **L2 Cache**: Permission check results (TTL configurable)
- **L3 Cache**: Tuple query results (TTL configurable)

Configure caching via GUC parameters (see [Configuration Guide](configuration.md)).

### Batch Operations

For bulk operations, consider batching:

```sql
-- Instead of multiple individual checks
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');
SELECT pgauthz_check('document', 'doc2', 'viewer', 'user', 'alice');

-- Use list_objects for better performance
SELECT * FROM pgauthz_list_objects('user', 'alice', 'viewer', 'document');
```

### Pagination

Always use pagination for list operations:

```sql
-- Good: paginated
SELECT * FROM pgauthz_list_objects('user', 'alice', 'viewer', 'document', 100, NULL);

-- Avoid: no pagination (may return too many results)
SELECT * FROM pgauthz_list_objects('user', 'alice', 'viewer', 'document', 10000, NULL);
```

---

## See Also

- [Quick Start Guide](quickstart.md) - Learn by example
- [Configuration Guide](configuration.md) - Tuning and optimization
- [Performance Guide](performance.md) - Best practices for production
- [Debugging Guide](debugging.md) - Troubleshooting tips
