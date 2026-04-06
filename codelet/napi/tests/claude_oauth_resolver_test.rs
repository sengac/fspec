#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::needless_collect)]
// Feature: spec/features/claude-oauth-credential-detection-and-session-routing.feature
//
// PROV-026: Credential Resolver Integration Test
//
// Tests that resolver.rs correctly falls back to claude_auth.json
// for the anthropic provider, and sets CLAUDE_CODE_OAUTH_TOKEN env var.

use codelet_providers::claude_auth::{get_claude_auth_path, ClaudeAuthJson};
use serial_test::serial;
use std::env;
use std::fs;

/// Helper: create a test ClaudeAuthJson and write it to the expected location
fn setup_claude_auth(access: &str, refresh: &str, expires: u64) -> anyhow::Result<()> {
    let auth = ClaudeAuthJson {
        access_token: access.to_string(),
        refresh_token: refresh.to_string(),
        expires,
    };
    let auth_path = get_claude_auth_path();
    if let Some(parent) = auth_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(&auth)?;
    fs::write(&auth_path, content)?;
    Ok(())
}

/// Helper: save and clear Claude-related env vars, returning originals
fn save_and_clear_claude_env() -> (Option<String>, Option<String>) {
    let orig_api_key = env::var("ANTHROPIC_API_KEY").ok();
    let orig_oauth_token = env::var("CLAUDE_CODE_OAUTH_TOKEN").ok();
    env::remove_var("ANTHROPIC_API_KEY");
    env::remove_var("CLAUDE_CODE_OAUTH_TOKEN");
    (orig_api_key, orig_oauth_token)
}

/// Helper: restore Claude-related env vars
fn restore_claude_env(orig_api_key: Option<String>, orig_oauth_token: Option<String>) {
    if let Some(key) = orig_api_key {
        env::set_var("ANTHROPIC_API_KEY", key);
    }
    if let Some(token) = orig_oauth_token {
        env::set_var("CLAUDE_CODE_OAUTH_TOKEN", token);
    }
}

// ============================================================================
// Scenario: Credential resolver sets env var from OAuth tokens
// ============================================================================

#[test]
#[serial]
fn test_credential_resolver_finds_oauth_tokens_from_claude_auth() {
    use codelet_napi::credentials::{resolve_credential};

    // @step Given I have authenticated with Claude via OAuth
    let original_home = env::var("CODELET_HOME").ok();
    let temp_dir = tempfile::tempdir().unwrap();
    env::set_var("CODELET_HOME", temp_dir.path());

    // @step And claude_auth.json exists with access_token starting with sk-ant-oat
    setup_claude_auth("sk-ant-oat-resolver-test", "sk-ant-ort-resolver-refresh", 9999999999999).unwrap();

    // @step And no ANTHROPIC_API_KEY or CLAUDE_CODE_OAUTH_TOKEN env vars exist
    let (orig_api_key, orig_oauth_token) = save_and_clear_claude_env();

    // @step And no credentials are stored in credentials.json for anthropic
    // (temp dir has no credentials.json)

    // @step When the credential resolver resolves credentials for anthropic
    let result = resolve_credential("anthropic", None, None);

    // @step Then the access_token from claude_auth.json should be returned
    assert!(result.is_ok());
    let credential = result.unwrap();
    assert!(credential.is_some(), "Resolver should find OAuth tokens from claude_auth.json");
    assert_eq!(credential.unwrap(), "sk-ant-oat-resolver-test");

    // Restore
    restore_claude_env(orig_api_key, orig_oauth_token);
    if let Some(home) = original_home { env::set_var("CODELET_HOME", home); } else { env::remove_var("CODELET_HOME"); }
}

#[test]
#[serial]
fn test_credential_resolver_sets_claude_code_oauth_token_env_var() {
    use codelet_napi::credentials::resolve_and_set_env_var;

    // @step Given I have authenticated with Claude via OAuth
    let original_home = env::var("CODELET_HOME").ok();
    let temp_dir = tempfile::tempdir().unwrap();
    env::set_var("CODELET_HOME", temp_dir.path());

    // @step And claude_auth.json exists with access_token starting with sk-ant-oat
    setup_claude_auth("sk-ant-oat-resolver-env-test", "sk-ant-ort-resolver-refresh", 9999999999999).unwrap();

    // @step And no ANTHROPIC_API_KEY or CLAUDE_CODE_OAUTH_TOKEN env vars exist
    let (orig_api_key, orig_oauth_token) = save_and_clear_claude_env();

    // @step When resolve_and_set_env_var is called for anthropic
    let set_result = resolve_and_set_env_var("anthropic", None);

    // @step Then CLAUDE_CODE_OAUTH_TOKEN should be set as the environment variable
    assert!(set_result.is_ok());
    assert!(set_result.unwrap(), "resolve_and_set_env_var should return true");
    // @step And CLAUDE_CODE_OAUTH_TOKEN should be set as the environment variable
    assert_eq!(
        env::var("CLAUDE_CODE_OAUTH_TOKEN").ok(),
        Some("sk-ant-oat-resolver-env-test".to_string()),
        "CLAUDE_CODE_OAUTH_TOKEN should be set with the OAuth access token"
    );

    // Restore
    restore_claude_env(orig_api_key, orig_oauth_token);
    if let Some(home) = original_home { env::set_var("CODELET_HOME", home); } else { env::remove_var("CODELET_HOME"); }
}

#[test]
#[serial]
fn test_resolver_does_not_use_claude_auth_for_non_anthropic_providers() {
    use codelet_napi::credentials::resolve_credential;

    // @step Given claude_auth.json exists with valid OAuth tokens
    let original_home = env::var("CODELET_HOME").ok();
    let temp_dir = tempfile::tempdir().unwrap();
    env::set_var("CODELET_HOME", temp_dir.path());
    setup_claude_auth("sk-ant-oat-test", "sk-ant-ort-test", 9999999999999).unwrap();

    // @step And no OpenAI env vars exist
    let orig_openai = env::var("OPENAI_API_KEY").ok();
    env::remove_var("OPENAI_API_KEY");

    // @step When the credential resolver resolves credentials for openai
    let result = resolve_credential("openai", None, None);

    // @step Then no credential should be found (claude_auth.json is only for anthropic)
    assert!(result.is_ok());
    assert!(result.unwrap().is_none(), "Claude OAuth tokens should not be returned for openai provider");

    // Restore
    if let Some(key) = orig_openai { env::set_var("OPENAI_API_KEY", key); } else { env::remove_var("OPENAI_API_KEY"); }
    if let Some(home) = original_home { env::set_var("CODELET_HOME", home); } else { env::remove_var("CODELET_HOME"); }
}
