//! Test helpers - exposes cache internals for integration testing
//!
//! Only compiled under the `noop` feature (test builds).
//! Allows integration tests to exercise the real REGISTRY_CACHE static,
//! get_registry(), and invalidate_registry_cache() functions.

use codelet_providers::models::ModelRegistry;
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
