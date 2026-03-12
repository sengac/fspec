//! NAPI bindings for model cache and selection functions
//!
//! MODEL-001: Exposes models.dev model listing to TypeScript.
//! MODEL-002: Registry cache uses Mutex<Option<Arc<...>>> for invalidation.
//! MODEL-003: Provider-specific listing applies same filter/sort as list_all;
//!            refresh path tested through real invalidation code; lock not held
//!            across async initialization.
//!
//! Architecture:
//! - Core logic (cache, types, helpers) always compiled
//! - NAPI bindings conditionally compiled (excluded under noop feature)
//! - Test helpers exposed under noop for integration testing
//!
//! NOTE: Cache directory is derived from the global data directory
//! set via persistenceSetDataDirectory(). No separate cache directory
//! configuration is needed.

#[cfg(not(feature = "noop"))]
mod napi_bindings;

#[cfg(feature = "noop")]
pub mod testing;

#[cfg(not(feature = "noop"))]
pub use napi_bindings::*;

use codelet_providers::models::{ModelCache, ModelInfo, ModelRegistry};
#[cfg(feature = "noop")]
use codelet_providers::models::ModelsDevResponse;
use std::sync::Arc;
use tokio::sync::Mutex;

// ============================================================================
// Cached Registry (DRY - avoid repeated JSON parsing)
// ============================================================================

/// Cached model registry - resettable so refresh can invalidate it
static REGISTRY_CACHE: Mutex<Option<Arc<ModelRegistry>>> = Mutex::const_new(None);

/// Get or initialize the cached model registry.
///
/// Uses a two-phase lock pattern to avoid holding the Mutex guard across
/// async initialization (MODEL-003 Rule [3]):
///
///   Phase 1 — check under lock (fast path):
///     Acquire lock → if cached, clone Arc and return → drop lock.
///
///   Phase 2 — initialize outside the lock (slow path):
///     Build registry asynchronously → re-acquire lock → double-check →
///     store Arc if no concurrent initializer beat us.
///
/// This keeps concurrent readers unblocked during the cold-start IO.
pub(crate) async fn get_registry() -> Result<Arc<ModelRegistry>, String> {
    // Fast path: check under lock without holding it across IO
    {
        let guard = REGISTRY_CACHE.lock().await;
        if let Some(ref registry) = *guard {
            return Ok(Arc::clone(registry));
        }
    } // Lock dropped here — safe to do IO without starving readers

    // Slow path: build the registry without holding the lock
    let cache = ModelCache::new()
        .map_err(|e| format!("Failed to initialize model cache: {}", e))?;
    let registry = ModelRegistry::new(&cache)
        .await
        .map_err(|e| format!("Failed to load model registry: {}", e))?;
    let arc = Arc::new(registry);

    // Re-acquire lock and store (double-check: another task may have initialized)
    let mut guard = REGISTRY_CACHE.lock().await;
    if let Some(ref existing) = *guard {
        // Concurrent initializer won; prefer the already-stored Arc
        return Ok(Arc::clone(existing));
    }
    *guard = Some(Arc::clone(&arc));
    Ok(arc)
}

/// Invalidate the in-memory registry cache so the next call to
/// get_registry() rebuilds it from the (refreshed) disk cache.
pub(crate) async fn invalidate_registry_cache() {
    let mut guard = REGISTRY_CACHE.lock().await;
    *guard = None;
}

// ============================================================================
// Public Refresh Path (MODEL-003 Rule [2]) — test builds only
// ============================================================================

/// Write `data` to the disk cache and invalidate the in-memory registry.
///
/// Only compiled under the `noop` feature (test builds). Mirrors the
/// core action of `models_refresh_cache()` (without the HTTP fetch) so
/// integration tests can exercise the EXACT invalidation code path.
/// Removing `invalidate_registry_cache()` from this function will break
/// the provider-listing-refresh test.
#[cfg(feature = "noop")]
pub(crate) async fn refresh_registry_from_data(data: &ModelsDevResponse) -> Result<(), String> {
    use tokio::fs;

    let cache = ModelCache::new()
        .map_err(|e| format!("Failed to initialize model cache: {}", e))?;

    let json = serde_json::to_string(data)
        .map_err(|e| format!("Failed to serialize model data: {}", e))?;

    // Ensure cache directory exists
    if let Some(parent) = cache.cache_path().parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create cache dir: {}", e))?;
    }

    // Write to disk (mirrors what ModelCache::fetch_and_cache does)
    fs::write(cache.cache_path(), json)
        .await
        .map_err(|e| format!("Failed to write cache: {}", e))?;

    // Invalidate in-memory registry (THE CRITICAL STEP — must not be removed)
    invalidate_registry_cache().await;

    Ok(())
}

// ============================================================================
// Model Filtering Helpers
// ============================================================================

use chrono::Datelike;

/// Check if a model should be shown in the UI (filters out deprecated/old models).
///
/// Always compiled (both noop and non-noop) so the filter logic can be tested
/// independently of the NAPI layer.
pub(crate) fn is_current_model(model: &ModelInfo) -> bool {
    use codelet_providers::models::ModelStatus;

    // Filter out deprecated models
    if model.status == Some(ModelStatus::Deprecated) {
        return false;
    }

    // Filter out invalid aliases (models without dated versions)
    // For Anthropic models, only show dated versions like "claude-opus-4-5-20251101"
    // NOT aliases like "claude-opus-4-5" which are not valid API model IDs
    // EXCEPTION: If the model has a release_date, it's a real model (e.g., claude-opus-4-6)
    if model.id.starts_with("claude-") {
        // Check if it ends with a date pattern (8 digits: YYYYMMDD)
        let has_date_suffix = model.id.chars().rev().take(8).all(|c| c.is_ascii_digit())
            && model.id.chars().rev().nth(8) == Some('-');

        // Only filter out if no date suffix AND no release_date
        // Models with release_date are real models, not just aliases
        if !has_date_suffix && model.release_date.is_none() {
            return false;
        }
    }

    // Filter out models older than 18 months
    if let Some(ref release_date) = model.release_date {
        if let Ok(date) = chrono::NaiveDate::parse_from_str(release_date, "%Y-%m-%d") {
            let today = chrono::Utc::now().date_naive();
            let age_months =
                (today.year() - date.year()) * 12 + (today.month() as i32 - date.month() as i32);
            if age_months > 18 {
                return false;
            }
        }
    }

    true
}
