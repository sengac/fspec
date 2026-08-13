//! Feature: spec/features/custom-model-rpc-surface.feature
//!
//! RPC-347 — NAPI binding round-trip for the custom-model write surface.
//!
//! Exercises the real `add_custom_model` NAPI binding (gated behind
//! `not(feature = "noop")`) against a temporary `fspec-config.json` pointed to
//! by `FSPEC_USER_DIR`. Serialized because the env var is process-global.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[cfg(all(test, not(feature = "noop")))]
mod napi_binding_exposes_custom_model_add {
    use codelet_rpc_types::CustomModelDefinition;
    use serde_json::Value;
    use serial_test::serial;
    use tempfile::TempDir;

    // Scenario: CustomModelDefinition round-trips through the NAPI boundary
    #[tokio::test]
    #[serial]
    async fn test_add_custom_model_napi_round_trip() {
        let tmp = TempDir::new().unwrap();

        // @step Given an openai profile "work-vllm" exists with no custom models
        let root = serde_json::json!({
            "providers": { "openai": { "profiles": {
                "work-vllm": { "baseUrl": "http://localhost:8000" }
            } } }
        });
        std::fs::write(
            tmp.path().join("fspec-config.json"),
            serde_json::to_string_pretty(&root).unwrap(),
        )
        .unwrap();
        std::env::set_var("FSPEC_USER_DIR", tmp.path());

        let definition = CustomModelDefinition {
            id: "my-model".to_string(),
            display_name: Some("My Model".to_string()),
            facade: Some("claude".to_string()),
            context_window: Some(200_000),
            max_output_tokens: Some(8_192),
            compaction_threshold_type: Some("tokens".to_string()),
            compaction_threshold_value: Some(150_000),
            reasoning: Some(true),
            has_vision: Some(false),
        };

        // @step When the NAPI add_custom_model binding receives a CustomModelDefinition-shaped object for "work-vllm"
        let result =
            codelet_napi::add_custom_model("openai".into(), "work-vllm".into(), definition).await;

        // @step Then the call resolves successfully
        result.expect("add_custom_model NAPI binding should resolve");

        // @step And the persisted entry carries the same id and optional fields that were supplied
        let content = std::fs::read_to_string(tmp.path().join("fspec-config.json")).unwrap();
        let root: Value = serde_json::from_str(&content).unwrap();
        let models = root["providers"]["openai"]["profiles"]["work-vllm"]["customModels"]
            .as_array()
            .expect("customModels[]");
        assert_eq!(models.len(), 1);
        assert_eq!(models[0]["id"], "my-model");
        assert_eq!(models[0]["displayName"], "My Model");
        assert_eq!(models[0]["facade"], "claude");
        assert_eq!(models[0]["contextWindow"], 200_000);
        assert_eq!(models[0]["maxOutputTokens"], 8_192);
        assert_eq!(models[0]["compactionThreshold"]["type"], "tokens");
        assert_eq!(models[0]["compactionThreshold"]["value"], 150_000);
        assert_eq!(models[0]["reasoning"], true);
        assert_eq!(models[0]["hasVision"], false);
    }
}
