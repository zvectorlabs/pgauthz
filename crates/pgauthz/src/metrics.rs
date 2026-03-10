//! OpenTelemetry metrics for pgauthz.
//!
//! This module provides metric instrumentation using OpenTelemetry.
//! Metrics are disabled by default and can be enabled via GUC parameters.

use opentelemetry::{
    KeyValue,
    metrics::{Counter, Histogram, Meter},
};
use std::sync::OnceLock;

use crate::guc;

/// Global metrics instance
static METRICS: OnceLock<PgAuthzMetrics> = OnceLock::new();

/// Container for all pgauthz metrics
pub struct PgAuthzMetrics {
    // Request metrics
    pub check_duration: Histogram<f64>,
    pub check_total: Counter<u64>,
    pub write_tuples_total: Counter<u64>,
    pub read_tuples_total: Counter<u64>,
    pub errors_total: Counter<u64>,

    // Cache metrics
    pub cache_hits_total: Counter<u64>,
    pub cache_misses_total: Counter<u64>,

    // Resolution metrics
    pub resolution_depth: Histogram<u64>,
    pub dispatch_count: Histogram<u64>,
    pub datastore_queries_per_check: Histogram<u64>,
    pub tuple_read_count: Histogram<u64>,

    // Model metrics
    pub model_load_duration: Histogram<f64>,
}

impl PgAuthzMetrics {
    fn new(meter: Meter) -> Self {
        Self {
            // Request metrics
            check_duration: meter
                .f64_histogram("pgauthz.check.duration")
                .with_description("Duration of authorization checks in seconds")
                .init(),
            check_total: meter
                .u64_counter("pgauthz.check.total")
                .with_description("Total number of authorization checks")
                .init(),
            write_tuples_total: meter
                .u64_counter("pgauthz.write_tuples.total")
                .with_description("Total number of tuple write operations")
                .init(),
            read_tuples_total: meter
                .u64_counter("pgauthz.read_tuples.total")
                .with_description("Total number of tuple read operations")
                .init(),
            errors_total: meter
                .u64_counter("pgauthz.errors.total")
                .with_description("Total number of errors")
                .init(),

            // Cache metrics
            cache_hits_total: meter
                .u64_counter("pgauthz.cache.hits.total")
                .with_description("Total number of cache hits")
                .init(),
            cache_misses_total: meter
                .u64_counter("pgauthz.cache.misses.total")
                .with_description("Total number of cache misses")
                .init(),

            // Resolution metrics
            resolution_depth: meter
                .u64_histogram("pgauthz.resolution.depth")
                .with_description("Depth of permission tree traversal")
                .init(),
            dispatch_count: meter
                .u64_histogram("pgauthz.resolution.dispatch_count")
                .with_description("Number of sub-dispatches per check")
                .init(),
            datastore_queries_per_check: meter
                .u64_histogram("pgauthz.resolution.datastore_queries")
                .with_description("Number of datastore queries per check")
                .init(),
            tuple_read_count: meter
                .u64_histogram("pgauthz.tuples.read_count")
                .with_description("Number of tuples read per operation")
                .init(),

            // Model metrics
            model_load_duration: meter
                .f64_histogram("pgauthz.model.load_duration")
                .with_description("Duration of model loading in seconds")
                .init(),
        }
    }
}

/// Initialize metrics using the global meter provider
/// This should be called after OpenTelemetry tracing is initialized
pub fn init_metrics() {
    if !guc::get_otel_enabled() {
        return;
    }

    // Get the global meter provider
    let meter = opentelemetry::global::meter("pgauthz");
    let metrics = PgAuthzMetrics::new(meter);

    if METRICS.set(metrics).is_err() {
        pgrx::warning!("Metrics already initialized");
    } else {
        pgrx::info!("OpenTelemetry metrics initialized");
    }
}

/// Get the global metrics instance
pub fn metrics() -> Option<&'static PgAuthzMetrics> {
    METRICS.get()
}

/// Record a check operation
pub fn record_check(duration_secs: f64, result: &str, object_type: &str, relation: &str) {
    if let Some(m) = metrics() {
        let attrs = &[
            KeyValue::new("result", result.to_string()),
            KeyValue::new("object_type", object_type.to_string()),
            KeyValue::new("relation", relation.to_string()),
        ];
        m.check_duration.record(duration_secs, attrs);
        m.check_total.add(1, attrs);
    }
}

/// Record a cache hit
pub fn record_cache_hit(cache_level: &str) {
    if let Some(m) = metrics() {
        m.cache_hits_total
            .add(1, &[KeyValue::new("cache_level", cache_level.to_string())]);
    }
}

/// Record a cache miss
pub fn record_cache_miss(cache_level: &str) {
    if let Some(m) = metrics() {
        m.cache_misses_total
            .add(1, &[KeyValue::new("cache_level", cache_level.to_string())]);
    }
}

/// Record an error
pub fn record_error(error_type: &str, operation: &str) {
    if let Some(m) = metrics() {
        m.errors_total.add(
            1,
            &[
                KeyValue::new("error_type", error_type.to_string()),
                KeyValue::new("operation", operation.to_string()),
            ],
        );
    }
}

/// Record tuple write operation
pub fn record_tuple_write(writes_count: u64, deletes_count: u64) {
    if let Some(m) = metrics() {
        m.write_tuples_total
            .add(writes_count, &[KeyValue::new("operation", "write")]);
        m.write_tuples_total
            .add(deletes_count, &[KeyValue::new("operation", "delete")]);
    }
}

/// Record tuple read operation
pub fn record_tuple_read(object_type: &str, count: u64) {
    if let Some(m) = metrics() {
        m.read_tuples_total
            .add(1, &[KeyValue::new("object_type", object_type.to_string())]);
        m.tuple_read_count.record(
            count,
            &[KeyValue::new("object_type", object_type.to_string())],
        );
    }
}

/// Record model load duration
pub fn record_model_load(duration_secs: f64, cache_hit: bool) {
    if let Some(m) = metrics() {
        m.model_load_duration.record(
            duration_secs,
            &[KeyValue::new("cache_hit", cache_hit.to_string())],
        );
    }
}

/// Record resolution metrics
pub fn record_resolution(depth: u64, dispatch_count: u64, datastore_queries: u64) {
    if let Some(m) = metrics() {
        m.resolution_depth.record(depth, &[]);
        m.dispatch_count.record(dispatch_count, &[]);
        m.datastore_queries_per_check.record(datastore_queries, &[]);
    }
}
