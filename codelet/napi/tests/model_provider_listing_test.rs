//! Feature: spec/features/provider-specific-model-listing-and-refresh-coverage-gaps.feature
//!
//! Work Unit: MODEL-003 - Provider-specific model listing and refresh coverage gaps
//!
//! Covers three gaps identified in the MODEL-002 review:
//!
//! 1. models_list_for_provider must apply is_current_model() filter and
//!    newest-first sort (same as models_list_all).
//!
//! 2. Public refresh coverage must exercise the actual refresh+invalidate path
//!    (not simulate it manually), so removing invalidation breaks this test.
//!
//! 3. get_registry() must not hold the Mutex guard across async initialization.
//!    Concurrent calls must remain race-safe.
//!
//! Each test:
//! 1. Sets the global data directory to a temp dir
//! 2. Writes a models.json disk cache file
//! 3. Exercises the public testing API
//! 4. Verifies behavior on the process-global REGISTRY_CACHE
//!
//! Tests are serialized (#[serial]) because they share global state.

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod model_provider_listing {
    use codelet_napi::models::testing;
    use codelet_providers::models::{
        LimitInfo, ModelInfo, ModelStatus, ModelsDevResponse, ProviderInfo,
    };
    use serial_test::serial;
    use std::collections::HashMap;
    use tempfile::TempDir;

    // =========================================================================
    // HELPERS
    // =========================================================================

    async fn setup_test_env() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        codelet_common::set_data_directory(temp_dir.path().to_path_buf())
            .expect("Failed to set data directory");
        testing::invalidate_registry_cache().await;
        temp_dir
    }

    fn make_model(id: &str, name: &str, release_date: Option<&str>) -> ModelInfo {
        ModelInfo {
            id: id.to_string(),
            name: name.to_string(),
            family: None,
            release_date: release_date.map(str::to_string),
            reasoning: false,
            tool_call: true,
            attachment: false,
            temperature: true,
            interleaved: None,
            modalities: None,
            cost: None,
            limit: LimitInfo {
                context: 128_000,
                output: 16_384,
            },
            status: None,
            experimental: None,
            options: HashMap::new(),
            headers: HashMap::new(),
        }
    }

    fn make_deprecated_model(id: &str) -> ModelInfo {
        let mut m = make_model(id, id, Some("2025-01-01"));
        m.status = Some(ModelStatus::Deprecated);
        m
    }

    fn make_stale_model(id: &str) -> ModelInfo {
        // Released 24 months ago — older than the 18-month cutoff
        make_model(id, id, Some("2023-01-01"))
    }

    fn make_response_with_models(models: Vec<(String, ModelInfo)>) -> ModelsDevResponse {
        let models_map: HashMap<String, ModelInfo> = models.into_iter().collect();

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
                models: models_map,
            },
        );

        ModelsDevResponse { providers }
    }

    fn write_disk_cache(temp_dir: &TempDir, response: &ModelsDevResponse) {
        let cache_dir = temp_dir.path().join("cache");
        std::fs::create_dir_all(&cache_dir).unwrap();
        let cache_path = cache_dir.join("models.json");
        let json = serde_json::to_string(response).expect("Failed to serialize");
        std::fs::write(cache_path, json).expect("Failed to write cache file");
    }

    // =========================================================================
    // Scenario: Provider listing filters deprecated and stale models
    // =========================================================================

    #[tokio::test]
    #[serial]
    async fn test_provider_listing_filters_deprecated_and_stale() {
        // @step Given a provider contains a deprecated model, an older-than-18-months model, and a current model
        let temp_dir = setup_test_env().await;

        let deprecated = make_deprecated_model("model-deprecated");
        let stale = make_stale_model("model-stale");
        let current = make_model("model-current", "Current Model", Some("2025-06-01"));

        let response = make_response_with_models(vec![
            ("model-deprecated".to_string(), deprecated),
            ("model-stale".to_string(), stale),
            ("model-current".to_string(), current),
        ]);
        write_disk_cache(&temp_dir, &response);

        // @step When models_list_for_provider is called for that provider
        let models = testing::list_models_for_provider("test-provider")
            .await
            .expect("list_models_for_provider should succeed");

        let ids: Vec<&str> = models.iter().map(|m| m.id.as_str()).collect();

        // @step Then only the current model is returned
        assert_eq!(ids.len(), 1, "Expected 1 current model, got: {ids:?}");

        // @step And the deprecated model is excluded
        assert!(
            !ids.contains(&"model-deprecated"),
            "Deprecated model must be excluded, got: {ids:?}"
        );

        // @step And the stale model is excluded
        assert!(
            !ids.contains(&"model-stale"),
            "Stale (>18 months) model must be excluded, got: {ids:?}"
        );

        assert!(
            ids.contains(&"model-current"),
            "Current model must be present, got: {ids:?}"
        );
    }

    // =========================================================================
    // Scenario: Provider listing preserves newest-first ordering
    // =========================================================================

    #[tokio::test]
    #[serial]
    async fn test_provider_listing_orders_newest_first() {
        // @step Given a provider contains two current models with different release dates
        let temp_dir = setup_test_env().await;

        let older = make_model("model-older", "Older Model", Some("2025-01-01"));
        let newer = make_model("model-newer", "Newer Model", Some("2025-06-01"));

        let response = make_response_with_models(vec![
            // Insert in "wrong" order to confirm sort is applied
            ("model-older".to_string(), older),
            ("model-newer".to_string(), newer),
        ]);
        write_disk_cache(&temp_dir, &response);

        // @step When models_list_for_provider is called for that provider
        let models = testing::list_models_for_provider("test-provider")
            .await
            .expect("list_models_for_provider should succeed");

        let ids: Vec<&str> = models.iter().map(|m| m.id.as_str()).collect();

        // @step Then the newer model appears before the older model in the result
        let newer_pos = ids.iter().position(|id| *id == "model-newer");
        let older_pos = ids.iter().position(|id| *id == "model-older");

        assert!(newer_pos.is_some(), "model-newer must be present, got: {ids:?}");
        assert!(older_pos.is_some(), "model-older must be present, got: {ids:?}");
        assert!(
            newer_pos.unwrap() < older_pos.unwrap(),
            "model-newer (pos {}) must come before model-older (pos {})",
            newer_pos.unwrap(),
            older_pos.unwrap()
        );
    }

    // =========================================================================
    // Scenario: After public refresh behaviour, next listing returns new model
    //
    // This test calls testing::refresh_registry_from_data() which exercises
    // the SAME invalidation code path as the real models_refresh_cache().
    // If invalidate_registry_cache() is removed from refresh_registry_from_data,
    // this test will fail.
    // =========================================================================

    #[tokio::test]
    #[serial]
    async fn test_public_refresh_path_invalidates_cache_for_provider_listing() {
        // @step Given the registry cache has been initialized with model data containing "codex-5.3"
        let temp_dir = setup_test_env().await;

        let v1 = make_model("codex-5.3", "Codex 5.3", Some("2025-01-01"));
        let response_v1 = make_response_with_models(vec![("codex-5.3".to_string(), v1)]);
        write_disk_cache(&temp_dir, &response_v1);

        // Warm up the cache
        let initial = testing::list_models_for_provider("test-provider")
            .await
            .expect("Initial list should succeed");
        let initial_ids: Vec<&str> = initial.iter().map(|m| m.id.as_str()).collect();
        assert!(
            initial_ids.contains(&"codex-5.3"),
            "Initial cache must contain codex-5.3, got: {initial_ids:?}"
        );

        // @step When the disk cache is refreshed from codex-5.3 to codex-5.4 through the public refresh behaviour
        let v2 = make_model("codex-5.4", "Codex 5.4", Some("2025-06-01"));
        let response_v2 = make_response_with_models(vec![("codex-5.4".to_string(), v2)]);

        // This exercises the ACTUAL refresh+invalidate code path.
        // Removing invalidate_registry_cache() from refresh_registry_from_data()
        // causes this test to fail.
        testing::refresh_registry_from_data(&response_v2)
            .await
            .expect("Refresh should succeed");

        // @step And models_list_for_provider is called to rebuild the registry
        let refreshed = testing::list_models_for_provider("test-provider")
            .await
            .expect("Post-refresh list should succeed");

        let refreshed_ids: Vec<&str> = refreshed.iter().map(|m| m.id.as_str()).collect();

        // @step Then the returned models contain "codex-5.4"
        assert!(
            refreshed_ids.contains(&"codex-5.4"),
            "After public refresh, codex-5.4 must be present, got: {refreshed_ids:?}"
        );

        // @step And "codex-5.3" is no longer returned
        assert!(
            !refreshed_ids.contains(&"codex-5.3"),
            "After public refresh, codex-5.3 must be gone, got: {refreshed_ids:?}"
        );
    }

    // =========================================================================
    // Scenario: Two concurrent callers and an invalidation remain race-safe
    //
    // This verifies that get_registry() does not hold the Mutex guard across
    // async initialization — both concurrent callers receive valid results.
    // =========================================================================

    #[tokio::test]
    #[serial]
    async fn test_concurrent_registry_callers_are_race_safe() {
        // @step Given the registry cache is empty
        let temp_dir = setup_test_env().await;

        let model = make_model("concurrent-model", "Concurrent Model", Some("2025-06-01"));
        let response = make_response_with_models(vec![("concurrent-model".to_string(), model)]);
        write_disk_cache(&temp_dir, &response);

        // @step When two concurrent callers request the registry while another invalidates after refresh
        let (r1, r2) = tokio::join!(
            testing::get_registry(),
            testing::get_registry(),
        );

        // @step Then both callers receive a valid registry without a data race
        let reg1 = r1.expect("First concurrent caller must succeed");
        let reg2 = r2.expect("Second concurrent caller must succeed");

        let ids1: Vec<String> = reg1
            .list_providers()
            .iter()
            .flat_map(|p| p.models.values().map(|m| m.id.clone()))
            .collect();
        let ids2: Vec<String> = reg2
            .list_providers()
            .iter()
            .flat_map(|p| p.models.values().map(|m| m.id.clone()))
            .collect();

        assert!(
            ids1.contains(&"concurrent-model".to_string()),
            "First caller must see concurrent-model, got: {ids1:?}"
        );
        assert!(
            ids2.contains(&"concurrent-model".to_string()),
            "Second caller must see concurrent-model, got: {ids2:?}"
        );

        // @step And subsequent reads rebuild from the refreshed cache
        let new_model = make_model("refreshed-model", "Refreshed Model", Some("2025-06-15"));
        let refreshed_response =
            make_response_with_models(vec![("refreshed-model".to_string(), new_model)]);
        testing::refresh_registry_from_data(&refreshed_response)
            .await
            .expect("Refresh should succeed");

        let post_refresh = testing::list_models_for_provider("test-provider")
            .await
            .expect("Post-refresh listing should succeed");
        let post_ids: Vec<&str> = post_refresh.iter().map(|m| m.id.as_str()).collect();
        assert!(
            post_ids.contains(&"refreshed-model"),
            "Subsequent read must see refreshed-model, got: {post_ids:?}"
        );
    }
}
