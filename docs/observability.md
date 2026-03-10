# Observability Guide

Complete guide to monitoring pgauthz with OpenTelemetry metrics and tracing.

## Overview

pgauthz provides comprehensive observability through:
- **OpenTelemetry Metrics** - Performance and usage metrics
- **OpenTelemetry Tracing** - Distributed tracing for debugging
- **PostgreSQL Logs** - Structured logging with tracing integration

## Quick Start

Enable OpenTelemetry:

```sql
SET authz.otel_enabled = true;
SET authz.otel_endpoint = 'http://localhost:4317';
SET authz.otel_service_name = 'pgauthz';
```

## OpenTelemetry Setup

### Prerequisites

You need an OpenTelemetry collector or compatible backend:
- **Jaeger** - Distributed tracing
- **Prometheus** - Metrics collection
- **Grafana Tempo** - Tracing backend
- **Grafana Loki** - Log aggregation
- **Cloud providers** - AWS X-Ray, Google Cloud Trace, Azure Monitor

### Local Setup with Docker

Run an OpenTelemetry collector:

```bash
docker run -d \
  --name otel-collector \
  -p 4317:4317 \
  -p 4318:4318 \
  otel/opentelemetry-collector:latest
```

Run Jaeger for tracing:

```bash
docker run -d \
  --name jaeger \
  -p 16686:16686 \
  -p 4317:4317 \
  jaegertracing/all-in-one:latest
```

Configure pgauthz:

```sql
SET authz.otel_enabled = true;
SET authz.otel_endpoint = 'http://localhost:4317';
```

Access Jaeger UI at http://localhost:16686

### Production Setup

For production, use a dedicated OpenTelemetry collector:

```yaml
# otel-collector-config.yaml
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4317

processors:
  batch:
    timeout: 10s
    send_batch_size: 1024

exporters:
  prometheus:
    endpoint: "0.0.0.0:8889"
  jaeger:
    endpoint: "jaeger:14250"
    tls:
      insecure: true

service:
  pipelines:
    traces:
      receivers: [otlp]
      processors: [batch]
      exporters: [jaeger]
    metrics:
      receivers: [otlp]
      processors: [batch]
      exporters: [prometheus]
```

## Available Metrics

### Request Metrics

#### pgauthz.check.duration

Histogram of authorization check durations.

**Type:** Histogram (seconds)  
**Labels:**
- `result` - "allowed" or "denied"
- `object_type` - Type of object checked
- `relation` - Relation checked

**Example Query (PromQL):**
```promql
# Average check duration
rate(pgauthz_check_duration_sum[5m]) / rate(pgauthz_check_duration_count[5m])

# P95 latency
histogram_quantile(0.95, rate(pgauthz_check_duration_bucket[5m]))

# Check duration by object type
sum(rate(pgauthz_check_duration_sum[5m])) by (object_type)
```

---

#### pgauthz.check.total

Counter of total authorization checks.

**Type:** Counter  
**Labels:**
- `result` - "allowed" or "denied"
- `object_type` - Type of object checked
- `relation` - Relation checked

**Example Query (PromQL):**
```promql
# Checks per second
rate(pgauthz_check_total[5m])

# Allowed vs denied ratio
sum(rate(pgauthz_check_total{result="allowed"}[5m])) /
sum(rate(pgauthz_check_total[5m]))

# Checks by object type
sum(rate(pgauthz_check_total[5m])) by (object_type)
```

---

#### pgauthz.write_tuples.total

Counter of tuple write operations.

**Type:** Counter  
**Labels:**
- `operation` - "write" or "delete"

**Example Query (PromQL):**
```promql
# Writes per second
rate(pgauthz_write_tuples_total{operation="write"}[5m])

# Deletes per second
rate(pgauthz_write_tuples_total{operation="delete"}[5m])
```

---

#### pgauthz.read_tuples.total

Counter of tuple read operations.

**Type:** Counter  
**Labels:**
- `object_type` - Type of objects read

**Example Query (PromQL):**
```promql
# Reads per second
rate(pgauthz_read_tuples_total[5m])

# Reads by object type
sum(rate(pgauthz_read_tuples_total[5m])) by (object_type)
```

---

### Cache Metrics

#### pgauthz.cache.hits.total

Counter of cache hits.

**Type:** Counter  
**Labels:**
- `cache_level` - "L1" (model), "L2" (result), or "L3" (tuple)

**Example Query (PromQL):**
```promql
# Cache hit rate
sum(rate(pgauthz_cache_hits_total[5m])) /
(sum(rate(pgauthz_cache_hits_total[5m])) + sum(rate(pgauthz_cache_misses_total[5m])))

# Hit rate by cache level
sum(rate(pgauthz_cache_hits_total[5m])) by (cache_level) /
(sum(rate(pgauthz_cache_hits_total[5m])) by (cache_level) + 
 sum(rate(pgauthz_cache_misses_total[5m])) by (cache_level))
```

---

#### pgauthz.cache.misses.total

Counter of cache misses.

**Type:** Counter  
**Labels:**
- `cache_level` - "L1" (model), "L2" (result), or "L3" (tuple)

**Example Query (PromQL):**
```promql
# Cache miss rate
sum(rate(pgauthz_cache_misses_total[5m])) /
(sum(rate(pgauthz_cache_hits_total[5m])) + sum(rate(pgauthz_cache_misses_total[5m])))
```

---

### Resolution Metrics

#### pgauthz.resolution.depth

Histogram of permission tree traversal depth.

**Type:** Histogram (integer)

**Example Query (PromQL):**
```promql
# Average resolution depth
rate(pgauthz_resolution_depth_sum[5m]) / rate(pgauthz_resolution_depth_count[5m])

# P95 depth
histogram_quantile(0.95, rate(pgauthz_resolution_depth_bucket[5m]))
```

---

#### pgauthz.resolution.dispatch_count

Histogram of sub-dispatches per check.

**Type:** Histogram (integer)

**Example Query (PromQL):**
```promql
# Average dispatches per check
rate(pgauthz_resolution_dispatch_count_sum[5m]) / 
rate(pgauthz_resolution_dispatch_count_count[5m])
```

---

#### pgauthz.resolution.datastore_queries

Histogram of datastore queries per check.

**Type:** Histogram (integer)

**Example Query (PromQL):**
```promql
# Average queries per check
rate(pgauthz_resolution_datastore_queries_sum[5m]) / 
rate(pgauthz_resolution_datastore_queries_count[5m])
```

---

#### pgauthz.tuples.read_count

Histogram of tuples read per operation.

**Type:** Histogram (integer)  
**Labels:**
- `object_type` - Type of objects read

**Example Query (PromQL):**
```promql
# Average tuples per read
rate(pgauthz_tuples_read_count_sum[5m]) / 
rate(pgauthz_tuples_read_count_count[5m])
```

---

### Error Metrics

#### pgauthz.errors.total

Counter of errors by type.

**Type:** Counter  
**Labels:**
- `error_type` - Error variant (Validation, PolicyParse, etc.)
- `operation` - Operation that failed

**Example Query (PromQL):**
```promql
# Error rate
rate(pgauthz_errors_total[5m])

# Errors by type
sum(rate(pgauthz_errors_total[5m])) by (error_type)

# Error ratio
sum(rate(pgauthz_errors_total[5m])) / 
sum(rate(pgauthz_check_total[5m]))
```

---

### Model Metrics

#### pgauthz.model.load_duration

Histogram of model loading duration.

**Type:** Histogram (seconds)  
**Labels:**
- `cache_hit` - "true" or "false"

**Example Query (PromQL):**
```promql
# Average load time
rate(pgauthz_model_load_duration_sum[5m]) / 
rate(pgauthz_model_load_duration_count[5m])

# Cache hit vs miss load time
rate(pgauthz_model_load_duration_sum{cache_hit="true"}[5m]) /
rate(pgauthz_model_load_duration_count{cache_hit="true"}[5m])
```

---

## Distributed Tracing

### Trace Spans

pgauthz creates spans for all major operations:

**Top-Level Spans:**
- `pgauthz_check` - Authorization check
- `pgauthz_define_policy` - Policy definition
- `pgauthz_write_tuples` - Tuple writes
- `pgauthz_read_tuples` - Tuple reads

**Span Attributes:**
- `authz.object_type` - Object type
- `authz.object_id` - Object ID
- `authz.relation` - Relation name
- `authz.subject_type` - Subject type
- `authz.subject_id` - Subject ID
- `authz.result` - Check result (true/false)
- `authz.has_context` - Whether context was provided

### Viewing Traces

In Jaeger UI:
1. Select service: `pgauthz`
2. Select operation: `pgauthz_check`
3. Filter by tags: `authz.object_type=document`

### Trace Sampling

Control sampling rate:

```sql
-- Sample 100% (development)
SET authz.otel_trace_sampling_ratio = 100;

-- Sample 10% (production)
SET authz.otel_trace_sampling_ratio = 10;

-- Sample 1% (high traffic)
SET authz.otel_trace_sampling_ratio = 1;
```

## Grafana Dashboards

### Example Dashboard: pgauthz Overview

```json
{
  "dashboard": {
    "title": "pgauthz Overview",
    "panels": [
      {
        "title": "Check Rate",
        "targets": [
          {
            "expr": "rate(pgauthz_check_total[5m])"
          }
        ]
      },
      {
        "title": "Check Latency (P95)",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, rate(pgauthz_check_duration_bucket[5m]))"
          }
        ]
      },
      {
        "title": "Cache Hit Rate",
        "targets": [
          {
            "expr": "sum(rate(pgauthz_cache_hits_total[5m])) / (sum(rate(pgauthz_cache_hits_total[5m])) + sum(rate(pgauthz_cache_misses_total[5m])))"
          }
        ]
      },
      {
        "title": "Error Rate",
        "targets": [
          {
            "expr": "rate(pgauthz_errors_total[5m])"
          }
        ]
      }
    ]
  }
}
```

### Key Metrics to Monitor

1. **Check Latency** - P50, P95, P99 latencies
2. **Check Rate** - Requests per second
3. **Cache Hit Rate** - Overall and per-level
4. **Error Rate** - Errors per second by type
5. **Resolution Depth** - Average and P95
6. **Datastore Queries** - Queries per check

## Alerting

### Recommended Alerts

#### High Latency

```yaml
alert: HighCheckLatency
expr: histogram_quantile(0.95, rate(pgauthz_check_duration_bucket[5m])) > 0.1
for: 5m
annotations:
  summary: "pgauthz check latency is high"
  description: "P95 latency is {{ $value }}s"
```

#### Low Cache Hit Rate

```yaml
alert: LowCacheHitRate
expr: |
  sum(rate(pgauthz_cache_hits_total[5m])) /
  (sum(rate(pgauthz_cache_hits_total[5m])) + sum(rate(pgauthz_cache_misses_total[5m]))) < 0.5
for: 10m
annotations:
  summary: "pgauthz cache hit rate is low"
  description: "Cache hit rate is {{ $value | humanizePercentage }}"
```

#### High Error Rate

```yaml
alert: HighErrorRate
expr: rate(pgauthz_errors_total[5m]) > 10
for: 5m
annotations:
  summary: "pgauthz error rate is high"
  description: "Error rate is {{ $value }} errors/sec"
```

#### High Resolution Depth

```yaml
alert: HighResolutionDepth
expr: |
  rate(pgauthz_resolution_depth_sum[5m]) /
  rate(pgauthz_resolution_depth_count[5m]) > 10
for: 10m
annotations:
  summary: "pgauthz resolution depth is high"
  description: "Average depth is {{ $value }}"
```

## PostgreSQL Logs

### Log Integration

pgauthz integrates with PostgreSQL logging:

```sql
-- Set log level
SET authz.tracing_level = 'info';

-- View logs
SELECT * FROM pg_stat_statements WHERE query LIKE '%pgauthz%';
```

### Log Levels

- `error` - Only errors (production default)
- `warn` - Errors and warnings
- `info` - General information
- `debug` - Detailed debugging
- `trace` - Very detailed (performance impact)

### Example Log Output

```
INFO: pgauthz_check: object_type=document object_id=doc1 relation=viewer result=true duration=0.002s
WARN: pgauthz_check: cache_miss cache_level=L2
ERROR: pgauthz_define_policy: policy_parse_error line=5 message="undefined type: group"
```

## Performance Monitoring

### Query Performance

Monitor slow checks:

```sql
-- Enable timing
\timing on

-- Run check
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');

-- Check execution plan
EXPLAIN ANALYZE
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');
```

### Cache Effectiveness

Monitor cache hit rates via metrics or logs:

```sql
SET authz.tracing_level = 'debug';
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');
-- Look for "cache_hit" or "cache_miss" in logs
```

## Troubleshooting

### Metrics Not Appearing

1. Verify OpenTelemetry is enabled:
```sql
SHOW authz.otel_enabled;
```

2. Check endpoint is reachable:
```bash
curl http://localhost:4317
```

3. Verify collector is running:
```bash
docker ps | grep otel-collector
```

### High Latency

Check metrics:
- Resolution depth (complex policies?)
- Datastore queries (missing cache?)
- Cache hit rate (TTL too low?)

### Memory Usage

Monitor cache capacity:
```sql
SHOW authz.cache_max_capacity;
```

Reduce if memory usage is high.

## See Also

- [Configuration Guide](configuration.md) - Tuning parameters
- [Performance Guide](performance.md) - Optimization strategies
- [Debugging Guide](debugging.md) - Troubleshooting tips
