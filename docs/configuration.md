# Configuration Guide

Complete guide to configuring pgauthz for optimal performance and observability.

## Overview

pgauthz is configured through PostgreSQL GUC (Grand Unified Configuration) parameters. All settings can be configured at the database, session, or transaction level.

## Configuration Methods

### Database-Level Configuration

Add settings to `postgresql.conf`:

```ini
# pgauthz configuration
authz.check_strategy = 'batch'
authz.tracing_level = 'info'
authz.model_cache_ttl_secs = 300
authz.result_cache_ttl_secs = 60
authz.tuple_cache_ttl_secs = 60
authz.cache_max_capacity = 10000
authz.revision_quantization_secs = 5
authz.otel_enabled = false
```

Then reload configuration:

```sql
SELECT pg_reload_conf();
```

### Session-Level Configuration

Set parameters for your current session:

```sql
SET authz.check_strategy = 'parallel';
SET authz.tracing_level = 'debug';
SET authz.model_cache_ttl_secs = 600;
```

### Transaction-Level Configuration

Set parameters for the current transaction only:

```sql
BEGIN;
SET LOCAL authz.tracing_level = 'debug';
-- Your queries here
COMMIT;
```

## Core Parameters

### authz.check_strategy

Controls the strategy for permission checks.

**Type:** `string`  
**Default:** `'batch'`  
**Valid Values:** `'batch'`, `'parallel'`

**Description:**
- `batch` - Batch database queries together (recommended for most cases)
- `parallel` - Execute checks in parallel (useful for high-latency datastores)

**Example:**
```sql
SET authz.check_strategy = 'batch';
```

**When to Use:**
- Use `batch` for local PostgreSQL (lower latency, better throughput)
- Use `parallel` for remote datastores or high-latency networks

---

### authz.tracing_level

Sets the logging verbosity for pgauthz operations.

**Type:** `string`  
**Default:** `'info'`  
**Valid Values:** `'error'`, `'warn'`, `'info'`, `'debug'`, `'trace'`

**Description:**
Controls how much detail is logged:
- `error` - Only errors
- `warn` - Errors and warnings
- `info` - General information (recommended for production)
- `debug` - Detailed debugging information
- `trace` - Very detailed tracing (performance impact)

**Example:**
```sql
SET authz.tracing_level = 'debug';
```

**Production Recommendation:** Use `'info'` or `'warn'` in production. Use `'debug'` or `'trace'` only for troubleshooting.

---

## Caching Parameters

### authz.model_cache_ttl_secs

Time-to-live for cached authorization policy models (L1 cache).

**Type:** `integer`  
**Default:** `0` (disabled)  
**Range:** `0` to `2147483647`  
**Unit:** seconds

**Description:**
Caches parsed policy models to avoid re-parsing on every check. Since policies change infrequently, a long TTL (5-10 minutes) is recommended.

**Example:**
```sql
SET authz.model_cache_ttl_secs = 300;  -- 5 minutes
```

**Recommendation:**
- Development: `60` (1 minute)
- Production: `300` (5 minutes) or higher
- Set to `0` to disable caching

---

### authz.result_cache_ttl_secs

Time-to-live for cached permission check results (L2 cache).

**Type:** `integer`  
**Default:** `0` (disabled)  
**Range:** `0` to `2147483647`  
**Unit:** seconds

**Description:**
Caches the results of permission checks. Use shorter TTLs since permissions can change more frequently than policies.

**Example:**
```sql
SET authz.result_cache_ttl_secs = 60;  -- 1 minute
```

**Recommendation:**
- Development: `30` (30 seconds)
- Production: `60` (1 minute) to `300` (5 minutes)
- High-churn environments: `10` to `30` seconds
- Set to `0` to disable caching

---

### authz.tuple_cache_ttl_secs

Time-to-live for cached tuple query results (L3 cache).

**Type:** `integer`  
**Default:** `0` (disabled)  
**Range:** `0` to `2147483647`  
**Unit:** seconds

**Description:**
Caches the results of tuple queries (relation lookups). Use shorter TTLs since relations change frequently.

**Example:**
```sql
SET authz.tuple_cache_ttl_secs = 60;  -- 1 minute
```

**Recommendation:**
- Development: `30` (30 seconds)
- Production: `60` (1 minute) to `120` (2 minutes)
- High-churn environments: `10` to `30` seconds
- Set to `0` to disable caching

---

### authz.cache_max_capacity

Maximum number of entries per cache layer.

**Type:** `integer`  
**Default:** `10000`  
**Range:** `1` to `2147483647`

**Description:**
Limits memory usage by capping the number of cached entries. When the limit is reached, least-recently-used entries are evicted.

**Example:**
```sql
SET authz.cache_max_capacity = 50000;
```

**Recommendation:**
- Small deployments: `10000` (default)
- Medium deployments: `50000` to `100000`
- Large deployments: `100000` to `500000`

**Memory Estimation:**
Each cache entry uses approximately 1-2 KB. A capacity of 10,000 uses ~10-20 MB per cache layer.

---

### authz.revision_quantization_secs

Quantization interval for revision-based cache keys.

**Type:** `integer`  
**Default:** `5`  
**Range:** `0` to `3600`  
**Unit:** seconds

**Description:**
Groups revisions into time buckets to improve cache hit rates. A value of `5` means revisions within a 5-second window use the same cache key.

**Example:**
```sql
SET authz.revision_quantization_secs = 10;
```

**Recommendation:**
- High-churn environments: `5` to `10` seconds
- Low-churn environments: `30` to `60` seconds
- Set to `0` to disable quantization (use exact revisions)

---

## OpenTelemetry Parameters

### authz.otel_enabled

Enable OpenTelemetry tracing and metrics.

**Type:** `boolean`  
**Default:** `false`

**Description:**
Enables OpenTelemetry instrumentation for distributed tracing and metrics collection.

**Example:**
```sql
SET authz.otel_enabled = true;
```

**Recommendation:**
- Development: `false` (unless testing observability)
- Production: `true` (for monitoring and debugging)

---

### authz.otel_endpoint

OpenTelemetry OTLP endpoint URL.

**Type:** `string`  
**Default:** `'http://localhost:4317'`

**Description:**
The endpoint where OpenTelemetry data is sent (typically an OTLP collector).

**Example:**
```sql
SET authz.otel_endpoint = 'http://otel-collector:4317';
```

**Common Endpoints:**
- Local collector: `http://localhost:4317`
- Jaeger: `http://jaeger:4317`
- Tempo: `http://tempo:4317`
- Cloud providers: Use provider-specific endpoints

---

### authz.otel_service_name

Service name for OpenTelemetry traces.

**Type:** `string`  
**Default:** `'pgauthz'`

**Description:**
Identifies this service in distributed traces.

**Example:**
```sql
SET authz.otel_service_name = 'my-app-authz';
```

---

### authz.otel_trace_sampling_ratio

Percentage of traces to sample.

**Type:** `integer`  
**Default:** `100`  
**Range:** `0` to `100`  
**Unit:** percentage

**Description:**
Controls what percentage of traces are sampled and sent to the collector.

**Example:**
```sql
SET authz.otel_trace_sampling_ratio = 10;  -- Sample 10% of traces
```

**Recommendation:**
- Development: `100` (sample everything)
- Production (low traffic): `100`
- Production (high traffic): `1` to `10`

---

## Configuration Profiles

### Development Profile

Optimized for fast iteration and debugging:

```sql
SET authz.check_strategy = 'batch';
SET authz.tracing_level = 'debug';
SET authz.model_cache_ttl_secs = 60;
SET authz.result_cache_ttl_secs = 30;
SET authz.tuple_cache_ttl_secs = 30;
SET authz.cache_max_capacity = 1000;
SET authz.revision_quantization_secs = 5;
SET authz.otel_enabled = false;
```

### Production Profile (Low Traffic)

Balanced configuration for small to medium deployments:

```sql
SET authz.check_strategy = 'batch';
SET authz.tracing_level = 'info';
SET authz.model_cache_ttl_secs = 300;
SET authz.result_cache_ttl_secs = 60;
SET authz.tuple_cache_ttl_secs = 60;
SET authz.cache_max_capacity = 10000;
SET authz.revision_quantization_secs = 10;
SET authz.otel_enabled = true;
SET authz.otel_trace_sampling_ratio = 100;
```

### Production Profile (High Traffic)

Optimized for high-throughput environments:

```sql
SET authz.check_strategy = 'batch';
SET authz.tracing_level = 'warn';
SET authz.model_cache_ttl_secs = 600;
SET authz.result_cache_ttl_secs = 120;
SET authz.tuple_cache_ttl_secs = 120;
SET authz.cache_max_capacity = 100000;
SET authz.revision_quantization_secs = 30;
SET authz.otel_enabled = true;
SET authz.otel_trace_sampling_ratio = 1;
```

### High-Churn Profile

For environments with frequent permission changes:

```sql
SET authz.check_strategy = 'batch';
SET authz.tracing_level = 'info';
SET authz.model_cache_ttl_secs = 300;
SET authz.result_cache_ttl_secs = 10;
SET authz.tuple_cache_ttl_secs = 10;
SET authz.cache_max_capacity = 50000;
SET authz.revision_quantization_secs = 5;
SET authz.otel_enabled = true;
SET authz.otel_trace_sampling_ratio = 10;
```

## Viewing Current Configuration

Check current settings:

```sql
-- View all pgauthz settings
SELECT name, setting, unit, short_desc
FROM pg_settings
WHERE name LIKE 'authz.%'
ORDER BY name;

-- Check specific setting
SHOW authz.check_strategy;
SHOW authz.model_cache_ttl_secs;
```

## Performance Tuning

### Cache Hit Rate Monitoring

Monitor cache effectiveness:

```sql
-- Enable metrics
SET authz.otel_enabled = true;

-- Check cache hit rates (via metrics)
-- See Observability Guide for details
```

### Adjusting Cache TTLs

If you see:
- **Low cache hit rates**: Increase TTLs
- **Stale permission data**: Decrease TTLs
- **High memory usage**: Decrease `cache_max_capacity`

### Choosing Check Strategy

Benchmark both strategies:

```sql
-- Test batch strategy
SET authz.check_strategy = 'batch';
\timing on
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');

-- Test parallel strategy
SET authz.check_strategy = 'parallel';
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');
```

## Security Considerations

### Restricting Configuration Changes

Limit who can change pgauthz settings:

```sql
-- Only superusers can change database-level settings
-- Grant specific users permission to change session settings
GRANT SET ON PARAMETER authz.tracing_level TO app_user;
```

### Sensitive Data in Logs

Be careful with `trace` level logging in production:

```sql
-- Avoid in production (may log sensitive data)
SET authz.tracing_level = 'trace';

-- Use info or warn instead
SET authz.tracing_level = 'info';
```

## Troubleshooting

### Configuration Not Taking Effect

1. Check if setting is valid:
```sql
SHOW authz.check_strategy;
```

2. Verify setting level (database vs session):
```sql
SELECT name, setting, source FROM pg_settings WHERE name = 'authz.check_strategy';
```

3. Reload configuration if changed in postgresql.conf:
```sql
SELECT pg_reload_conf();
```

### Cache Not Working

Verify cache is enabled:

```sql
SHOW authz.model_cache_ttl_secs;
SHOW authz.result_cache_ttl_secs;
SHOW authz.tuple_cache_ttl_secs;
```

If all show `0`, caching is disabled. Set appropriate TTLs.

## See Also

- [Performance Guide](performance.md) - Optimization strategies
- [Observability Guide](observability.md) - Monitoring and metrics
- [API Reference](api-reference.md) - Function documentation
