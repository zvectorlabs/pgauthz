# Performance Guide

Comprehensive guide to optimizing pgauthz for production workloads.

## Overview

pgauthz is designed for high performance with multi-level caching, batch operations, and efficient query patterns. This guide covers optimization strategies and best practices.

## Performance Characteristics

### Typical Latencies

With proper caching:
- **Cached checks**: 1-5ms
- **Uncached checks**: 10-50ms (depending on policy complexity)
- **List operations**: 20-100ms (depending on result set size)

### Throughput

On modern hardware:
- **Cached checks**: 10,000+ checks/second per core
- **Uncached checks**: 500-2,000 checks/second per core

## Caching Strategy

### Three-Level Cache

pgauthz uses a three-level cache hierarchy:

1. **L1 Cache (Model)** - Parsed authorization policies
2. **L2 Cache (Result)** - Permission check results
3. **L3 Cache (Tuple)** - Relation query results

### Recommended Cache Configuration

**Development:**
```sql
SET authz.model_cache_ttl_secs = 60;
SET authz.result_cache_ttl_secs = 30;
SET authz.tuple_cache_ttl_secs = 30;
SET authz.cache_max_capacity = 1000;
```

**Production (Low Traffic):**
```sql
SET authz.model_cache_ttl_secs = 300;
SET authz.result_cache_ttl_secs = 60;
SET authz.tuple_cache_ttl_secs = 60;
SET authz.cache_max_capacity = 10000;
```

**Production (High Traffic):**
```sql
SET authz.model_cache_ttl_secs = 600;
SET authz.result_cache_ttl_secs = 120;
SET authz.tuple_cache_ttl_secs = 120;
SET authz.cache_max_capacity = 100000;
```

**High-Churn Environments:**
```sql
SET authz.model_cache_ttl_secs = 300;
SET authz.result_cache_ttl_secs = 10;
SET authz.tuple_cache_ttl_secs = 10;
SET authz.cache_max_capacity = 50000;
```

### Cache Tuning

Monitor cache hit rates and adjust TTLs:

```sql
-- Enable metrics
SET authz.otel_enabled = true;

-- Check cache hit rate via metrics
-- Target: >90% hit rate for L1, >70% for L2/L3
```

If hit rate is low:
- **Increase TTLs** - Cache entries longer
- **Increase capacity** - Store more entries

If data is stale:
- **Decrease TTLs** - Expire entries faster
- **Use revision quantization** - Group similar revisions

### Revision Quantization

Group revisions into time buckets to improve cache hit rates:

```sql
-- 5-second buckets (default)
SET authz.revision_quantization_secs = 5;

-- 30-second buckets (better cache hits, slightly staler data)
SET authz.revision_quantization_secs = 30;

-- Disable (exact revisions, lower cache hits)
SET authz.revision_quantization_secs = 0;
```

## Query Optimization

### Use Batch Operations

**Bad:**
```sql
-- Multiple individual checks
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');
SELECT pgauthz_check('document', 'doc2', 'viewer', 'user', 'alice');
SELECT pgauthz_check('document', 'doc3', 'viewer', 'user', 'alice');
```

**Good:**
```sql
-- Single list operation
SELECT * FROM pgauthz_list_objects('user', 'alice', 'viewer', 'document');
```

### Pagination

Always paginate list operations:

```sql
-- Good: paginated
SELECT * FROM pgauthz_list_objects(
    'user', 'alice', 'viewer', 'document',
    100,  -- page_size
    NULL  -- continuation_token
);

-- Bad: no limit (may return thousands of results)
SELECT * FROM pgauthz_list_objects(
    'user', 'alice', 'viewer', 'document',
    10000,
    NULL
);
```

### Minimize Context Size

For conditional checks, keep context small:

**Bad:**
```sql
SELECT pgauthz_check_with_context(
    'document', 'doc1', 'viewer', 'user', 'alice',
    '{"large_array": [1,2,3,...,1000], "unused_field": "value"}'::jsonb
);
```

**Good:**
```sql
SELECT pgauthz_check_with_context(
    'document', 'doc1', 'viewer', 'user', 'alice',
    '{"hour": 14}'::jsonb
);
```

## Policy Optimization

### Minimize Depth

**Bad (Deep Hierarchy):**
```sql
type document {
  relations
    define level1: [user]
    define level2: level1
    define level3: level2
    define level4: level3
    define level5: level4
}
```

**Good (Flat):**
```sql
type document {
  relations
    define viewer: [user]
    define editor: [user] | viewer
    define owner: [user] | editor
}
```

### Avoid Excessive Unions

**Bad:**
```sql
type document {
  relations
    define viewer: [user] | group1#member | group2#member | group3#member | group4#member | group5#member
}
```

**Good:**
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

### Use Direct Relations When Possible

**Bad (Computed):**
```sql
type document {
  relations
    define viewer: editor
    define editor: owner
    define owner: [user]
}
```

**Good (Direct):**
```sql
type document {
  relations
    define viewer: [user]
    define editor: [user]
    define owner: [user]
}
```

## Database Optimization

### Indexes

pgauthz automatically creates indexes on tuple tables. Verify they exist:

```sql
\d+ authz_tuples
```

Expected indexes:
- Primary key on (object_type, object_id, relation, subject_type, subject_id)
- Index on object_type
- Index on subject_type, subject_id

### Connection Pooling

Use connection pooling to reduce overhead:

```python
# Python example with psycopg2
from psycopg2 import pool

connection_pool = pool.SimpleConnectionPool(
    minconn=10,
    maxconn=100,
    host="localhost",
    database="mydb"
)
```

### Prepared Statements

Use prepared statements for repeated queries:

```python
# Python example
cursor.execute(
    "PREPARE check_plan AS "
    "SELECT pgauthz_check($1, $2, $3, $4, $5)"
)

cursor.execute(
    "EXECUTE check_plan(%s, %s, %s, %s, %s)",
    ('document', 'doc1', 'viewer', 'user', 'alice')
)
```

## Check Strategy

### Batch vs Parallel

pgauthz supports two check strategies:

**Batch (Default):**
- Groups database queries together
- Lower latency for local PostgreSQL
- Better throughput

**Parallel:**
- Executes checks in parallel
- Better for high-latency datastores
- Higher CPU usage

Benchmark both:

```sql
-- Test batch
SET authz.check_strategy = 'batch';
\timing on
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');

-- Test parallel
SET authz.check_strategy = 'parallel';
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');
```

Use batch for most cases. Use parallel only if:
- Database is remote (high network latency)
- Checks involve many sub-dispatches
- You have spare CPU cores

## Monitoring Performance

### Key Metrics

Monitor these metrics (via OpenTelemetry):

1. **Check Latency (P95)** - Target: <50ms
2. **Cache Hit Rate** - Target: >70%
3. **Resolution Depth** - Target: <5 levels
4. **Datastore Queries per Check** - Target: <10

### Performance Queries

```sql
-- Enable timing
\timing on

-- Warm up cache
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');

-- Measure cached performance
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');

-- Measure uncached performance
SET authz.result_cache_ttl_secs = 0;
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');
```

## Benchmarking

### Simple Benchmark

```sql
-- Create test data
SELECT pgauthz_define_policy('
  type user {}
  type document {
    relations
      define viewer: [user]
  }
');

-- Add 1000 relations
DO $$
BEGIN
  FOR i IN 1..1000 LOOP
    PERFORM pgauthz_add_relation(
      'document',
      'doc' || i,
      'viewer',
      'user',
      'user' || (i % 100)
    );
  END LOOP;
END $$;

-- Benchmark checks
\timing on
SELECT COUNT(*) FROM (
  SELECT pgauthz_check('document', 'doc' || i, 'viewer', 'user', 'user' || (i % 100))
  FROM generate_series(1, 1000) i
) t;
```

### Load Testing

Use tools like pgbench:

```bash
# Create test script
cat > check_bench.sql <<EOF
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');
EOF

# Run benchmark
pgbench -f check_bench.sql -c 10 -j 4 -t 1000 mydb
```

## Scaling Strategies

### Vertical Scaling

Increase PostgreSQL resources:
- **CPU**: More cores for parallel checks
- **Memory**: Larger cache capacity
- **Storage**: Faster SSD for tuple queries

### Horizontal Scaling

Use read replicas for read-heavy workloads:

```sql
-- Write to primary
SELECT pgauthz_add_relation('document', 'doc1', 'viewer', 'user', 'alice');

-- Read from replica
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');
```

### Partitioning

For very large tuple sets, consider partitioning:

```sql
-- Partition by object_type
CREATE TABLE authz_tuples_documents PARTITION OF authz_tuples
  FOR VALUES IN ('document');

CREATE TABLE authz_tuples_folders PARTITION OF authz_tuples
  FOR VALUES IN ('folder');
```

## Production Checklist

Before going to production:

- [ ] Enable caching with appropriate TTLs
- [ ] Set cache capacity based on memory budget
- [ ] Configure revision quantization
- [ ] Enable OpenTelemetry metrics
- [ ] Set up monitoring and alerts
- [ ] Benchmark with production-like data
- [ ] Test cache hit rates
- [ ] Verify indexes exist
- [ ] Use connection pooling
- [ ] Set appropriate log level (info or warn)
- [ ] Document your policy for team reference

## Performance Troubleshooting

### Slow Checks

If checks are slow (>100ms):

1. **Check cache hit rate** - Should be >70%
2. **Enable debug logging** - Look for cache misses
3. **Use pgauthz_expand()** - Check resolution depth
4. **Simplify policy** - Reduce depth and unions
5. **Increase cache TTLs** - Cache longer
6. **Check database performance** - Slow queries?

### High Memory Usage

If memory usage is high:

1. **Reduce cache capacity**
2. **Reduce cache TTLs**
3. **Monitor cache eviction rate**
4. **Consider partitioning large tuple sets**

### Low Throughput

If throughput is low:

1. **Use batch operations** - Avoid individual checks
2. **Enable caching** - Reduce database load
3. **Use connection pooling** - Reduce connection overhead
4. **Optimize policy** - Reduce complexity
5. **Scale horizontally** - Add read replicas

## Best Practices

1. **Cache Aggressively** - Use long TTLs for policies, shorter for results
2. **Batch Operations** - Use list functions instead of individual checks
3. **Monitor Metrics** - Track latency, hit rates, and errors
4. **Simplify Policies** - Avoid deep hierarchies and excessive unions
5. **Use Pagination** - Always limit result set sizes
6. **Test at Scale** - Benchmark with production-like data
7. **Plan for Growth** - Design policies that scale
8. **Document Policies** - Make policies maintainable

## See Also

- [Configuration Guide](configuration.md) - Tuning parameters
- [Observability Guide](observability.md) - Monitoring and metrics
- [Debugging Guide](debugging.md) - Troubleshooting tips
- [API Reference](api-reference.md) - Function documentation
