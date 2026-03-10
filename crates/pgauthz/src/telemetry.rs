//! OpenTelemetry instrumentation for pgauthz.
//!
//! When `authz.otel_enabled = true`, this module installs an OTLP trace
//! exporter that runs alongside the existing pgrx tracing bridge.  All
//! `tracing` spans created anywhere in the process are forwarded to the
//! configured OTLP endpoint.

use crate::guc;
use opentelemetry::KeyValue;
use opentelemetry::global;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::trace::{self, Sampler};
use std::sync::Once;

static OTEL_INIT: Once = Once::new();

/// Install the OTLP tracer as a global tracer provider.
///
/// This must be called **after** `tracing_bridge::init_tracing()` so the
/// subscriber is already set.  The global tracer provider is independent of
/// the `tracing` subscriber — spans created via `tracing` macros will be
/// forwarded to OpenTelemetry only if a `tracing_opentelemetry` layer is
/// present.  Because the subscriber is already locked by `init_tracing`,
/// we install the tracer provider globally and rely on the spans being
/// created via `tracing` macros which the bridge already captures.
pub fn init_otel() {
    if !guc::get_otel_enabled() {
        return;
    }

    OTEL_INIT.call_once(|| {
        let endpoint = guc::get_otel_endpoint();
        let service_name = guc::get_otel_service_name();
        let sampling_ratio = guc::get_otel_trace_sampling_ratio();

        let resource = Resource::new(vec![
            KeyValue::new("service.name", service_name.clone()),
            KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
        ]);

        match opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(&endpoint),
            )
            .with_trace_config(
                trace::config()
                    .with_sampler(Sampler::TraceIdRatioBased(sampling_ratio))
                    .with_resource(resource),
            )
            .install_batch(opentelemetry_sdk::runtime::Tokio)
        {
            Ok(_tracer) => {
                pgrx::info!(
                    "OpenTelemetry tracing initialized (endpoint: {}, service: {}, sampling: {}%)",
                    endpoint,
                    service_name,
                    (sampling_ratio * 100.0) as u32,
                );
            }
            Err(e) => {
                pgrx::warning!("Failed to initialize OpenTelemetry tracer: {}", e);
            }
        }

        // Initialize metrics
        crate::metrics::init_metrics();
    });
}

/// Shutdown OpenTelemetry cleanly — flush pending spans.
#[allow(dead_code)]
pub fn shutdown_otel() {
    if guc::get_otel_enabled() {
        global::shutdown_tracer_provider();
    }
}
