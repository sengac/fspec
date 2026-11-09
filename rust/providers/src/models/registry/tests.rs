//! ModelRegistry unit tests (REFAC-029: moved out of registry.rs).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::models::types::{LimitInfo, Modalities, Modality};

fn create_test_response() -> ModelsDevResponse {
    let mut providers = HashMap::new();

    // Anthropic provider
    let mut anthropic_models = HashMap::new();
    anthropic_models.insert(
        "claude-sonnet-4".to_string(),
        ModelInfo {
            id: "claude-sonnet-4-20250514".to_string(),
            name: "Claude Sonnet 4".to_string(),
            family: Some("claude-sonnet".to_string()),
            release_date: Some("2025-05-14".to_string()),
            attachment: true,
            reasoning: true,
            tool_call: true,
            temperature: true,
            interleaved: None,
            reasoning_options: None,
            modalities: Some(Modalities {
                input: vec![Modality::Text, Modality::Image],
                output: vec![Modality::Text],
            }),
            cost: None,
            limit: LimitInfo {
                context: 200000,
                output: 16000,
            },
            status: None,
            experimental: None,
            options: HashMap::new(),
            headers: HashMap::new(),
        },
    );
    anthropic_models.insert(
        "claude-opus-4".to_string(),
        ModelInfo {
            id: "claude-opus-4-5-20251101".to_string(),
            name: "Claude Opus 4".to_string(),
            family: Some("claude-opus".to_string()),
            release_date: None,
            attachment: true,
            reasoning: true,
            tool_call: true,
            temperature: true,
            interleaved: None,
            reasoning_options: None,
            modalities: Some(Modalities {
                input: vec![Modality::Text, Modality::Image],
                output: vec![Modality::Text],
            }),
            cost: None,
            limit: LimitInfo {
                context: 200_000,
                output: 16_000,
            },
            status: None,
            experimental: None,
            options: HashMap::new(),
            headers: HashMap::new(),
        },
    );

    providers.insert(
        "anthropic".to_string(),
        ProviderInfo {
            id: "anthropic".to_string(),
            name: "Anthropic".to_string(),
            env: vec!["ANTHROPIC_API_KEY".to_string()],
            npm: Some("@ai-sdk/anthropic".to_string()),
            api: None,
            doc: None,
            models: anthropic_models,
        },
    );

    ModelsDevResponse { providers }
}

#[test]
fn test_parse_model_string() {
    let registry = ModelRegistry::from_response(create_test_response());

    let result = registry.parse_model_string("anthropic/claude-sonnet-4");
    assert!(result.is_ok());
    let (provider, model) = result.unwrap();
    assert_eq!(provider, "anthropic");
    assert_eq!(model, "claude-sonnet-4");
}

#[test]
fn test_parse_model_string_invalid_format() {
    let registry = ModelRegistry::from_response(create_test_response());

    let result = registry.parse_model_string("claude-sonnet-4");
    assert!(result.is_err());
}

#[test]
fn test_get_model() {
    let registry = ModelRegistry::from_response(create_test_response());

    let model = registry.get_model("anthropic", "claude-sonnet-4");
    assert!(model.is_ok());
    assert_eq!(model.unwrap().name, "Claude Sonnet 4");
}

#[test]
fn test_get_model_with_suggestions() {
    let registry = ModelRegistry::from_response(create_test_response());

    // BUG-335: lookups are exact, so an unknown id gets suggestions.
    let result = registry.get_model("anthropic", "sonnt");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Did you mean"));
}

#[test]
fn test_list_providers() {
    let registry = ModelRegistry::from_response(create_test_response());

    let providers = registry.list_providers();
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0].name, "Anthropic");
}

#[test]
fn test_filter_by_capability() {
    let registry = ModelRegistry::from_response(create_test_response());

    let reasoning_models = registry.filter_by_capability(Capability::Reasoning);
    assert!(!reasoning_models.is_empty());

    let vision_models = registry.filter_by_capability(Capability::Vision);
    assert!(!vision_models.is_empty());
}

#[test]
fn test_search() {
    let registry = ModelRegistry::from_response(create_test_response());

    let results = registry.search("claude");
    assert_eq!(results.len(), 2);
}

#[test]
fn test_levenshtein() {
    assert_eq!(levenshtein_distance("claude", "claud"), 1);
    assert_eq!(levenshtein_distance("sonnet", "sonnet"), 0);
    assert_eq!(levenshtein_distance("gpt", "gemini"), 5);
}
