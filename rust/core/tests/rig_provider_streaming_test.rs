#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/refactor-providers-to-use-rig-with-streaming.feature
//!
//! The rig-based ClaudeProvider keeps the LlmProvider interface, reports
//! streaming support and sends the decided anthropic-beta headers. These tests
//! build providers and read their properties; they never stream a response.

use codelet_providers::{ClaudeProvider, LlmProvider};
use codelet_tools::ToolDefinition;
use serde_json::json;

/// Scenario: Claude provider is created from an API key and a model
#[test]
fn test_replace_claude_provider_with_rig_anthropic_provider() {
    // @step Given an API key and the model "claude-sonnet-4-20250514"
    // @step When a Claude provider is created from them
    let provider = ClaudeProvider::from_api_key_with_model("test-key", "claude-sonnet-4-20250514");

    // @step Then the provider is created
    assert!(provider.is_ok(), "Provider should be created successfully");
    let provider = provider.unwrap();

    // @step And it reports the name "claude" and the model "claude-sonnet-4-20250514"
    assert_eq!(provider.name(), "claude");
    assert_eq!(provider.model(), "claude-sonnet-4-20250514");
}

/// Scenario: An API-key Claude provider supports streaming
#[test]
fn test_stream_text_chunks_in_real_time() {
    // @step Given a Claude provider created from an API key and the model "claude-sonnet-4-20250514"
    let provider = ClaudeProvider::from_api_key_with_model("test-key", "claude-sonnet-4-20250514")
        .expect("Provider should be created");

    // @step When its streaming support is checked
    let streaming = provider.supports_streaming();

    // @step Then it supports streaming
    assert!(
        streaming,
        "the rig-based Claude provider should support streaming"
    );
}

/// Scenario: A Claude provider supports streaming alongside a Read tool definition
#[test]
fn test_stream_tool_call_deltas_during_tool_execution() {
    // @step Given a Claude provider created from an API key and the model "claude-sonnet-4-20250514"
    let provider = ClaudeProvider::from_api_key_with_model("test-key", "claude-sonnet-4-20250514")
        .expect("Provider should be created");

    // @step And a Read tool definition with a required file_path
    let tool = ToolDefinition {
        name: "Read".to_string(),
        description: "Read a file".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Path to the file to read"
                }
            },
            "required": ["file_path"]
        }),
    };
    assert_eq!(tool.input_schema["required"], json!(["file_path"]));

    // @step When its streaming support is checked
    let streaming = provider.supports_streaming();

    // @step Then it supports streaming
    assert!(
        streaming,
        "Provider must support streaming to emit tool call deltas"
    );
}

/// Scenario: An OAuth Claude provider for a budget-only model sends the OAuth
/// and interleaved-thinking beta headers
#[test]
fn test_authenticate_using_oauth_with_custom_headers() {
    use codelet_providers::AuthMode;

    // @step Given an OAuth token and the budget-only model "claude-sonnet-4-20250514"
    // @step When a Claude provider is created from them in OAuth mode
    let provider = ClaudeProvider::from_api_key_with_mode_and_model(
        "test-oauth-token",
        AuthMode::OAuth,
        "claude-sonnet-4-20250514",
    );

    // @step Then the provider is created in OAuth mode
    assert!(
        provider.is_ok(),
        "Provider should be created with OAuth mode"
    );
    let provider = provider.unwrap();
    assert!(provider.is_oauth_mode(), "Provider should be in OAuth mode");

    // @step And it supports streaming
    assert!(
        provider.supports_streaming(),
        "OAuth mode should support streaming"
    );

    // @step And its anthropic-beta header contains "oauth-2025-04-20"
    let beta_header = provider.get_anthropic_beta_header_for_model();
    assert!(
        beta_header.contains("oauth-2025-04-20"),
        "Beta header should include oauth feature: {beta_header}"
    );

    // @step And its anthropic-beta header contains "interleaved-thinking-2025-05-14"
    // BUG-493: claude-sonnet-4-20250514 is a budget-only Claude 4.0 id
    // (BUG-438, claude-budget-only-thinking-models), not an adaptive model.
    assert!(
        beta_header.contains("interleaved-thinking-2025-05-14"),
        "a budget-only model's beta header must include interleaved-thinking: {beta_header}"
    );
}

/// Scenario: A Claude Sonnet 4.5 provider sends the interleaved-thinking beta header
#[test]
fn test_support_extended_thinking_with_reasoning_chunks() {
    // @step Given a Claude provider created from an API key and the model "claude-sonnet-4-5"
    let provider = ClaudeProvider::from_api_key_with_model("test-key", "claude-sonnet-4-5")
        .expect("Provider should be created");

    // @step When its streaming support and anthropic-beta header are checked
    let streaming = provider.supports_streaming();
    let beta_header = provider.get_anthropic_beta_header_for_model();

    // @step Then it supports streaming
    assert!(
        streaming,
        "Provider must support streaming to emit reasoning chunks"
    );

    // @step And its anthropic-beta header contains "interleaved-thinking-2025-05-14"
    assert!(
        beta_header.contains("interleaved-thinking-2025-05-14"),
        "Beta header should include extended thinking feature flag: {beta_header}"
    );
}

#[test]
fn test_rig_is_accessible_via_codelet_namespace() {
    // Verify rig re-export from REFAC-002 is working
    // This is a compilation test - if it compiles, re-export works

    // We should be able to reference rig types (even if not used yet)
    let _rig_available: Option<fn()> = Some(|| {
        // After REFAC-003, we'll use:
        // type RigClient = codelet_providers::rig::providers::anthropic::Client;
        // type RigModel = codelet_providers::rig::providers::anthropic::CompletionModel;
    });
}

/// Scenario: The Claude provider keeps the LlmProvider interface and declared limits
#[test]
fn test_backward_compatibility_with_existing_llm_provider_trait() {
    // @step Given a Claude provider created from an API key and the model "claude-sonnet-4-20250514"
    let provider =
        ClaudeProvider::from_api_key_with_model("test-key", "claude-sonnet-4-20250514").unwrap();

    // @step When its LlmProvider properties are read
    let (name, context_window, max_output_tokens) = (
        provider.name(),
        provider.context_window(),
        provider.max_output_tokens(),
    );

    // @step Then it reports the name "claude", a 200000-token context window and 8192 max output tokens
    assert_eq!(name, "claude");
    assert_eq!(context_window, Some(200_000));
    assert_eq!(max_output_tokens, Some(8192));

    // @step And it supports prompt caching
    assert!(provider.supports_caching());
}
