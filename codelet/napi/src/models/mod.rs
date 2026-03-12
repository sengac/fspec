//! NAPI bindings for model cache and selection functions
//!
//! MODEL-001: Exposes models.dev model listing to TypeScript.
//! MODEL-002: Registry cache uses Mutex<Option<Arc<...>>> for invalidation.
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

use codelet_providers::models::{ModelCache, ModelRegistry};
use std::sync::Arc;
use tokio::sync::Mutex;

// ============================================================================
// Cached Registry (DRY - avoid repeated JSON parsing)
// ============================================================================

/// Cached model registry - resettable so refresh can invalidate it
static REGISTRY_CACHE: Mutex<Option<Arc<ModelRegistry>>> = Mutex::const_new(None);

/// Get or initialize the cached model registry
pub(crate) async fn get_registry() -> Result<Arc<ModelRegistry>, String> {
    let mut guard = REGISTRY_CACHE.lock().await;
    if let Some(ref registry) = *guard {
        return Ok(Arc::clone(registry));
    }
    let cache = ModelCache::new()
        .map_err(|e| format!("Failed to initialize model cache: {}", e))?;
    let registry = ModelRegistry::new(&cache)
        .await
        .map_err(|e| format!("Failed to load model registry: {}", e))?;
    let arc = Arc::new(registry);
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
// Model Filtering Helpers
// ============================================================================

#[cfg(not(feature = "noop"))]
use chrono::Datelike;
#[cfg(not(feature = "noop"))]
use codelet_providers::models::ModelInfo;

/// Check if a model should be shown in the UI (filters out deprecated/old models)
#[cfg(not(feature = "noop"))]
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
