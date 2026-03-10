# Performance Testing Example

Guide to benchmarking and performance testing your pgauthz deployment.

## Overview

This guide covers:
- Setting up performance tests
- Benchmarking authorization checks
- Measuring cache effectiveness
- Load testing strategies
- Interpreting results

## Basic Performance Test

### Step 1: Create Test Data

```sql
-- Create a simple policy
SELECT pgauthz_define_policy('
  type user {}
  type document {
    relations
      define viewer: [user]
      define editor: [user]
  }
');

-- Generate test data
DO $$
BEGIN
  -- Create 1000 documents
  FOR i IN 1..1000 LOOP
    -- Each document has 10 viewers
    FOR j IN 1..10 LOOP
      PERFORM pgauthz_add_relation(
        'document',
        'doc' || i,
        'viewer',
        'user',
        'user' || ((i * 10 + j) % 100)
      );
    END LOOP;
  END LOOP;
END $$;
```

### Step 2: Benchmark Checks

```sql
-- Enable timing
\timing on

-- Cold cache (first run)
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'user1');

-- Warm cache (subsequent runs)
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'user1');
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'user1');
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'user1');

-- Measure average over 100 checks
SELECT AVG(duration) FROM (
  SELECT
    extract(epoch from (clock_timestamp() - start_time)) * 1000 as duration
  FROM (
    SELECT clock_timestamp() as start_time
  ) t,
  LATERAL (
    SELECT pgauthz_check('document', 'doc' || (i % 1000), 'viewer', 'user', 'user' || (i % 100))
    FROM generate_series(1, 100) i
  ) checks
) timings;
```

### Step 3: Benchmark List Operations

```sql
-- Time list_objects
\timing on
SELECT COUNT(*) FROM pgauthz_list_objects('user', 'user1', 'viewer', 'document');

-- Time list_subjects
SELECT COUNT(*) FROM pgauthz_list_subjects('document', 'doc1', 'viewer', 'user');
```

## Cache Performance Testing

### Test Cache Hit Rates

```sql
-- Configure caching
SET authz.model_cache_ttl_secs = 300;
SET authz.result_cache_ttl_secs = 60;
SET authz.tuple_cache_ttl_secs = 60;
SET authz.tracing_level = 'debug';

-- First check (cache miss)
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'user1');
-- Look for "cache_miss" in output

-- Second check (cache hit)
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'user1');
-- Look for "cache_hit" in output

-- Measure cache effectiveness
DO $$
DECLARE
  start_time timestamp;
  cold_duration interval;
  warm_duration interval;
BEGIN
  -- Cold cache
  SET authz.result_cache_ttl_secs = 0;
  start_time := clock_timestamp();
  PERFORM pgauthz_check('document', 'doc1', 'viewer', 'user', 'user1');
  cold_duration := clock_timestamp() - start_time;
  
  -- Warm cache
  SET authz.result_cache_ttl_secs = 60;
  PERFORM pgauthz_check('document', 'doc1', 'viewer', 'user', 'user1');
  start_time := clock_timestamp();
  PERFORM pgauthz_check('document', 'doc1', 'viewer', 'user', 'user1');
  warm_duration := clock_timestamp() - start_time;
  
  RAISE NOTICE 'Cold cache: % ms', extract(epoch from cold_duration) * 1000;
  RAISE NOTICE 'Warm cache: % ms', extract(epoch from warm_duration) * 1000;
  RAISE NOTICE 'Speedup: %x', cold_duration / warm_duration;
END $$;
```

## Load Testing with pgbench

### Create Test Script

```bash
# Create check_bench.sql
cat > check_bench.sql <<'EOF'
\set doc_id random(1, 1000)
\set user_id random(1, 100)
SELECT pgauthz_check('document', 'doc' || :doc_id, 'viewer', 'user', 'user' || :user_id);
EOF

# Run benchmark
pgbench -f check_bench.sql -c 10 -j 4 -t 1000 mydb
```

### Interpret Results

```
transaction type: check_bench.sql
scaling factor: 1
query mode: simple
number of clients: 10
number of threads: 4
number of transactions per client: 1000
number of transactions actually processed: 10000/10000
latency average = 5.234 ms
tps = 1910.234 (including connections establishing)
tps = 1912.456 (excluding connections establishing)
```

Key metrics:
- **latency average**: Average time per check
- **tps**: Transactions (checks) per second

## Python Load Testing

### Using locust

```python
# locustfile.py
from locust import User, task, between
import psycopg2
import random

class PgAuthzUser(User):
    wait_time = between(0.1, 0.5)
    
    def on_start(self):
        self.conn = psycopg2.connect("dbname=mydb")
        self.cur = self.conn.cursor()
    
    @task(10)
    def check_permission(self):
        doc_id = f"doc{random.randint(1, 1000)}"
        user_id = f"user{random.randint(1, 100)}"
        
        self.cur.execute(
            "SELECT pgauthz_check(%s, %s, %s, %s, %s)",
            ('document', doc_id, 'viewer', 'user', user_id)
        )
        self.cur.fetchone()
    
    @task(1)
    def list_objects(self):
        user_id = f"user{random.randint(1, 100)}"
        
        self.cur.execute(
            "SELECT * FROM pgauthz_list_objects(%s, %s, %s, %s)",
            ('user', user_id, 'viewer', 'document')
        )
        self.cur.fetchall()
    
    def on_stop(self):
        self.cur.close()
        self.conn.close()
```

Run the test:
```bash
locust -f locustfile.py --host=localhost
# Open http://localhost:8089 in browser
```

## Stress Testing

### High Concurrency Test

```sql
-- Create test function
CREATE OR REPLACE FUNCTION stress_test_checks(num_iterations int)
RETURNS TABLE(iteration int, duration_ms numeric) AS $$
DECLARE
  start_time timestamp;
  end_time timestamp;
BEGIN
  FOR i IN 1..num_iterations LOOP
    start_time := clock_timestamp();
    
    PERFORM pgauthz_check(
      'document',
      'doc' || (random() * 1000)::int,
      'viewer',
      'user',
      'user' || (random() * 100)::int
    );
    
    end_time := clock_timestamp();
    
    iteration := i;
    duration_ms := extract(epoch from (end_time - start_time)) * 1000;
    RETURN NEXT;
  END LOOP;
END;
$$ LANGUAGE plpgsql;

-- Run stress test
SELECT
  COUNT(*) as total_checks,
  AVG(duration_ms) as avg_ms,
  MIN(duration_ms) as min_ms,
  MAX(duration_ms) as max_ms,
  PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY duration_ms) as p50_ms,
  PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY duration_ms) as p95_ms,
  PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY duration_ms) as p99_ms
FROM stress_test_checks(10000);
```

## Complex Policy Performance

### Test Deep Hierarchies

```sql
-- Create policy with deep hierarchy
SELECT pgauthz_define_policy('
  type user {}
  type document {
    relations
      define level1: [user]
      define level2: level1
      define level3: level2
      define level4: level3
      define level5: level4
  }
');

-- Add relations
SELECT pgauthz_add_relation('document', 'deep_doc', 'level1', 'user', 'alice');

-- Benchmark deep check
\timing on
SELECT pgauthz_check('document', 'deep_doc', 'level5', 'user', 'alice');

-- Use expand to see complexity
SELECT pgauthz_expand('document', 'deep_doc', 'level5');
```

### Test Wide Unions

```sql
-- Create policy with many unions
SELECT pgauthz_define_policy('
  type user {}
  type group {
    relations
      define member: [user]
  }
  type document {
    relations
      define viewer: [user] | group1#member | group2#member | group3#member | group4#member | group5#member
  }
');

-- Add relations to multiple groups
DO $$
BEGIN
  FOR i IN 1..5 LOOP
    PERFORM pgauthz_add_relation('group', 'group' || i, 'member', 'user', 'alice');
  END LOOP;
  PERFORM pgauthz_add_relation('document', 'wide_doc', 'viewer', 'group', 'group1');
END $$;

-- Benchmark
\timing on
SELECT pgauthz_check('document', 'wide_doc', 'viewer', 'user', 'alice');
```

## Monitoring During Tests

### Enable Metrics

```sql
SET authz.otel_enabled = true;
SET authz.otel_endpoint = 'http://localhost:4317';
```

### Watch Key Metrics

Monitor these during load tests:
- Check latency (P50, P95, P99)
- Cache hit rate
- Resolution depth
- Datastore queries per check
- Error rate

### PostgreSQL Monitoring

```sql
-- Monitor active queries
SELECT pid, query, state, wait_event_type, wait_event
FROM pg_stat_activity
WHERE query LIKE '%pgauthz%';

-- Monitor table statistics
SELECT schemaname, tablename, seq_scan, idx_scan, n_tup_ins, n_tup_upd, n_tup_del
FROM pg_stat_user_tables
WHERE tablename LIKE '%authz%';

-- Monitor index usage
SELECT schemaname, tablename, indexname, idx_scan, idx_tup_read, idx_tup_fetch
FROM pg_stat_user_indexes
WHERE tablename LIKE '%authz%';
```

## Optimization Based on Results

### If Latency is High

1. **Enable caching:**
```sql
SET authz.model_cache_ttl_secs = 300;
SET authz.result_cache_ttl_secs = 60;
```

2. **Simplify policy:**
- Reduce hierarchy depth
- Minimize unions
- Use direct relations

3. **Add indexes:**
```sql
-- Check existing indexes
\d+ authz_tuples
```

### If Cache Hit Rate is Low

1. **Increase TTLs:**
```sql
SET authz.result_cache_ttl_secs = 120;
SET authz.tuple_cache_ttl_secs = 120;
```

2. **Increase capacity:**
```sql
SET authz.cache_max_capacity = 50000;
```

3. **Enable revision quantization:**
```sql
SET authz.revision_quantization_secs = 10;
```

### If Memory Usage is High

1. **Reduce cache capacity:**
```sql
SET authz.cache_max_capacity = 5000;
```

2. **Reduce TTLs:**
```sql
SET authz.result_cache_ttl_secs = 30;
```

## Benchmark Report Template

```markdown
# pgauthz Performance Benchmark Report

## Test Environment
- PostgreSQL Version: 16.1
- pgauthz Version: 1.0.0
- Hardware: 4 CPU cores, 16GB RAM
- Dataset: 1000 documents, 100 users, 10000 relations

## Configuration
- model_cache_ttl_secs: 300
- result_cache_ttl_secs: 60
- tuple_cache_ttl_secs: 60
- cache_max_capacity: 10000

## Results

### Check Performance
- Cold cache: 25ms average
- Warm cache: 2ms average
- Speedup: 12.5x

### Throughput
- Single client: 500 checks/sec
- 10 concurrent clients: 3500 checks/sec

### Latency Percentiles
- P50: 2.1ms
- P95: 5.3ms
- P99: 12.7ms

### Cache Effectiveness
- L1 hit rate: 95%
- L2 hit rate: 78%
- L3 hit rate: 82%

## Recommendations
1. Current configuration is optimal for this workload
2. Consider increasing cache capacity for larger datasets
3. Monitor cache hit rates in production
```

## See Also

- [Performance Guide](../performance.md) - Optimization strategies
- [Configuration Guide](../configuration.md) - Tuning parameters
- [Observability Guide](../observability.md) - Monitoring metrics
