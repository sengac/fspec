#![allow(clippy::unwrap_used, clippy::expect_used)]
// Feature: spec/features/model-registry-selection-and-filtering.feature
// Work Units: MODEL-001 (scenarios), REFAC-029 (split from
// model_selection_test.rs; exact-match lookup after BUG-335).

mod model_selection_support;

use codelet_providers::models::{Capability, Modality, ModelRegistry};
use model_selection_support::create_test_response;
use serial_test::serial;

// =============================================================================
// MODEL REGISTRY SCENARIOS
// =============================================================================

/// Scenario: Select model using full provider/model path
#[test]
#[serial]
fn test_select_model_using_full_provider_model_path() {
    // @step Given the models cache contains anthropic provider with claude-sonnet-4 model
    let registry = ModelRegistry::from_response(create_test_response());

    // @step When I run codelet with --model anthropic/claude-sonnet-4
    let result = registry.parse_model_string("anthropic/claude-sonnet-4");

    // @step Then the model should be selected from the registry
    assert!(result.is_ok(), "Should parse model string successfully");
    let (provider, model) = result.unwrap();
    assert_eq!(provider, "anthropic");
    assert_eq!(model, "claude-sonnet-4");

    // @step And the appropriate provider facade should be used
    let model_info = registry.get_model(&provider, &model).unwrap();
    assert_eq!(model_info.name, "Claude Sonnet 4");
}

/// Scenario: List models for a specific provider
#[test]
#[serial]
fn test_list_models_for_specific_provider() {
    // @step Given the models cache contains anthropic provider with multiple models
    let registry = ModelRegistry::from_response(create_test_response());

    // @step When I run codelet models anthropic
    let models = registry.list_models("anthropic");

    // @step Then all Anthropic models should be displayed
    assert!(models.is_ok(), "Should list models successfully");
    let models = models.unwrap();
    assert_eq!(models.len(), 3, "Should have 3 Anthropic models");

    // @step And each model should show name and capabilities
    let model_names: Vec<&str> = models.iter().map(|m| m.name.as_str()).collect();
    assert!(model_names.contains(&"Claude Sonnet 4"));
    assert!(model_names.contains(&"Claude Opus 4"));
    assert!(model_names.contains(&"Claude Haiku 4"));
}

/// Scenario: Unknown model shows error with fuzzy suggestions
#[test]
#[serial]
fn test_unknown_model_shows_error_with_fuzzy_suggestions() {
    // @step Given the models cache contains anthropic provider with claude models
    let registry = ModelRegistry::from_response(create_test_response());

    // @step When I run codelet with --model anthropic/claud
    let result = registry.get_model("anthropic", "claud");

    // @step Then an error should be displayed: "Model 'claud' not found in provider 'anthropic'"
    let error_msg = result.expect_err("an unknown model").to_string();
    assert!(
        error_msg.contains("claud") && error_msg.contains("not found"),
        "Error should name the unknown model: {error_msg}"
    );

    // @step And fuzzy match suggestions should be shown: claude-sonnet-4, claude-opus-4, claude-haiku
    assert!(
        error_msg.contains("Did you mean") && error_msg.contains("claude-sonnet-4"),
        "Error should include suggestions: {error_msg}"
    );
}

/// Scenario: Reject model without tool_call capability
#[test]
#[serial]
fn test_reject_model_without_tool_call_capability() {
    // @step Given the cache contains google/gemini-2.0-flash-thinking-exp with tool_call=false
    let registry = ModelRegistry::from_response(create_test_response());

    // @step When I run codelet with --model google/gemini-2.0-flash-thinking-exp
    let result = registry.validate_model_for_use("google", "gemini-2.0-flash-thinking-exp");

    // @step Then an error should be displayed: "Model google/gemini-2.0-flash-thinking-exp does not support tool_call"
    assert!(result.is_err(), "Should reject model without tool_call");
    let error = result.unwrap_err();
    let error_msg = error.to_string();
    assert!(
        error_msg.contains("tool_call"),
        "Error should mention tool_call requirement: {error_msg}"
    );
    // @step And the command should exit with code 1
    // (CLI exit code testing is at CLI level, here we verify the error is returned)
}

/// Scenario: Filter models by reasoning capability
#[test]
#[serial]
fn test_filter_models_by_reasoning_capability() {
    // @step Given the cache contains models with and without reasoning capability
    let registry = ModelRegistry::from_response(create_test_response());

    // @step When I run codelet models --reasoning
    let models = registry.filter_by_capability(Capability::Reasoning);

    // @step Then only models with reasoning=true should be displayed
    assert!(!models.is_empty(), "Should find models with reasoning");
    for (_provider, model) in &models {
        assert!(
            model.reasoning,
            "All returned models should have reasoning=true"
        );
    }

    // Verify specific models are included
    let model_ids: Vec<&str> = models.iter().map(|(_, m)| m.id.as_str()).collect();
    assert!(model_ids.contains(&"claude-sonnet-4-20250514"));
    assert!(model_ids.contains(&"claude-opus-4-5-20251101"));
    assert!(model_ids.contains(&"gemini-2.5-pro"));

    // Verify GPT-4o is NOT included (reasoning=false)
    assert!(!model_ids.contains(&"gpt-4o"));
}

/// Scenario: List all available providers
#[test]
#[serial]
fn test_list_all_available_providers() {
    // @step Given the models cache contains anthropic, google, and openai providers
    let registry = ModelRegistry::from_response(create_test_response());

    // @step When I run codelet models --providers
    let providers = registry.list_providers();

    // @step Then a list of providers should be displayed
    assert_eq!(providers.len(), 3, "Should have 3 providers");

    // @step And each provider should show id, name, and model count
    let provider_ids: Vec<&str> = registry.list_provider_ids();
    assert!(provider_ids.contains(&"anthropic"));
    assert!(provider_ids.contains(&"google"));
    assert!(provider_ids.contains(&"openai"));
}

/// Scenario: Filter models by vision capability
#[test]
#[serial]
fn test_filter_models_by_vision_capability() {
    // @step Given the cache contains models with and without vision capability
    let registry = ModelRegistry::from_response(create_test_response());

    // @step When I run codelet models --vision
    let models = registry.filter_by_capability(Capability::Vision);

    // @step Then only models with image input modality should be displayed
    assert!(!models.is_empty(), "Should find models with vision");
    for (_, model) in &models {
        let has_image = model
            .modalities
            .as_ref()
            .map(|m| m.input.contains(&Modality::Image))
            .unwrap_or(false);
        assert!(
            has_image,
            "All returned models should have image input modality"
        );
    }

    // Verify gemini-2.0-flash-thinking-exp is NOT included (no modalities)
    assert!(!models
        .iter()
        .map(|(_, m)| m.id.as_str())
        .any(|id| id == "gemini-2.0-flash-thinking-exp"));
}

/// Scenario: Search models with fuzzy matching
#[test]
#[serial]
fn test_search_models_with_fuzzy_matching() {
    // @step Given the cache contains multiple models from various providers
    let registry = ModelRegistry::from_response(create_test_response());

    // @step When I run codelet models --search "claude"
    let results = registry.search("claude");

    // @step Then all models containing "claude" in name or id should be displayed
    assert!(!results.is_empty(), "Should find models matching 'claude'");

    // @step And results should include anthropic/claude-sonnet-4 and anthropic/claude-opus-4
    let model_ids: Vec<&str> = results.iter().map(|(_, m)| m.id.as_str()).collect();
    assert!(
        model_ids.iter().any(|id| id.contains("sonnet")),
        "Should find claude-sonnet: {model_ids:?}"
    );
    assert!(
        model_ids.iter().any(|id| id.contains("opus")),
        "Should find claude-opus: {model_ids:?}"
    );
    assert!(
        model_ids.iter().any(|id| id.contains("haiku")),
        "Should find claude-haiku: {model_ids:?}"
    );
}

/// Scenario: Show verbose model details
#[test]
#[serial]
fn test_show_verbose_model_details() {
    // @step Given the cache contains anthropic/claude-sonnet-4 model
    let registry = ModelRegistry::from_response(create_test_response());

    // @step When I run codelet models anthropic/claude-sonnet-4 --verbose
    let model = registry.get_model("anthropic", "claude-sonnet-4").unwrap();

    // @step Then detailed model information should be displayed
    assert_eq!(model.id, "claude-sonnet-4-20250514");
    assert_eq!(model.name, "Claude Sonnet 4");

    // @step And the output should include context limit, output limit, capabilities, and cost
    assert_eq!(model.limit.context, 200000);
    assert_eq!(model.limit.output, 16000);
    assert!(model.reasoning);
    assert!(model.tool_call);
    assert!(model.attachment);
}

/// Scenario: A prefix of a catalog model id is not found
#[test]
fn a_prefix_of_a_catalog_model_id_is_not_found() {
    // @step Given the models cache contains anthropic provider with claude-sonnet-4 model
    let registry = ModelRegistry::from_response(create_test_response());
    assert!(registry.get_model("anthropic", "claude-sonnet-4").is_ok());
    // @step When the model "claude-sonnet" is looked up in provider "anthropic"
    let result = registry.get_model("anthropic", "claude-sonnet");
    // @step Then the lookup fails with a model-not-found error
    let error = result.expect_err("a prefix must not match").to_string();
    assert!(error.contains("claude-sonnet"), "{error}");
}

/// Scenario: The registry and model selection sources stay within the file size limit
#[test]
fn the_registry_and_model_selection_sources_stay_within_the_file_size_limit() {
    // @step Given the files rust/providers/src/models/registry.rs and rust/providers/tests/model_selection_test.rs
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let files = [
        root.join("src/models/registry.rs"),
        root.join("tests/model_selection_test.rs"),
    ];
    for file in files {
        // @step When their line counts and comments are read
        let source = std::fs::read_to_string(&file).expect("source is readable");
        // @step Then each has at most 300 lines
        assert!(
            source.lines().count() <= 300,
            "{} is over 300 lines",
            file.display()
        );
        // @step And neither claims that a model id matches by prefix
        assert!(
            !source.contains("matches via prefix"),
            "{} still claims prefix matching",
            file.display()
        );
    }
}
