//! Unified cache module for pgauthz — L1 (model), L2 (result), L3 (tuple).
//!
//! All caches are process-wide singletons backed by `moka`.  When the
//! corresponding GUC TTL is 0 the accessor returns a `NoopCache` so that
//! entries are never stored (safe default for tests).

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use authz_core::cache::{AuthzCache, CacheMetrics, noop_cache};
use authz_core::error::AuthzError;
use authz_core::model_parser::parse_dsl;
use authz_core::resolver::CheckResult;
use authz_core::traits::{ModelReader, RevisionReader, Tuple};
use authz_core::type_system::TypeSystem;
use authz_datastore_pgx::PostgresDatastore;
use moka::sync::Cache;

use crate::guc;

// ---------------------------------------------------------------------------
// Revision quantization helper
// ---------------------------------------------------------------------------

/// Quantize a revision ULID to a time boundary for cache key sharing.
///
/// When `quantum_secs` is 0, returns the revision unchanged (no quantization).
/// Otherwise, extracts the timestamp from the ULID and rounds it down to the
/// nearest quantum boundary, returning a string like "q1234567890000" where
/// the number is the quantized timestamp in milliseconds.
pub(crate) fn quantize_revision(revision_ulid: &str, quantum_secs: u64) -> String {
    if quantum_secs == 0 || revision_ulid == "0" {
        return revision_ulid.to_string();
    }

    // Try to parse as ULID
    if let Ok(ulid) = ulid::Ulid::from_string(revision_ulid) {
        let timestamp_ms = ulid.timestamp_ms();
        let quantum_ms = quantum_secs * 1000;
        let quantized_ms = (timestamp_ms / quantum_ms) * quantum_ms;
        format!("q{}", quantized_ms)
    } else {
        // If not a valid ULID, return as-is (e.g., "0" for bootstrap)
        revision_ulid.to_string()
    }
}

// ---------------------------------------------------------------------------
// MokaCache — AuthzCache implementation backed by moka::sync::Cache
// ---------------------------------------------------------------------------

/// A `moka`-backed implementation of `AuthzCache`.
pub struct MokaCache<V: Clone + Send + Sync + 'static> {
    inner: Cache<String, V>,
}

impl<V: Clone + Send + Sync + 'static> MokaCache<V> {
    pub fn new(max_capacity: u64, ttl_secs: u64) -> Self {
        Self {
            inner: Cache::builder()
                .max_capacity(max_capacity)
                .time_to_live(Duration::from_secs(ttl_secs))
                .build(),
        }
    }
}

/// Metrics wrapper for moka cache.
struct MokaMetrics {
    hits: u64,
    misses: u64,
}

impl CacheMetrics for MokaMetrics {
    fn hits(&self) -> u64 {
        self.hits
    }

    fn misses(&self) -> u64 {
        self.misses
    }
}

impl<V: Clone + Send + Sync + 'static> AuthzCache<V> for MokaCache<V> {
    fn get(&self, key: &str) -> Option<V> {
        self.inner.get(&key.to_string())
    }

    fn insert(&self, key: &str, value: V) {
        self.inner.insert(key.to_string(), value);
    }

    fn invalidate(&self, key: &str) {
        self.inner.invalidate(&key.to_string());
    }

    fn invalidate_all(&self) {
        self.inner.invalidate_all();
    }

    fn metrics(&self) -> Box<dyn CacheMetrics> {
        // Moka provides hit_count() and miss_count() via entry_count() API
        let entry_count = self.inner.entry_count();
        let weighted_size = self.inner.weighted_size();

        // Moka doesn't expose hit/miss counters directly in the sync API,
        // so we return the entry count as a proxy metric
        Box::new(MokaMetrics {
            hits: entry_count,
            misses: weighted_size.saturating_sub(entry_count),
        })
    }
}

// ---------------------------------------------------------------------------
// L1 — Model cache (TypeSystem)
// ---------------------------------------------------------------------------

static MODEL_CACHE: OnceLock<Cache<String, Arc<TypeSystem>>> = OnceLock::new();

fn model_cache() -> &'static Cache<String, Arc<TypeSystem>> {
    MODEL_CACHE.get_or_init(|| {
        let ttl = guc::get_model_cache_ttl();
        let cap = guc::get_cache_max_capacity();
        let mut builder = Cache::builder().max_capacity(cap);
        if ttl > 0 {
            builder = builder.time_to_live(Duration::from_secs(ttl));
        }
        builder.build()
    })
}

/// Load the latest `TypeSystem`, using the L1 model cache when TTL > 0.
/// Cache keys include revision so stale entries are never served.
pub(crate) fn load_typesystem_cached(
    ds: &PostgresDatastore,
) -> Result<Arc<TypeSystem>, AuthzError> {
    let _span = tracing::info_span!("load_typesystem_cached").entered();
    let ttl = guc::get_model_cache_ttl();

    // When caching is disabled, always load fresh
    if ttl == 0 {
        tracing::debug!("L1 model cache disabled (TTL=0)");
        return load_model_from_db(ds);
    }

    // Read and quantize revision
    let raw_revision =
        pollster::block_on(ds.read_latest_revision()).unwrap_or_else(|_| "0".to_string());
    let quantum_secs = guc::get_revision_quantization_secs();
    let revision = quantize_revision(&raw_revision, quantum_secs);

    // Key by revision:latest (use "latest" as model_id since we only have one global model)
    let key = format!("{}:latest", revision);

    if let Some(ts) = model_cache().get(&key) {
        tracing::debug!("L1 model cache hit");
        crate::metrics::record_cache_hit("L1");
        return Ok(ts);
    }

    tracing::debug!("L1 model cache miss — loading from DB");
    crate::metrics::record_cache_miss("L1");
    let ts = load_model_from_db(ds)?;
    model_cache().insert(key, ts.clone());
    Ok(ts)
}

/// Helper to load model from DB without caching.
fn load_model_from_db(ds: &PostgresDatastore) -> Result<Arc<TypeSystem>, AuthzError> {
    let _span = tracing::info_span!("load_model_from_db").entered();
    let start = std::time::Instant::now();

    let model = match pollster::block_on(ds.read_latest_authorization_policy())? {
        Some(m) => m,
        None => return Err(AuthzError::ModelNotFound),
    };

    let parsed =
        parse_dsl(&model.definition).map_err(|e| AuthzError::ModelParse(format!("{}", e)))?;

    let ts = Arc::new(TypeSystem::new(parsed));

    // Record model load duration
    let duration = start.elapsed().as_secs_f64();
    crate::metrics::record_model_load(duration, false);

    Ok(ts)
}

// ---------------------------------------------------------------------------
// L2 — Dispatch result cache
// ---------------------------------------------------------------------------

static RESULT_CACHE: OnceLock<Arc<dyn AuthzCache<CheckResult>>> = OnceLock::new();

/// Get the process-wide L2 result cache.
///
/// Returns `NoopCache` when `authz.result_cache_ttl_secs = 0`.
pub(crate) fn get_result_cache() -> Arc<dyn AuthzCache<CheckResult>> {
    RESULT_CACHE
        .get_or_init(|| {
            let ttl = guc::get_result_cache_ttl();
            if ttl == 0 {
                return noop_cache();
            }
            let cap = guc::get_cache_max_capacity();
            Arc::new(MokaCache::<CheckResult>::new(cap, ttl))
        })
        .clone()
}

// ---------------------------------------------------------------------------
// L3 — Tuple iterator cache
// ---------------------------------------------------------------------------

static TUPLE_CACHE: OnceLock<Arc<dyn AuthzCache<Vec<Tuple>>>> = OnceLock::new();

/// Get the process-wide L3 tuple cache.
///
/// Returns `NoopCache` when `authz.tuple_cache_ttl_secs = 0`.
pub(crate) fn get_tuple_cache() -> Arc<dyn AuthzCache<Vec<Tuple>>> {
    TUPLE_CACHE
        .get_or_init(|| {
            let ttl = guc::get_tuple_cache_ttl();
            if ttl == 0 {
                return noop_cache();
            }
            let cap = guc::get_cache_max_capacity();
            Arc::new(MokaCache::<Vec<Tuple>>::new(cap, ttl))
        })
        .clone()
}
