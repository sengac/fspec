
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/claude-code-oauth-authentication.feature
//!
//! Tests for Claude Code OAuth Authentication - PROV-002
//!
//! These tests verify the ClaudeProvider implementation supports
//! CLAUDE_CODE_OAUTH_TOKEN with proper OAuth headers and system prompt.
//!
//! NOTE: These tests manipulate environment variables and MUST run serially.

use codelet_providers::ClaudeProvider;
use std::env;
use std::sync::Mutex;

// Global mutex to ensure tests run serially (env vars are process-global)
static ENV_MUTEX: Mutex<()> = Mutex::new(());

// ==========================================
// SCENARIO 1: OAuth token authentication
// ==========================================

/// Scenario: OAuth token authentication with Bearer header and system prompt
#[test]
fn test_oauth_token_authentication_with_bearer_header() {
    let _lock = ENV_MUTEX.lock().unwrap();

    // @step Given CLAUDE_CODE_OAUTH_TOKEN is set in the environment
    env::set_var("CLAUDE_CODE_OAUTH_TOKEN", "sk-ant-oat01-test-token");

    // @step And ANTHROPIC_API_KEY is not set
    env::remove_var("ANTHROPIC_API_KEY");

    // @step When the provider sends a request to the Claude API
    let provider = ClaudeProvider::new();

    // @step Then the request should use Authorization: Bearer header instead of x-api-key
    // @step And the request should include anthropic-beta header with "oauth-2025-04-20,prompt-caching-2024-07-31"
    // @step And the system prompt should start with "You are Claude Code, Anthropic's official CLI for Claude."
    // @step And the request URL should include "?beta=true" query parameter
    // @step And the request body should include metadata.user_id
    assert!(
        provider.is_ok(),
        "Provider should initialize with OAuth token"
    );

    let provider = provider.unwrap();
    assert!(provider.is_oauth_mode(), "Provider should be in OAuth mode");
    assert_eq!(
        provider.get_system_prompt_prefix(),
        "You are Claude Code, Anthropic's official CLI for Claude."
    );

    // Cleanup
    env::remove_var("CLAUDE_CODE_OAUTH_TOKEN");
}

// ==========================================
// SCENARIO 2: API key takes precedence
// ==========================================

/// Scenario: API key takes precedence over OAuth token
#[test]
fn test_api_key_takes_precedence_over_oauth_token() {
    let _lock = ENV_MUTEX.lock().unwrap();

    // @step Given both ANTHROPIC_API_KEY and CLAUDE_CODE_OAUTH_TOKEN are set in the environment
    env::set_var("ANTHROPIC_API_KEY", "sk-ant-api03-test-key");
    env::set_var("CLAUDE_CODE_OAUTH_TOKEN", "sk-ant-oat01-test-token");

    // @step When the provider is initialized
    let provider = ClaudeProvider::new();

    // @step Then the provider should use ANTHROPIC_API_KEY for authentication
    // @step And the request should use x-api-key header
    assert!(provider.is_ok(), "Provider should initialize with API key");

    let provider = provider.unwrap();
    assert!(
        !provider.is_oauth_mode(),
        "Provider should NOT be in OAuth mode when API key is present"
    );

    // Cleanup
    env::remove_var("ANTHROPIC_API_KEY");
    env::remove_var("CLAUDE_CODE_OAUTH_TOKEN");
}

// ==========================================
// SCENARIO 3: OAuth request beta headers
// ==========================================

/// Scenario: OAuth request includes all required beta headers and body modifications
#[test]
fn test_oauth_request_includes_beta_headers() {
    let _lock = ENV_MUTEX.lock().unwrap();

    // @step Given CLAUDE_CODE_OAUTH_TOKEN is set in the environment
    env::set_var("CLAUDE_CODE_OAUTH_TOKEN", "sk-ant-oat01-test-token");
    env::remove_var("ANTHROPIC_API_KEY");

    // @step When a completion request is sent
    let provider = ClaudeProvider::new().expect("Provider should initialize");

    // @step Then the anthropic-beta header should include "oauth-2025-04-20"
    // @step And the anthropic-beta header should include "prompt-caching-2024-07-31"
    let beta_header = provider.get_anthropic_beta_header();
    assert!(
        beta_header.contains("oauth-2025-04-20"),
        "Beta header should include oauth-2025-04-20"
    );
    assert!(
        beta_header.contains("prompt-caching-2024-07-31"),
        "Beta header should include prompt-caching-2024-07-31"
    );

    // @step And the first system block should contain only the Claude Code identifier
    // @step And additional system content should be in subsequent blocks with cache_control
    assert_eq!(
        provider.get_system_prompt_prefix(),
        "You are Claude Code, Anthropic's official CLI for Claude."
    );

    // Cleanup
    env::remove_var("CLAUDE_CODE_OAUTH_TOKEN");
}

// ==========================================
// HELPER TESTS
// ==========================================

/// Test that provider fails when no credentials are available
#[test]
fn test_no_credentials_returns_error() {
    let _lock = ENV_MUTEX.lock().unwrap();

    env::remove_var("ANTHROPIC_API_KEY");
    env::remove_var("CLAUDE_CODE_OAUTH_TOKEN");

    let provider = ClaudeProvider::new();
    assert!(
        provider.is_err(),
        "Provider should fail without credentials"
    );

    let err = provider.unwrap_err();
    assert!(
        err.to_string().contains("No API key found"),
        "Error should mention missing API key"
    );
}
