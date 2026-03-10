//! Extension config via Postgres GUCs.

use std::ffi::{CStr, CString};

use pgrx::guc::{GucContext, GucFlags, GucRegistry, GucSetting};

// Static C string literals for GUC registration.
const CHECK_STRATEGY_NAME: &[u8] = b"authz.check_strategy\0";
const STRATEGY_SHORT: &[u8] = b"check strategy\0";
const STRATEGY_LONG: &[u8] = b"Check optimization strategy: batch (default) or parallel\0";

const TRACING_LEVEL_NAME: &[u8] = b"authz.tracing_level\0";
const TRACING_SHORT: &[u8] = b"tracing level\0";
const TRACING_LONG: &[u8] =
    b"Tracing level for authz-core logs: error, warn, info, debug, trace (default: info)\0";

const MODEL_CACHE_TTL_NAME: &[u8] = b"authz.model_cache_ttl_secs\0";
const MODEL_CACHE_TTL_SHORT: &[u8] = b"L1 model cache TTL\0";
const MODEL_CACHE_TTL_LONG: &[u8] = b"L1 model cache TTL in seconds (0 = disabled, default: 0)\0";

const RESULT_CACHE_TTL_NAME: &[u8] = b"authz.result_cache_ttl_secs\0";
const RESULT_CACHE_TTL_SHORT: &[u8] = b"L2 result cache TTL\0";
const RESULT_CACHE_TTL_LONG: &[u8] =
    b"L2 dispatch result cache TTL in seconds (0 = disabled, default: 0)\0";

const TUPLE_CACHE_TTL_NAME: &[u8] = b"authz.tuple_cache_ttl_secs\0";
const TUPLE_CACHE_TTL_SHORT: &[u8] = b"L3 tuple cache TTL\0";
const TUPLE_CACHE_TTL_LONG: &[u8] =
    b"L3 tuple iterator cache TTL in seconds (0 = disabled, default: 0)\0";

const CACHE_MAX_CAPACITY_NAME: &[u8] = b"authz.cache_max_capacity\0";
const CACHE_MAX_CAPACITY_SHORT: &[u8] = b"cache max capacity\0";
const CACHE_MAX_CAPACITY_LONG: &[u8] = b"Maximum entries per cache layer (default: 10000)\0";

const REVISION_QUANTIZATION_NAME: &[u8] = b"authz.revision_quantization_secs\0";
const REVISION_QUANTIZATION_SHORT: &[u8] = b"revision quantization\0";
const REVISION_QUANTIZATION_LONG: &[u8] =
    b"Revision quantization interval in seconds (0 = disabled, default: 5)\0";

// OpenTelemetry GUCs
const OTEL_ENABLED_NAME: &[u8] = b"authz.otel_enabled\0";
const OTEL_ENABLED_SHORT: &[u8] = b"OpenTelemetry enabled\0";
const OTEL_ENABLED_LONG: &[u8] = b"Enable OpenTelemetry tracing and metrics (default: false)\0";

const OTEL_ENDPOINT_NAME: &[u8] = b"authz.otel_endpoint\0";
const OTEL_ENDPOINT_SHORT: &[u8] = b"OpenTelemetry endpoint\0";
const OTEL_ENDPOINT_LONG: &[u8] = b"OpenTelemetry OTLP endpoint (default: http://localhost:4317)\0";

const OTEL_SERVICE_NAME_NAME: &[u8] = b"authz.otel_service_name\0";
const OTEL_SERVICE_NAME_SHORT: &[u8] = b"OpenTelemetry service name\0";
const OTEL_SERVICE_NAME_LONG: &[u8] = b"OpenTelemetry service name (default: pgauthz)\0";

const OTEL_TRACE_SAMPLING_RATIO_NAME: &[u8] = b"authz.otel_trace_sampling_ratio\0";
const OTEL_TRACE_SAMPLING_RATIO_SHORT: &[u8] = b"OpenTelemetry trace sampling ratio\0";
const OTEL_TRACE_SAMPLING_RATIO_LONG: &[u8] =
    b"OpenTelemetry trace sampling ratio (0-100 percent, default: 100)\0";

static CHECK_STRATEGY: GucSetting<Option<CString>> = GucSetting::<Option<CString>>::new(None);
static TRACING_LEVEL: GucSetting<Option<CString>> = GucSetting::<Option<CString>>::new(None);
static MODEL_CACHE_TTL: GucSetting<i32> = GucSetting::<i32>::new(0);
static RESULT_CACHE_TTL: GucSetting<i32> = GucSetting::<i32>::new(0);
static TUPLE_CACHE_TTL: GucSetting<i32> = GucSetting::<i32>::new(0);
static CACHE_MAX_CAPACITY: GucSetting<i32> = GucSetting::<i32>::new(10_000);
static REVISION_QUANTIZATION: GucSetting<i32> = GucSetting::<i32>::new(5);

static OTEL_ENABLED: GucSetting<bool> = GucSetting::<bool>::new(false);
static OTEL_ENDPOINT: GucSetting<Option<CString>> = GucSetting::<Option<CString>>::new(None);
static OTEL_SERVICE_NAME: GucSetting<Option<CString>> = GucSetting::<Option<CString>>::new(None);
static OTEL_TRACE_SAMPLING_RATIO: GucSetting<i32> = GucSetting::<i32>::new(100);

/// Register GUCs. Call from _PG_init.
pub fn register_gucs() {
    let strategy_name = unsafe { CStr::from_bytes_with_nul_unchecked(CHECK_STRATEGY_NAME) };
    let strategy_short = unsafe { CStr::from_bytes_with_nul_unchecked(STRATEGY_SHORT) };
    let strategy_long = unsafe { CStr::from_bytes_with_nul_unchecked(STRATEGY_LONG) };
    GucRegistry::define_string_guc(
        strategy_name,
        strategy_short,
        strategy_long,
        &CHECK_STRATEGY,
        GucContext::Userset,
        GucFlags::default(),
    );

    let tracing_name = unsafe { CStr::from_bytes_with_nul_unchecked(TRACING_LEVEL_NAME) };
    let tracing_short = unsafe { CStr::from_bytes_with_nul_unchecked(TRACING_SHORT) };
    let tracing_long = unsafe { CStr::from_bytes_with_nul_unchecked(TRACING_LONG) };
    GucRegistry::define_string_guc(
        tracing_name,
        tracing_short,
        tracing_long,
        &TRACING_LEVEL,
        GucContext::Userset,
        GucFlags::default(),
    );

    // Cache GUCs
    GucRegistry::define_int_guc(
        unsafe { CStr::from_bytes_with_nul_unchecked(MODEL_CACHE_TTL_NAME) },
        unsafe { CStr::from_bytes_with_nul_unchecked(MODEL_CACHE_TTL_SHORT) },
        unsafe { CStr::from_bytes_with_nul_unchecked(MODEL_CACHE_TTL_LONG) },
        &MODEL_CACHE_TTL,
        0,
        i32::MAX,
        GucContext::Userset,
        GucFlags::default(),
    );
    GucRegistry::define_int_guc(
        unsafe { CStr::from_bytes_with_nul_unchecked(RESULT_CACHE_TTL_NAME) },
        unsafe { CStr::from_bytes_with_nul_unchecked(RESULT_CACHE_TTL_SHORT) },
        unsafe { CStr::from_bytes_with_nul_unchecked(RESULT_CACHE_TTL_LONG) },
        &RESULT_CACHE_TTL,
        0,
        i32::MAX,
        GucContext::Userset,
        GucFlags::default(),
    );
    GucRegistry::define_int_guc(
        unsafe { CStr::from_bytes_with_nul_unchecked(TUPLE_CACHE_TTL_NAME) },
        unsafe { CStr::from_bytes_with_nul_unchecked(TUPLE_CACHE_TTL_SHORT) },
        unsafe { CStr::from_bytes_with_nul_unchecked(TUPLE_CACHE_TTL_LONG) },
        &TUPLE_CACHE_TTL,
        0,
        i32::MAX,
        GucContext::Userset,
        GucFlags::default(),
    );
    GucRegistry::define_int_guc(
        unsafe { CStr::from_bytes_with_nul_unchecked(CACHE_MAX_CAPACITY_NAME) },
        unsafe { CStr::from_bytes_with_nul_unchecked(CACHE_MAX_CAPACITY_SHORT) },
        unsafe { CStr::from_bytes_with_nul_unchecked(CACHE_MAX_CAPACITY_LONG) },
        &CACHE_MAX_CAPACITY,
        1,
        i32::MAX,
        GucContext::Userset,
        GucFlags::default(),
    );
    GucRegistry::define_int_guc(
        unsafe { CStr::from_bytes_with_nul_unchecked(REVISION_QUANTIZATION_NAME) },
        unsafe { CStr::from_bytes_with_nul_unchecked(REVISION_QUANTIZATION_SHORT) },
        unsafe { CStr::from_bytes_with_nul_unchecked(REVISION_QUANTIZATION_LONG) },
        &REVISION_QUANTIZATION,
        0,
        3600,
        GucContext::Userset,
        GucFlags::default(),
    );

    // OpenTelemetry GUCs
    GucRegistry::define_bool_guc(
        unsafe { CStr::from_bytes_with_nul_unchecked(OTEL_ENABLED_NAME) },
        unsafe { CStr::from_bytes_with_nul_unchecked(OTEL_ENABLED_SHORT) },
        unsafe { CStr::from_bytes_with_nul_unchecked(OTEL_ENABLED_LONG) },
        &OTEL_ENABLED,
        GucContext::Userset,
        GucFlags::default(),
    );

    GucRegistry::define_string_guc(
        unsafe { CStr::from_bytes_with_nul_unchecked(OTEL_ENDPOINT_NAME) },
        unsafe { CStr::from_bytes_with_nul_unchecked(OTEL_ENDPOINT_SHORT) },
        unsafe { CStr::from_bytes_with_nul_unchecked(OTEL_ENDPOINT_LONG) },
        &OTEL_ENDPOINT,
        GucContext::Userset,
        GucFlags::default(),
    );

    GucRegistry::define_string_guc(
        unsafe { CStr::from_bytes_with_nul_unchecked(OTEL_SERVICE_NAME_NAME) },
        unsafe { CStr::from_bytes_with_nul_unchecked(OTEL_SERVICE_NAME_SHORT) },
        unsafe { CStr::from_bytes_with_nul_unchecked(OTEL_SERVICE_NAME_LONG) },
        &OTEL_SERVICE_NAME,
        GucContext::Userset,
        GucFlags::default(),
    );

    GucRegistry::define_int_guc(
        unsafe { CStr::from_bytes_with_nul_unchecked(OTEL_TRACE_SAMPLING_RATIO_NAME) },
        unsafe { CStr::from_bytes_with_nul_unchecked(OTEL_TRACE_SAMPLING_RATIO_SHORT) },
        unsafe { CStr::from_bytes_with_nul_unchecked(OTEL_TRACE_SAMPLING_RATIO_LONG) },
        &OTEL_TRACE_SAMPLING_RATIO,
        0,
        100,
        GucContext::Userset,
        GucFlags::default(),
    );
}

/// Get authz.otel_enabled from GUC
pub fn get_otel_enabled() -> bool {
    OTEL_ENABLED.get()
}

/// Get authz.otel_endpoint from GUC, defaults to "http://localhost:4317"
pub fn get_otel_endpoint() -> String {
    OTEL_ENDPOINT
        .get()
        .map(|s: CString| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "http://localhost:4317".to_string())
}

/// Get authz.otel_service_name from GUC, defaults to "pgauthz"
pub fn get_otel_service_name() -> String {
    OTEL_SERVICE_NAME
        .get()
        .map(|s: CString| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "pgauthz".to_string())
}

/// Get authz.otel_trace_sampling_ratio from GUC (converts percentage to 0.0-1.0)
pub fn get_otel_trace_sampling_ratio() -> f64 {
    let percentage = OTEL_TRACE_SAMPLING_RATIO.get().max(0).min(100);
    percentage as f64 / 100.0
}
/// Check optimization strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CheckStrategy {
    /// Batch queries together (default, best for most cases)
    #[default]
    Batch,
    /// Parallel evaluation (useful for high-latency datastores)
    Parallel,
}

/// Get authz.check_strategy from GUC, defaults to Batch.
pub fn get_check_strategy() -> CheckStrategy {
    CHECK_STRATEGY
        .get()
        .and_then(|s: CString| {
            let strategy = s.to_string_lossy();
            match strategy.to_lowercase().as_str() {
                "parallel" => Some(CheckStrategy::Parallel),
                "batch" => Some(CheckStrategy::Batch),
                _ => None,
            }
        })
        .unwrap_or_default()
}

/// Get authz.model_cache_ttl_secs from GUC (0 = disabled).
pub fn get_model_cache_ttl() -> u64 {
    MODEL_CACHE_TTL.get().max(0) as u64
}

/// Get authz.result_cache_ttl_secs from GUC (0 = disabled).
pub fn get_result_cache_ttl() -> u64 {
    RESULT_CACHE_TTL.get().max(0) as u64
}

/// Get authz.tuple_cache_ttl_secs from GUC (0 = disabled).
pub fn get_tuple_cache_ttl() -> u64 {
    TUPLE_CACHE_TTL.get().max(0) as u64
}

/// Get authz.cache_max_capacity from GUC.
pub fn get_cache_max_capacity() -> u64 {
    CACHE_MAX_CAPACITY.get().max(1) as u64
}

/// Get authz.revision_quantization_secs from GUC (0 = disabled).
pub fn get_revision_quantization_secs() -> u64 {
    REVISION_QUANTIZATION.get().max(0) as u64
}

/// Get authz.tracing_level from GUC, defaults to "info".
pub fn get_tracing_level() -> String {
    TRACING_LEVEL
        .get()
        .map(|s: CString| s.to_string_lossy().to_lowercase())
        .filter(|level| {
            matches!(
                level.as_str(),
                "error" | "warn" | "info" | "debug" | "trace"
            )
        })
        .unwrap_or_else(|| "info".to_string())
}
