# Debugging Guide

Comprehensive guide to troubleshooting and debugging pgauthz.

## Overview

This guide covers common issues, debugging techniques, and error resolution strategies for pgauthz.

## Quick Diagnostics

### Check Extension Status

```sql
-- Verify extension is installed
SELECT extname, extversion FROM pg_extension WHERE extname = 'pgauthz';

-- Test basic functionality
SELECT pgauthz_define_policy('type user {}');
```

### Enable Debug Logging

```sql
SET authz.tracing_level = 'debug';
```

### View Current Configuration

```sql
SELECT name, setting FROM pg_settings WHERE name LIKE 'authz.%';
```

## Common Issues

### Issue: Permission Check Returns Unexpected Result

**Symptom:** `pgauthz_check()` returns `false` when you expect `true` (or vice versa).

**Diagnosis:**

Use `pgauthz_expand()` to see how the permission is computed:

```sql
SELECT pgauthz_expand('document', 'doc1', 'viewer');
```

**Common Causes:**

1. **Missing Relation:**
```sql
-- Check if relation exists
SELECT * FROM pgauthz_read_relationships('document', 'doc1', 'viewer', 'user', 'alice');
```

2. **Wrong Relation Name:**
```sql
-- List all relations for the object
SELECT * FROM pgauthz_read_relationships('document', 'doc1', NULL, NULL, NULL);
```

3. **Policy Mismatch:**
```sql
-- View current policy
SELECT * FROM pgauthz_read_latest_policy();
```

4. **Condition Not Met:**
```sql
-- Check with context
SELECT pgauthz_check_with_context(
    'document', 'doc1', 'viewer', 'user', 'alice',
    '{"hour": 14}'::jsonb
);
```

**Solution:**

Add missing relation or fix policy definition:

```sql
-- Add the relation
SELECT pgauthz_add_relation('document', 'doc1', 'viewer', 'user', 'alice');

-- Or update the policy
SELECT pgauthz_define_policy('...');
```

---

### Issue: Policy Definition Fails

**Symptom:** `ERROR: policy parse error` or `ERROR: policy validation error`

**Error Codes:**
- `22000` - Syntax error in policy
- `23514` - Validation error (undefined types, cycles, etc.)

**Common Causes:**

1. **Syntax Error:**
```sql
-- Missing bracket
SELECT pgauthz_define_policy('
  type document {
    relations
      define viewer: [user
  }
');
-- ERROR: policy parse error at line 4
```

2. **Undefined Type:**
```sql
-- Referencing undefined type
SELECT pgauthz_define_policy('
  type document {
    relations
      define viewer: [group#member]
  }
');
-- ERROR: undefined type: group
```

3. **Circular Dependency:**
```sql
-- Cycle in relations
SELECT pgauthz_define_policy('
  type document {
    relations
      define viewer: editor
      define editor: viewer
  }
');
-- ERROR: cycle detected
```

**Solution:**

Fix the policy syntax:

```sql
-- Define all types
SELECT pgauthz_define_policy('
  type user {}
  type group {
    relations
      define member: [user]
  }
  type document {
    relations
      define viewer: [user | group#member]
  }
');
```

---

### Issue: Slow Performance

**Symptom:** Checks take longer than expected (>100ms).

**Diagnosis:**

1. **Enable timing:**
```sql
\timing on
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');
```

2. **Check cache hit rate:**
```sql
SET authz.tracing_level = 'debug';
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');
-- Look for "cache_hit" or "cache_miss" in output
```

3. **Check resolution depth:**
```sql
SELECT pgauthz_expand('document', 'doc1', 'viewer');
-- Count the depth of the tree
```

**Common Causes:**

1. **Caching Disabled:**
```sql
SHOW authz.model_cache_ttl_secs;  -- Should be > 0
SHOW authz.result_cache_ttl_secs;  -- Should be > 0
```

2. **Complex Policy:**
Deep permission hierarchies cause many database queries.

3. **Large Tuple Set:**
Many relations for a single object slow down resolution.

**Solution:**

Enable caching:

```sql
SET authz.model_cache_ttl_secs = 300;
SET authz.result_cache_ttl_secs = 60;
SET authz.tuple_cache_ttl_secs = 60;
```

Simplify policy if possible, or use batch operations:

```sql
-- Instead of multiple checks
SELECT * FROM pgauthz_list_objects('user', 'alice', 'viewer', 'document');
```

---

### Issue: High Memory Usage

**Symptom:** PostgreSQL memory usage grows over time.

**Diagnosis:**

Check cache capacity:

```sql
SHOW authz.cache_max_capacity;
```

**Solution:**

Reduce cache capacity:

```sql
SET authz.cache_max_capacity = 5000;
```

Or reduce cache TTLs to expire entries faster:

```sql
SET authz.model_cache_ttl_secs = 60;
SET authz.result_cache_ttl_secs = 30;
```

---

### Issue: Stale Permission Data

**Symptom:** Permission checks return outdated results after changes.

**Cause:** Cache TTL is too long.

**Solution:**

Reduce cache TTLs:

```sql
SET authz.result_cache_ttl_secs = 10;
SET authz.tuple_cache_ttl_secs = 10;
```

Or disable caching:

```sql
SET authz.result_cache_ttl_secs = 0;
SET authz.tuple_cache_ttl_secs = 0;
```

---

### Issue: Extension Not Found

**Symptom:** `ERROR: could not open extension control file`

**Diagnosis:**

Check extension files:

```bash
# Check control file
ls -l $(pg_config --sharedir)/extension/pgauthz.control

# Check shared library
ls -l $(pg_config --pkglibdir)/pgauthz.so
```

**Solution:**

Reinstall the extension package. See [Installation Guide](installation.md).

---

### Issue: Version Mismatch

**Symptom:** `ERROR: extension "pgauthz" has no update path`

**Diagnosis:**

Check available versions:

```sql
SELECT * FROM pg_available_extension_versions WHERE name = 'pgauthz';
```

**Solution:**

Update to compatible version or reinstall.

---

## Error Code Reference

### SQLSTATE Codes

| Code | Error Type | Description | Solution |
|------|------------|-------------|----------|
| `22023` | Invalid Parameter | Empty or invalid input | Check parameter values |
| `22000` | Data Exception | Policy parsing error | Fix policy syntax |
| `23514` | Check Violation | Validation failure | Fix policy or tuple data |
| `02000` | No Data Found | Policy not found | Define a policy first |
| `42704` | Undefined Object | Relation not in policy | Update policy definition |
| `54000` | Program Limit | Max depth exceeded | Simplify policy hierarchy |
| `38000` | External Routine | Datastore error | Check database connectivity |
| `XX000` | Internal Error | Unexpected error | Report bug |

### Handling Errors in Application Code

```sql
-- PL/pgSQL example
DO $$
BEGIN
    PERFORM pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');
EXCEPTION
    WHEN SQLSTATE '22023' THEN
        RAISE NOTICE 'Invalid parameter: %', SQLERRM;
    WHEN SQLSTATE '02000' THEN
        RAISE NOTICE 'Policy not found - define policy first';
    WHEN SQLSTATE '54000' THEN
        RAISE NOTICE 'Max depth exceeded - policy too complex';
    WHEN OTHERS THEN
        RAISE NOTICE 'Unexpected error: % (SQLSTATE: %)', SQLERRM, SQLSTATE;
END $$;
```

## Debugging Techniques

### 1. Use pgauthz_expand()

Visualize permission resolution:

```sql
SELECT pgauthz_expand('document', 'doc1', 'viewer');
```

Output shows the permission tree:
```
union(
  direct(user:alice),
  computed(editor) -> union(
    direct(user:bob)
  )
)
```

### 2. Query Relations Directly

Inspect stored relations:

```sql
-- All relations for an object
SELECT * FROM pgauthz_read_relationships('document', 'doc1', NULL, NULL, NULL);

-- All relations for a subject
SELECT * FROM pgauthz_read_relationships(NULL, NULL, NULL, 'user', 'alice');

-- Specific relation
SELECT * FROM pgauthz_read_relationships('document', 'doc1', 'viewer', NULL, NULL);
```

### 3. Enable Debug Logging

```sql
SET authz.tracing_level = 'debug';
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');
```

Look for log messages about:
- Cache hits/misses
- Database queries
- Resolution steps

### 4. Test with Simplified Policy

Create a minimal policy to isolate issues:

```sql
-- Minimal test policy
SELECT pgauthz_define_policy('
  type user {}
  type document {
    relations
      define viewer: [user]
  }
');

-- Add test relation
SELECT pgauthz_add_relation('document', 'test', 'viewer', 'user', 'alice');

-- Test check
SELECT pgauthz_check('document', 'test', 'viewer', 'user', 'alice');
-- Should return true
```

### 5. Check Policy Version

Ensure you're using the latest policy:

```sql
-- List all policies
SELECT id, LEFT(definition, 100) FROM pgauthz_list_policies(10, NULL);

-- Get latest
SELECT * FROM pgauthz_read_latest_policy();
```

### 6. Verify Context Variables

For conditional permissions:

```sql
-- Check without context (should fail if condition required)
SELECT pgauthz_check('document', 'doc1', 'editor', 'user', 'bob');

-- Check with context
SELECT pgauthz_check_with_context(
    'document', 'doc1', 'editor', 'user', 'bob',
    '{"allowed_ips": ["10.0.0.1"], "current_ip": "10.0.0.1"}'::jsonb
);
```

### 7. Monitor Performance

```sql
-- Enable timing
\timing on

-- Run check multiple times
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');

-- First call: cache miss (slower)
-- Subsequent calls: cache hit (faster)
```

## Advanced Debugging

### Trace SQL Queries

Enable PostgreSQL query logging:

```sql
SET log_statement = 'all';
SET log_min_duration_statement = 0;
```

Then check PostgreSQL logs for queries executed by pgauthz.

### Use EXPLAIN ANALYZE

```sql
EXPLAIN ANALYZE
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');
```

### Check for Locks

```sql
SELECT * FROM pg_locks WHERE relation::regclass::text LIKE '%authz%';
```

### Monitor Cache Statistics

With OpenTelemetry enabled:

```sql
SET authz.otel_enabled = true;
-- Check metrics for cache hit rates
```

## Troubleshooting Checklist

When debugging an issue:

- [ ] Check extension is installed and loaded
- [ ] Verify policy is defined
- [ ] Confirm relations exist
- [ ] Use `pgauthz_expand()` to visualize resolution
- [ ] Enable debug logging
- [ ] Check cache configuration
- [ ] Verify context variables (for conditions)
- [ ] Test with simplified policy
- [ ] Check error codes and messages
- [ ] Review PostgreSQL logs
- [ ] Monitor performance metrics

## Getting Help

If you're still stuck:

1. **Check Documentation:**
   - [API Reference](api-reference.md)
   - [Configuration Guide](configuration.md)
   - [Performance Guide](performance.md)

2. **Search Issues:**
   - [GitHub Issues](https://github.com/zvectorlabs/pgauthz/issues)

3. **Ask for Help:**
   - [GitHub Discussions](https://github.com/zvectorlabs/pgauthz/discussions)

4. **Report a Bug:**
   Include:
   - pgauthz version
   - PostgreSQL version
   - Policy definition
   - Steps to reproduce
   - Error messages
   - Debug logs

## See Also

- [API Reference](api-reference.md) - Function documentation
- [Configuration Guide](configuration.md) - Tuning parameters
- [Performance Guide](performance.md) - Optimization strategies
- [Observability Guide](observability.md) - Monitoring and metrics
