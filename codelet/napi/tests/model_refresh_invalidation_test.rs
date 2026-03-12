//! Feature: spec/features/model-refresh-invalidates-in-memory-registry-cache.feature
//!
//! Work Unit: MODEL-002 - Model refresh does not invalidate in-memory registry cache
//!
//! These tests exercise the ACTUAL REGISTRY_CACHE static and the real
//! get_registry() / invalidate_registry_cache() functions via the
//! codelet_napi::models::testing module.
//!
//! Each test:
//! 1. Sets the global data directory to a temp dir
//! 2. Writes a models.json disk cache file
//! 3. Calls the real get_registry() / invalidate_registry_cache()
//! 4. Verifies behavior on the process-global REGISTRY_CACHE
//!
//! Tests are serialized (#[serial]) because they share global state.

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod model_refresh_invalidation {
    use codelet_napi::models::testing;
    use codelet_providers::models::{
        LimitInfo, ModelInfo, ModelsDevResponse, ProviderInfo,
    };
    use serial_test::serial;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tempfile::TempDir;

    // =========================================================================
    // HELPERS
    // =========================================================================

    /// Set up a temp directory as the global data dir and ensure REGISTRY_CACHE
    /// is empty. Returns the TempDir (must be held alive for the test duration).
    async fn setup_test_env() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        codelet_common::set_data_directory(temp_dir.path().to_path_buf())
            .expect("Failed to set data directory");
        // Ensure REGISTRY_CACHE is empty from any prior test
        testing::invalidate_registry_cache().await;
        temp_dir
    }

    /// Build a minimal ModelsDevResponse with one provider and one model.
    fn make_response(model_id: &str, model_name: &str) -> ModelsDevResponse {
        let mut models = HashMap::new();
        models.insert(
            model_id.to_string(),
            ModelInfo {
                id: model_id.to_string(),
                name: model_name.to_string(),
                family: None,
                release_date: Some("2025-06-01".to_string()),
                reasoning: false,
                tool_call: true,
                attachment: false,
                temperature: true,
                interleaved: None,
                modalities: None,
                cost: None,
                limit: LimitInfo {
                    context: 128000,
                    output: 16384,
                },
                status: None,
                experimental: None,
                options: HashMap::new(),
                headers: HashMap::new(),
            },
        );

        let mut providers = HashMap::new();
        providers.insert(
            "test-provider".to_string(),
            ProviderInfo {
                id: "test-provider".to_string(),
                name: "Test Provider".to_string(),
                env: vec![],
                npm: None,
                api: None,
                doc: None,
                models,
            },
        );

        ModelsDevResponse { providers }
    }

    /// Write a ModelsDevResponse to the disk cache at {data_dir}/cache/models.json.
    fn write_disk_cache(temp_dir: &TempDir, response: &ModelsDevResponse) {
        let cache_dir = temp_dir.path().join("cache");
        std::fs::create_dir_all(&cache_dir).unwrap();
        let cache_path = cache_dir.join("models.json");
        let json = serde_json::to_string(response).expect("Failed to serialize response");
        std::fs::write(cache_path, json).expect("Failed to write cache file");
    }

    /// Extract all model IDs from a registry.
    fn get_model_ids(
        registry: &codelet_providers::models::ModelRegistry,
    ) -> Vec<String> {
        registry
            .list_providers()
            .iter()
            .flat_map(|p| p.models.values().map(|m| m.id.clone()))
            .collect()
    }

    // =========================================================================
    // Scenario: Registry returns stale data when OnceCell is not invalidated
    //           after refresh
    //
    // This proves the core caching property: once REGISTRY_CACHE is populated,
    // updating the disk cache alone does NOT change what get_registry() returns.
    // Without invalidation, the in-memory cache is stale.
    // =========================================================================

    #[tokio::test]
    #[serial]
    async fn test_stale_data_when_cache_not_invalidated() {
        // @step Given the REGISTRY_CACHE has been initialized with model data containing "codex-5.3"
        let temp_dir = setup_test_env().await;
        let response_v1 = make_response("codex-5.3", "Codex 5.3");
        write_disk_cache(&temp_dir, &response_v1);

        let registry_v1 = testing::get_registry().await.unwrap();
        let ids_v1 = get_model_ids(&registry_v1);
        assert!(
            ids_v1.contains(&"codex-5.3".to_string()),
            "Initial cache should contain codex-5.3, got: {ids_v1:?}"
        );

        // @step And the disk cache has been refreshed with new data containing "codex-5.4"
        let response_v2 = make_response("codex-5.4", "Codex 5.4");
        write_disk_cache(&temp_dir, &response_v2);

        // @step When models_list_all is called without invalidating the in-memory cache
        let registry_stale = testing::get_registry().await.unwrap();

        // @step Then the returned models still contain "codex-5.3" but not "codex-5.4"
        let ids = get_model_ids(&registry_stale);
        assert!(
            ids.contains(&"codex-5.3".to_string()),
            "Stale cache should still contain codex-5.3, got: {ids:?}"
        );
        assert!(
            !ids.contains(&"codex-5.4".to_string()),
            "Stale cache should NOT contain codex-5.4 (not invalidated), got: {ids:?}"
        );
    }

    // =========================================================================
    // Scenario: Refresh invalidates in-memory cache so new models appear
    //
    // This proves the fix: after disk cache update + invalidate_registry_cache(),
    // the next call to get_registry() rebuilds from the fresh disk data.
    // =========================================================================

    #[tokio::test]
    #[serial]
    async fn test_refresh_invalidates_cache_so_new_models_appear() {
        // @step Given the REGISTRY_CACHE has been initialized with model data containing "codex-5.3"
        let temp_dir = setup_test_env().await;
        let response_v1 = make_response("codex-5.3", "Codex 5.3");
        write_disk_cache(&temp_dir, &response_v1);

        let registry_v1 = testing::get_registry().await.unwrap();
        let ids_v1 = get_model_ids(&registry_v1);
        assert!(
            ids_v1.contains(&"codex-5.3".to_string()),
            "Initial cache should contain codex-5.3"
        );

        // @step When models_refresh_cache is called which fetches fresh data and invalidates the in-memory cache
        // (Simulates what models_refresh_cache() does: write new disk data + invalidate)
        let response_v2 = make_response("codex-5.4", "Codex 5.4");
        write_disk_cache(&temp_dir, &response_v2);
        testing::invalidate_registry_cache().await;

        // @step And models_list_all is called to rebuild the registry
        let registry_fresh = testing::get_registry().await.unwrap();

        // @step Then the returned models contain "codex-5.4"
        let ids = get_model_ids(&registry_fresh);
        assert!(
            ids.contains(&"codex-5.4".to_string()),
            "After invalidation + rebuild, cache should contain codex-5.4, got: {ids:?}"
        );
        assert!(
            !ids.contains(&"codex-5.3".to_string()),
            "After invalidation + rebuild, old codex-5.3 should be gone, got: {ids:?}"
        );
    }

    // =========================================================================
    // Scenario: Registry is lazy-initialized and cached across calls
    //
    // Proves that the Mutex<Option<...>> approach preserves lazy initialization:
    // the registry is only built on the first call, and subsequent calls return
    // the same Arc (pointer-equal).
    // =========================================================================

    #[tokio::test]
    #[serial]
    async fn test_registry_lazy_initialized_and_cached_across_calls() {
        // @step Given the REGISTRY_CACHE is empty (no prior initialization)
        let temp_dir = setup_test_env().await;

        // Write data to disk so the registry can be built on first access
        let response = make_response("test-model", "Test Model");
        write_disk_cache(&temp_dir, &response);

        // @step When get_registry is called for the first time
        let first_arc = testing::get_registry().await.unwrap();

        // @step Then it initializes the registry from the disk cache
        let ids = get_model_ids(&first_arc);
        assert!(
            ids.contains(&"test-model".to_string()),
            "Registry should be initialized from disk cache, got: {ids:?}"
        );

        // @step And a second call returns the same cached Arc without re-parsing
        let second_arc = testing::get_registry().await.unwrap();
        assert!(
            Arc::ptr_eq(&first_arc, &second_arc),
            "Second call should return the same Arc (pointer-equal), not a new one"
        );
    }
}
