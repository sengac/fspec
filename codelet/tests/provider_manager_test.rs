
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/provider-manager-for-dynamic-provider-selection.feature
//!
//! Tests for ProviderManager implementation following ACDD workflow.
//! Each test maps to a Gherkin scenario with @step comments.

use codelet::providers::ProviderManager;
use std::env;
use std::fs;
use std::path::PathBuf;

/// Helper to get test codex home directory
fn test_codex_home() -> PathBuf {
    let temp_dir = std::env::temp_dir();
    temp_dir.join("codelet_test_provider_manager_codex")
}

/// Helper to clean up test environment
fn cleanup_test_env() {
    // Remove test directories
    let _ = fs::remove_dir_all(test_codex_home());

    // Remove all provider-related env vars
    env::remove_var("ANTHROPIC_API_KEY");
    env::remove_var("CLAUDE_CODE_OAUTH_TOKEN");
    env::remove_var("OPENAI_API_KEY");
    env::remove_var("GOOGLE_GENERATIVE_AI_API_KEY");
    env::remove_var("CODEX_HOME");
    env::remove_var("CLAUDE_HOME");
}

/// Helper to set up Codex auth.json for testing
fn setup_codex_auth() {
    let codex_home = test_codex_home();
    fs::create_dir_all(&codex_home).expect("Failed to create test codex home");

    let auth_content = r#"{
        "OPENAI_API_KEY": "sk-proj-test-key-12345"
    }"#;

    fs::write(codex_home.join("auth.json"), auth_content).expect("Failed to write codex auth.json");
    env::set_var("CODEX_HOME", codex_home.to_str().unwrap());
}

#[test]
fn test_automatic_claude_provider_selection_with_api_key() {
    // @step Given the ANTHROPIC_API_KEY environment variable is set
    cleanup_test_env();
    env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-12345");

    // @step And no other provider credentials are available
    // (cleanup ensured this)

    // @step When I run codelet with prompt "Who made you?"
    let manager = ProviderManager::new();
    assert!(
        manager.is_ok(),
        "ProviderManager should initialize successfully"
    );
    let manager = manager.unwrap();

    // @step Then the response should come from Claude provider
    assert_eq!(manager.current_provider_name(), "claude");

    // @step And the response should mention "Anthropic"
    // (this will be verified in integration tests with actual API calls)

    cleanup_test_env();
}

#[test]
fn test_explicit_codex_provider_selection_via_cli_flag() {
    // @step Given the ~/.codex/auth.json file exists with valid credentials
    cleanup_test_env();
    setup_codex_auth();

    // @step When I run codelet with --provider codex and prompt "Who made you?"
    let manager = ProviderManager::with_provider("codex");
    assert!(
        manager.is_ok(),
        "ProviderManager should initialize with codex provider"
    );
    let manager = manager.unwrap();

    // @step Then the response should come from Codex provider
    assert_eq!(manager.current_provider_name(), "codex");

    // @step And the response should mention "OpenAI"
    // (verified in integration tests)

    cleanup_test_env();
}

#[test]
fn test_error_when_no_credentials_available() {
    // @step Given no provider credentials are set
    cleanup_test_env();

    // @step And no auth files exist
    // (cleanup ensured this)

    // @step When I run codelet with prompt "Hello"
    let result = ProviderManager::new();

    // @step Then I should receive an error
    assert!(
        result.is_err(),
        "Should error when no credentials available"
    );

    // @step And the error message should contain "No provider credentials found"
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("No provider credentials found"),
        "Error should mention no credentials found: {}",
        error_msg
    );

    // @step And the error message should suggest "Set ANTHROPIC_API_KEY or run codex auth login"
    assert!(
        error_msg.contains("ANTHROPIC_API_KEY") || error_msg.contains("codex auth login"),
        "Error should suggest how to fix: {}",
        error_msg
    );

    cleanup_test_env();
}

#[test]
fn test_error_when_requested_provider_unavailable() {
    // @step Given only ANTHROPIC_API_KEY is set
    cleanup_test_env();
    env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-12345");

    // @step When I run codelet with --provider openai and prompt "Hello"
    let result = ProviderManager::with_provider("openai");

    // @step Then I should receive an error
    assert!(
        result.is_err(),
        "Should error when requested provider unavailable"
    );

    // @step And the error message should contain "Provider openai not available"
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("openai") && error_msg.contains("not available"),
        "Error should mention provider not available: {}",
        error_msg
    );

    // @step And the error message should list available providers as "claude"
    assert!(
        error_msg.contains("claude"),
        "Error should list available providers: {}",
        error_msg
    );

    cleanup_test_env();
}

#[test]
fn test_priority_based_provider_selection() {
    // @step Given both ANTHROPIC_API_KEY and ~/.codex/auth.json exist
    cleanup_test_env();
    env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-12345");
    setup_codex_auth();

    // @step When I run codelet without --provider flag and prompt "Who made you?"
    let manager = ProviderManager::new();
    assert!(
        manager.is_ok(),
        "ProviderManager should initialize successfully"
    );
    let manager = manager.unwrap();

    // @step Then the response should come from Claude provider
    assert_eq!(
        manager.current_provider_name(),
        "claude",
        "Should select Claude due to higher priority"
    );

    // @step And the Codex provider should not be used
    // (verified by checking current provider is claude, not codex)

    cleanup_test_env();
}

#[test]
fn test_claude_code_oauth_fallback() {
    // @step Given no ANTHROPIC_API_KEY is set
    cleanup_test_env();

    // @step And CLAUDE_CODE_OAUTH_TOKEN environment variable is set
    env::set_var("CLAUDE_CODE_OAUTH_TOKEN", "sk-ant-oat01-test-token");

    // @step And no other provider credentials are available
    // (cleanup ensured this)

    // @step When I run codelet with prompt "Who made you?"
    let manager = ProviderManager::new();
    assert!(
        manager.is_ok(),
        "ProviderManager should initialize with Claude OAuth"
    );
    let manager = manager.unwrap();

    // @step Then the response should come from Claude provider via OAuth
    assert_eq!(manager.current_provider_name(), "claude");

    // @step And the response should mention "Anthropic"
    // (verified in integration tests)

    cleanup_test_env();
}

#[test]
fn test_gemini_provider_selection() {
    // @step Given GOOGLE_GENERATIVE_AI_API_KEY environment variable is set
    cleanup_test_env();
    env::set_var("GOOGLE_GENERATIVE_AI_API_KEY", "test-gemini-key");

    // @step When I run codelet with --provider gemini and prompt "Hello"
    let manager = ProviderManager::with_provider("gemini");
    assert!(
        manager.is_ok(),
        "ProviderManager should initialize with Gemini"
    );
    let manager = manager.unwrap();

    // @step Then the response should come from Gemini provider
    assert_eq!(manager.current_provider_name(), "gemini");

    cleanup_test_env();
}

#[test]
fn test_openai_provider_selection() {
    // @step Given OPENAI_API_KEY environment variable is set
    cleanup_test_env();
    env::set_var("OPENAI_API_KEY", "sk-test-openai-key");

    // @step When I run codelet with --provider openai and prompt "Hello"
    let manager = ProviderManager::with_provider("openai");
    assert!(
        manager.is_ok(),
        "ProviderManager should initialize with OpenAI"
    );
    let manager = manager.unwrap();

    // @step Then the response should come from OpenAI provider
    assert_eq!(manager.current_provider_name(), "openai");

    cleanup_test_env();
}
