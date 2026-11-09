#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/provider-manager-for-dynamic-provider-selection.feature
//!
//! Tests for ProviderManager implementation following ACDD workflow.
//! Each test maps to a Gherkin scenario with @step comments.

use codelet_providers::ProviderManager;
use serial_test::serial;
use std::env;

/// Helper to clean up test environment
fn cleanup_test_env() {
    // Remove all provider-related env vars
    env::remove_var("ANTHROPIC_API_KEY");
    env::remove_var("CLAUDE_CODE_OAUTH_TOKEN");
    env::remove_var("OPENAI_API_KEY");
    env::remove_var("GOOGLE_GENERATIVE_AI_API_KEY");
    // BUG-306: point file-based credential sources at an empty directory so
    // the machine's own Codex/Claude/Copilot logins never leak into the test.
    static EMPTY_HOME: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();
    let empty = EMPTY_HOME
        .get_or_init(|| tempfile::tempdir().expect("empty credential home"))
        .path();
    env::set_var("CODEX_HOME", empty);
    env::set_var("CLAUDE_HOME", empty);
    env::set_var("FSPEC_HOME", empty);
    env::remove_var("ZAI_API_KEY");
    env::remove_var("ZAI_PLAN_API_KEY");
}

#[test]
#[serial]
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
        "ProviderManager should initialize successfully: {:?}",
        manager.as_ref().err()
    );
    let manager = manager.unwrap();

    // @step Then the response should come from Claude provider
    assert_eq!(manager.current_provider_name(), "claude");

    // @step And the response should mention "Anthropic"
    // (this will be verified in integration tests with actual API calls)

    cleanup_test_env();
}

#[test]
#[serial]
fn test_priority_based_provider_selection() {
    // @step Given both ANTHROPIC_API_KEY and OPENAI_API_KEY exist
    cleanup_test_env();
    env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-12345");
    env::set_var("OPENAI_API_KEY", "sk-test-openai-key");

    // @step When I run codelet without --provider flag and prompt "Who made you?"
    let manager = ProviderManager::new();

    // @step Then the response should come from Claude provider
    // BUG-306: PROV-101 removed the silent priority default; with several
    // credentialed providers the caller must select one explicitly
    // (spec/features/provider-resolution-no-silent-default.feature).
    let error = manager.expect_err("ambiguous credentials must not pick a provider");
    assert!(
        error.to_string().contains("none was explicitly selected"),
        "{error}"
    );

    // @step And the OpenAI provider should not be used
    // (no provider is used until one is selected explicitly)

    cleanup_test_env();
}

#[test]
#[serial]
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
        "ProviderManager should initialize with Claude OAuth: {:?}",
        manager.as_ref().err()
    );
    let manager = manager.unwrap();

    // @step Then the response should come from Claude provider via OAuth
    // The credential homes point at an empty directory (cleanup_test_env),
    // so the machine's own Codex login is never counted (BUG-548).
    assert_eq!(manager.current_provider_name(), "claude");

    // @step And the response should mention "Anthropic"
    // (verified in integration tests)

    cleanup_test_env();
}

#[test]
#[serial]
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
