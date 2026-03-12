//! Test helpers - exposes cache internals for integration testing
//!
//! Only compiled under the `noop` feature (test builds).
//! Allows integration tests to exercise the real REGISTRY_CACHE static,
//! get_registry(), invalidate_registry_cache(), and refresh_registry_from_data()
//! functions.

use codelet_providers::models::{ModelInfo, ModelRegistry, ModelsDevResponse};
use std::sync::Arc;

/// Get or initialize the cached model registry.
///
/// Delegates to the actual production get_registry() that uses
/// the process-global REGISTRY_CACHE static.
pub async fn get_registry() -> Result<Arc<ModelRegistry>, String> {
    super::get_registry().await
}

/// Invalidate the in-memory registry cache.
///
/// Delegates to the actual production invalidate_registry_cache()
/// that clears the process-global REGISTRY_CACHE static.
pub async fn invalidate_registry_cache() {
    super::invalidate_registry_cache().await
}

/// Write `data` to the disk cache and invalidate the in-memory registry.
///
/// Exercises the SAME invalidation code path as `models_refresh_cache()`
/// (without making an HTTP request). If `invalidate_registry_cache()` is
/// removed from the underlying implementation, tests using this helper fail.
pub async fn refresh_registry_from_data(data: &ModelsDevResponse) -> Result<(), String> {
    super::refresh_registry_from_data(data).await
}

/// List models for a specific provider, applying is_current_model() filter
/// and newest-first sort (mirrors the fixed models_list_for_provider NAPI fn).
pub async fn list_models_for_provider(provider_id: &str) -> Result<Vec<ModelInfo>, String> {
    let registry = super::get_registry().await?;

    let models = registry
        .list_models(provider_id)
        .map_err(|e| format!("Failed to list models for '{}': {}", provider_id, e))?;

    let mut filtered: Vec<ModelInfo> = models
        .into_iter()
        .filter(|m| super::is_current_model(m))
        .cloned()
        .collect();

    // Sort by release date descending (newest first)
    filtered.sort_by(|a, b| {
        let date_a = a.release_date.as_deref().unwrap_or("1970-01-01");
        let date_b = b.release_date.as_deref().unwrap_or("1970-01-01");
        date_b.cmp(date_a)
    });

    Ok(filtered)
}

/// Check if a model is current (not deprecated, not older than 18 months).
///
/// Delegates to the same helper used by models_list_all() and
/// models_list_for_provider().
pub fn is_current_model(model: &ModelInfo) -> bool {
    super::is_current_model(model)
}
