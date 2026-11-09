//! Shared models.dev fixture for the model selection test files
//! (REFAC-029 split of model_selection_test.rs).
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use codelet_providers::models::{
    LimitInfo, Modalities, Modality, ModelInfo, ModelsDevResponse, ProviderInfo,
};
use std::collections::HashMap;
use tempfile::TempDir;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Helper to get test cache directory
pub fn test_cache_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp directory")
}

/// Helper to create a test registry with sample data
pub fn create_test_response() -> ModelsDevResponse {
    let mut providers = HashMap::new();

    // Anthropic provider with 3 models
    let mut anthropic_models = HashMap::new();
    anthropic_models.insert(
        "claude-sonnet-4".to_string(),
        ModelInfo {
            id: "claude-sonnet-4-20250514".to_string(),
            name: "Claude Sonnet 4".to_string(),
            family: Some("claude-sonnet".to_string()),
            release_date: Some("2025-05-14".to_string()),
            reasoning: true,
            tool_call: true,
            attachment: true,
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
            reasoning: true,
            tool_call: true,
            attachment: true,
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
                output: 32000,
            },
            status: None,
            experimental: None,
            options: HashMap::new(),
            headers: HashMap::new(),
        },
    );
    anthropic_models.insert(
        "claude-haiku".to_string(),
        ModelInfo {
            id: "claude-haiku-4-5-20251015".to_string(),
            name: "Claude Haiku 4".to_string(),
            family: Some("claude-haiku".to_string()),
            release_date: None,
            reasoning: false,
            tool_call: true,
            attachment: true,
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
                output: 8192,
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

    // Google provider with 2 models
    let mut google_models = HashMap::new();
    google_models.insert(
        "gemini-2.5-pro".to_string(),
        ModelInfo {
            id: "gemini-2.5-pro".to_string(),
            name: "Gemini 2.5 Pro".to_string(),
            family: Some("gemini".to_string()),
            release_date: None,
            reasoning: true,
            tool_call: true,
            attachment: true,
            temperature: true,
            interleaved: None,
            reasoning_options: None,
            modalities: Some(Modalities {
                input: vec![Modality::Text, Modality::Image, Modality::Video],
                output: vec![Modality::Text],
            }),
            cost: None,
            limit: LimitInfo {
                context: 1000000,
                output: 65536,
            },
            status: None,
            experimental: None,
            options: HashMap::new(),
            headers: HashMap::new(),
        },
    );
    google_models.insert(
        "gemini-2.0-flash-thinking-exp".to_string(),
        ModelInfo {
            id: "gemini-2.0-flash-thinking-exp".to_string(),
            name: "Gemini 2.0 Flash Thinking".to_string(),
            family: Some("gemini".to_string()),
            release_date: None,
            reasoning: true,
            tool_call: false, // No tool_call!
            attachment: false,
            temperature: true,
            interleaved: None,
            reasoning_options: None,
            modalities: None, // No vision
            cost: None,
            limit: LimitInfo {
                context: 32000,
                output: 8192,
            },
            status: None,
            experimental: None,
            options: HashMap::new(),
            headers: HashMap::new(),
        },
    );

    providers.insert(
        "google".to_string(),
        ProviderInfo {
            id: "google".to_string(),
            name: "Google".to_string(),
            env: vec!["GOOGLE_GENERATIVE_AI_API_KEY".to_string()],
            npm: None,
            api: None,
            doc: None,
            models: google_models,
        },
    );

    // OpenAI provider with 1 model
    let mut openai_models = HashMap::new();
    openai_models.insert(
        "gpt-4o".to_string(),
        ModelInfo {
            id: "gpt-4o".to_string(),
            name: "GPT-4o".to_string(),
            family: Some("gpt-4".to_string()),
            release_date: None,
            reasoning: false, // No reasoning
            tool_call: true,
            attachment: true,
            temperature: true,
            interleaved: None,
            reasoning_options: None,
            modalities: Some(Modalities {
                input: vec![Modality::Text, Modality::Image],
                output: vec![Modality::Text],
            }),
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

    providers.insert(
        "openai".to_string(),
        ProviderInfo {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            env: vec!["OPENAI_API_KEY".to_string()],
            npm: None,
            api: None,
            doc: None,
            models: openai_models,
        },
    );

    ModelsDevResponse { providers }
}
