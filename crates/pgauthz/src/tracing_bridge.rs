//! Tracing bridge for pgauthz - forwards tracing events to pgrx logging

use crate::guc;
use std::sync::{Mutex, Once};
use tracing::Subscriber;
use tracing_subscriber::{
    EnvFilter, Layer, Registry, layer::SubscriberExt, util::SubscriberInitExt,
};

static INIT: Once = Once::new();
static CURRENT_FILTER: Mutex<Option<EnvFilter>> = Mutex::new(None);

/// Initialize tracing to forward to pgrx logging
pub fn init_tracing() {
    INIT.call_once(|| {
        // Create a custom layer that forwards to pgrx
        let pgrx_layer = PgxTracingLayer::new();

        // Set up env filter from GUC or RUST_LOG or default to info
        let env_filter = create_env_filter();

        // Store the filter for potential updates
        if let Ok(mut filter_guard) = CURRENT_FILTER.lock() {
            *filter_guard = Some(env_filter.clone());
        }

        // Initialize subscriber with pgrx layer
        Registry::default().with(env_filter).with(pgrx_layer).init();

        pgrx::info!("Tracing initialized for authz-core -> pgauthz");
    });
}

/// Update tracing level from GUC (can be called at runtime)
#[allow(dead_code)]
pub fn update_tracing_level() {
    if let Ok(mut filter_guard) = CURRENT_FILTER.lock() {
        let new_filter = create_env_filter();
        *filter_guard = Some(new_filter.clone());

        // Note: tracing-subscriber doesn't support runtime filter updates easily
        // For now, this requires a reload. In a future version, we could
        // implement a custom filter that reads from GUC on each event.
        pgrx::info!("Tracing level updated (requires reload to take effect)");
    }
}

/// Create env filter from GUC or environment
fn create_env_filter() -> EnvFilter {
    // Try RUST_LOG first, then fall back to GUC
    EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        let level = guc::get_tracing_level();
        EnvFilter::new(format!("authz_core={},pgauthz={}", level, level))
    })
}

/// Custom tracing layer that forwards events to pgrx logging
struct PgxTracingLayer;

impl PgxTracingLayer {
    fn new() -> Self {
        Self
    }
}

impl<S> Layer<S> for PgxTracingLayer
where
    S: Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // Extract metadata and fields
        let metadata = event.metadata();
        let level = metadata.level();
        let target = metadata.target();
        let module_path = metadata.module_path().unwrap_or("unknown");

        // Extract the message if present
        let mut message = String::new();
        let mut visitor = MessageVisitor(&mut message);
        event.record(&mut visitor);

        // Extract additional fields for structured logging and metrics
        let mut fields = String::new();
        let mut cache_level = None;
        let mut field_visitor = FieldVisitor(&mut fields, &mut cache_level);
        event.record(&mut field_visitor);

        // Record cache hit metrics if this is a cache_hit event
        if message == "cache_hit"
            && let Some(level) = cache_level
        {
            crate::metrics::record_cache_hit(level.as_str());
        }

        // Format the log message
        let formatted = if !message.is_empty() && !fields.is_empty() {
            format!("[{}] {}: {} {}", target, module_path, message, fields)
        } else if !message.is_empty() {
            format!("[{}] {}: {}", target, module_path, message)
        } else if !fields.is_empty() {
            format!("[{}] {} {}", target, module_path, fields)
        } else {
            format!("[{}] {}", target, module_path)
        };

        // Forward to appropriate pgrx log level
        match *level {
            tracing::Level::ERROR => pgrx::error!("{}", formatted),
            tracing::Level::WARN => pgrx::warning!("{}", formatted),
            tracing::Level::INFO => pgrx::info!("{}", formatted),
            tracing::Level::DEBUG => pgrx::notice!("{}", formatted), // Use NOTICE for DEBUG
            tracing::Level::TRACE => pgrx::notice!("{}", formatted), // Use NOTICE for TRACE
        }
    }
}

/// Visitor to extract the message field from tracing events
struct MessageVisitor<'a>(&'a mut String);

impl<'a> tracing::field::Visit for MessageVisitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            *self.0 = format!("{:?}", value);
        }
    }
}

/// Visitor to extract additional fields from tracing events
struct FieldVisitor<'a>(&'a mut String, &'a mut Option<String>);

impl<'a> tracing::field::Visit for FieldVisitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "cache_level" {
            *self.1 = Some(format!("{:?}", value));
        } else if field.name() != "message" {
            if self.0.is_empty() {
                *self.0 = format!("{}={:?}", field.name(), value);
            } else {
                *self.0 = format!("{}, {}={:?}", self.0, field.name(), value);
            }
        }
    }
}
