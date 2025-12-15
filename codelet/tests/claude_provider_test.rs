//! Feature: spec/features/anthropic-claude-provider.feature
//!
//! Tests for Anthropic Claude Provider Implementation - PROV-001
//!
//! These tests verify the ClaudeProvider implementation of the LlmProvider trait
//! for communicating with the Anthropic Claude API.

use codelet::agent::Message;
use codelet::providers::ClaudeProvider;
use codelet::providers::LlmProvider;

// ==========================================
// CLAUDE PROVIDER TESTS
// ==========================================

/// Scenario: Complete simple user message with valid API key
#[tokio::test]
async fn test_complete_simple_user_message_with_valid_api_key() {
    // @step Given a ClaudeProvider is created with valid ANTHROPIC_API_KEY
    // Skip this test if no API key is set (for CI environments)
    if std::env::var("ANTHROPIC_API_KEY").is_err() {
        eprintln!("Skipping test: ANTHROPIC_API_KEY not set");
        return;
    }

    let provider = ClaudeProvider::new().expect("Should create provider with valid API key");

    // @step When I send a user message 'Hello, Claude'
    let messages = vec![Message::user(
        "Hello, Claude. Reply with just 'Hi' and nothing else.",
    )];
    let result = provider.complete(&messages).await;

    // @step Then I receive a non-empty text response
    assert!(result.is_ok(), "Should receive successful response");
    let response = result.unwrap();
    assert!(!response.is_empty(), "Response should not be empty");
}

/// Scenario: Reject provider creation when API key is missing
#[test]
fn test_reject_provider_creation_when_api_key_missing() {
    // @step Given the ANTHROPIC_API_KEY environment variable is not set
    // We need to temporarily unset the API key for this test
    // This test uses a separate constructor that doesn't read from env
    let result = ClaudeProvider::from_api_key("");

    // @step When I attempt to create a ClaudeProvider
    // (constructor is called above)

    // @step Then I receive an error containing 'ANTHROPIC_API_KEY'
    assert!(result.is_err(), "Should fail with empty API key");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("API key") || err.to_string().contains("ANTHROPIC"),
        "Error should mention API key: {}",
        err
    );
}

/// Scenario: Handle authentication error for invalid API key
#[tokio::test]
async fn test_handle_authentication_error_for_invalid_api_key() {
    // @step Given a ClaudeProvider is created with an invalid API key
    let provider =
        ClaudeProvider::from_api_key("invalid-key-12345").expect("Should accept any non-empty key");

    // @step When I send a user message
    let messages = vec![Message::user("Hello")];
    let result = provider.complete(&messages).await;

    // @step Then I receive an authentication error from the API
    assert!(result.is_err(), "Should fail with invalid API key");
    let err = result.unwrap_err();
    // Anthropic returns 401 for invalid API keys
    assert!(
        err.to_string().contains("401")
            || err.to_string().contains("authentication")
            || err.to_string().contains("unauthorized")
            || err.to_string().contains("invalid"),
        "Error should indicate authentication failure: {}",
        err
    );
}

/// Scenario: Format system and user messages correctly for API
#[tokio::test]
async fn test_format_system_and_user_messages_correctly() {
    // @step Given a ClaudeProvider is created with valid credentials
    // Skip if no API key
    if std::env::var("ANTHROPIC_API_KEY").is_err() {
        eprintln!("Skipping test: ANTHROPIC_API_KEY not set");
        return;
    }

    let provider = ClaudeProvider::new().expect("Should create provider");

    // @step When I send messages with system and user roles
    let messages = vec![
        Message::system("You are a helpful assistant. Always respond with exactly 'OK'."),
        Message::user("Please confirm."),
    ];
    let result = provider.complete(&messages).await;

    // @step Then the API request separates system content from the messages array
    // We verify this by checking we get a valid response (API would reject if format wrong)
    assert!(
        result.is_ok(),
        "Should succeed with system + user messages: {:?}",
        result.err()
    );
    let response = result.unwrap();
    assert!(!response.is_empty(), "Response should not be empty");
}

/// Scenario: Return correct provider name
#[test]
fn test_return_correct_provider_name() {
    // @step Given a ClaudeProvider instance exists
    let provider = ClaudeProvider::from_api_key("test-key").expect("Should create provider");

    // @step When I query the provider name
    let name = provider.name();

    // @step Then I receive 'claude'
    assert_eq!(name, "claude", "Provider name should be 'claude'");
}

/// Scenario: Return correct model limits
#[test]
fn test_return_correct_model_limits() {
    // @step Given a ClaudeProvider instance exists
    let provider = ClaudeProvider::from_api_key("test-key").expect("Should create provider");

    // @step When I query the context window and max output tokens
    let context_window = provider.context_window();
    let max_output = provider.max_output_tokens();

    // @step Then I receive context_window=200000 and max_output_tokens=8192
    assert_eq!(
        context_window, 200000,
        "Context window should be 200000 tokens"
    );
    assert_eq!(max_output, 8192, "Max output tokens should be 8192");
}

// ==========================================
// ADDITIONAL TRAIT IMPLEMENTATION TESTS
// ==========================================

/// Test that ClaudeProvider reports correct model name
#[test]
fn test_provider_model_name() {
    // @step Given a ClaudeProvider instance exists
    let provider = ClaudeProvider::from_api_key("test-key").expect("Should create provider");

    // @step When I query the model name
    let model = provider.model();

    // @step Then I receive the default Claude model
    assert_eq!(
        model, "claude-sonnet-4-20250514",
        "Should use claude-sonnet-4 model"
    );
}

/// Test that ClaudeProvider reports caching support
#[test]
fn test_provider_supports_caching() {
    // @step Given a ClaudeProvider instance exists
    let provider = ClaudeProvider::from_api_key("test-key").expect("Should create provider");

    // @step When I check caching support
    let supports_caching = provider.supports_caching();

    // @step Then caching is supported (Anthropic supports prompt caching)
    assert!(supports_caching, "Claude should support prompt caching");
}

/// Test that ClaudeProvider reports streaming support
#[test]
fn test_provider_streaming_support() {
    // @step Given a ClaudeProvider instance exists
    let provider = ClaudeProvider::from_api_key("test-key").expect("Should create provider");

    // @step When I check streaming support
    let supports_streaming = provider.supports_streaming();

    // @step Then streaming is supported (implemented in REFAC-003)
    assert!(
        supports_streaming,
        "Streaming implemented in REFAC-003 via rig integration"
    );
}
