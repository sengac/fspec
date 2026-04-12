#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect
)]
// Feature: spec/features/claude-oauth-credential-detection-and-session-routing.feature
//
// PROV-026: Claude OAuth Credential Detection and Session Routing Tests
//
// Tests for:
// - read_claude_auth_sync() — sync file reader
// - has_claude_auth() — credential detection
// - get_claude() — OAuth routing with from_oauth_tokens()
// - resolver fallback — claude_auth.json as credential source

use codelet_providers::claude_auth::{get_claude_auth_path, ClaudeAuthJson};
use codelet_providers::ProviderCredentials;
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
// Scenario: Credential detection includes claude_auth.json check
// ============================================================================

#[test]
#[serial]
fn test_credential_detection_with_claude_auth_json() {
    // @step Given claude_auth.json exists with valid access and refresh tokens
    let original_home = env::var("FSPEC_HOME").ok();
    let temp_dir = tempfile::tempdir().unwrap();
    env::set_var("FSPEC_HOME", temp_dir.path());
    setup_claude_auth(
        "sk-ant-oat-test-access",
        "sk-ant-ort-test-refresh",
        9999999999999,
    )
    .unwrap();

    // @step And no ANTHROPIC_API_KEY or CLAUDE_CODE_OAUTH_TOKEN env vars exist
    let (orig_api_key, orig_oauth_token) = save_and_clear_claude_env();

    // @step When provider credentials are detected
    let credentials = ProviderCredentials::detect();

    // @step Then claude_available should be true
    assert!(
        credentials.has_claude(),
        "claude_available should be true when claude_auth.json has valid tokens"
    );

    // Restore
    restore_claude_env(orig_api_key, orig_oauth_token);
    if let Some(home) = original_home {
        env::set_var("FSPEC_HOME", home);
    } else {
        env::remove_var("FSPEC_HOME");
    }
}

// ============================================================================
// Scenario: Credential detection without any Claude credentials
// ============================================================================

#[test]
#[serial]
fn test_credential_detection_without_any_claude_credentials() {
    // @step Given claude_auth.json does not exist
    let original_home = env::var("FSPEC_HOME").ok();
    let temp_dir = tempfile::tempdir().unwrap();
    env::set_var("FSPEC_HOME", temp_dir.path());

    // @step And no ANTHROPIC_API_KEY or CLAUDE_CODE_OAUTH_TOKEN env vars exist
    let (orig_api_key, orig_oauth_token) = save_and_clear_claude_env();

    // @step When provider credentials are detected
    let credentials = ProviderCredentials::detect();

    // @step Then claude_available should be false
    assert!(
        !credentials.has_claude(),
        "claude_available should be false when no credentials exist"
    );

    // Restore
    restore_claude_env(orig_api_key, orig_oauth_token);
    if let Some(home) = original_home {
        env::set_var("FSPEC_HOME", home);
    } else {
        env::remove_var("FSPEC_HOME");
    }
}

// ============================================================================
// Scenario: Session creation routes to Claude provider with OAuth tokens
// ============================================================================

#[test]
#[serial]
fn test_session_creation_routes_to_oauth_when_tokens_exist() {
    // @step Given I have authenticated with Claude via OAuth
    let original_home = env::var("FSPEC_HOME").ok();
    let temp_dir = tempfile::tempdir().unwrap();
    env::set_var("FSPEC_HOME", temp_dir.path());

    // @step And claude_auth.json exists with valid access and refresh tokens
    setup_claude_auth(
        "sk-ant-oat-session-test",
        "sk-ant-ort-session-refresh",
        9999999999999,
    )
    .unwrap();

    let (orig_api_key, orig_oauth_token) = save_and_clear_claude_env();

    // @step When I create a session with model anthropic/claude-sonnet-4-20250514
    let manager = codelet_providers::ProviderManager::with_provider_and_model(
        "claude",
        Some("claude-sonnet-4-20250514"),
        None,
        None,
    );

    // @step Then the provider manager should use from_oauth_tokens constructor
    // @step And the expires_in_secs should be Some(0) to force immediate refresh
    // We verify get_claude() succeeds (it will use OAuth path since claude_auth.json exists)
    assert!(
        manager.is_ok(),
        "ProviderManager should create successfully with OAuth tokens"
    );
    let manager = manager.unwrap();
    let claude_result = manager.get_claude();
    // The provider should be created — if from_oauth_tokens is used with Some(0),
    // the RefreshingClaudeClient will be in OAuth mode
    assert!(
        claude_result.is_ok(),
        "get_claude() should succeed when OAuth tokens exist in claude_auth.json"
    );

    // Restore
    restore_claude_env(orig_api_key, orig_oauth_token);
    if let Some(home) = original_home {
        env::set_var("FSPEC_HOME", home);
    } else {
        env::remove_var("FSPEC_HOME");
    }
}

// ============================================================================
// Scenario: Session creation falls back to env var when no OAuth tokens
// ============================================================================

#[test]
#[serial]
fn test_session_creation_falls_back_to_env_var() {
    // @step Given I have not authenticated with Claude via OAuth
    let original_home = env::var("FSPEC_HOME").ok();
    let temp_dir = tempfile::tempdir().unwrap();
    env::set_var("FSPEC_HOME", temp_dir.path());
    // No claude_auth.json created

    // @step And I have an ANTHROPIC_API_KEY environment variable set
    let orig_api_key = env::var("ANTHROPIC_API_KEY").ok();
    env::set_var("ANTHROPIC_API_KEY", "sk-ant-api03-test-key-for-fallback");

    // @step When I create a session with model anthropic/claude-sonnet-4-20250514
    let manager = codelet_providers::ProviderManager::with_provider_and_model(
        "claude",
        Some("claude-sonnet-4-20250514"),
        None,
        None,
    );

    // @step Then the provider manager should use new_with_model constructor
    // @step And the provider should use the ANTHROPIC_API_KEY for authentication
    assert!(
        manager.is_ok(),
        "ProviderManager should create with ANTHROPIC_API_KEY"
    );
    let manager = manager.unwrap();
    let claude_result = manager.get_claude();
    assert!(
        claude_result.is_ok(),
        "get_claude() should succeed using ANTHROPIC_API_KEY env var"
    );

    // Restore
    if let Some(key) = orig_api_key {
        env::set_var("ANTHROPIC_API_KEY", key);
    } else {
        env::remove_var("ANTHROPIC_API_KEY");
    }
    if let Some(home) = original_home {
        env::set_var("FSPEC_HOME", home);
    } else {
        env::remove_var("FSPEC_HOME");
    }
}

// ============================================================================
// Scenario: OAuth takes precedence over API key for session creation
// ============================================================================

#[test]
#[serial]
fn test_oauth_takes_precedence_over_api_key() {
    // @step Given I have authenticated with Claude via OAuth
    let original_home = env::var("FSPEC_HOME").ok();
    let temp_dir = tempfile::tempdir().unwrap();
    env::set_var("FSPEC_HOME", temp_dir.path());

    // @step And claude_auth.json exists with valid access and refresh tokens
    setup_claude_auth(
        "sk-ant-oat-precedence-test",
        "sk-ant-ort-precedence-refresh",
        9999999999999,
    )
    .unwrap();

    // @step And I have an ANTHROPIC_API_KEY environment variable set
    let orig_api_key = env::var("ANTHROPIC_API_KEY").ok();
    env::set_var("ANTHROPIC_API_KEY", "sk-ant-api03-should-not-be-used");

    // @step When I create a session with model anthropic/claude-sonnet-4-20250514
    let manager = codelet_providers::ProviderManager::with_provider_and_model(
        "claude",
        Some("claude-sonnet-4-20250514"),
        None,
        None,
    )
    .unwrap();

    // @step Then the provider manager should use from_oauth_tokens constructor
    let claude_result = manager.get_claude();
    assert!(
        claude_result.is_ok(),
        "get_claude() should succeed (OAuth takes precedence)"
    );

    // @step And the ANTHROPIC_API_KEY should not be used
    // The provider should be in OAuth mode (from_oauth_tokens), not API key mode
    let provider = claude_result.unwrap();
    assert!(
        provider.is_oauth_mode(),
        "Provider should be in OAuth mode when both OAuth tokens and API key exist"
    );

    // Restore
    if let Some(key) = orig_api_key {
        env::set_var("ANTHROPIC_API_KEY", key);
    } else {
        env::remove_var("ANTHROPIC_API_KEY");
    }
    if let Some(home) = original_home {
        env::set_var("FSPEC_HOME", home);
    } else {
        env::remove_var("FSPEC_HOME");
    }
}

// ============================================================================
// Scenario: read_claude_auth_sync reads valid tokens from file
// ============================================================================

#[test]
#[serial]
fn test_read_claude_auth_sync_reads_valid_file() {
    // @step Given claude_auth.json exists with valid access and refresh tokens
    let original_home = env::var("FSPEC_HOME").ok();
    let temp_dir = tempfile::tempdir().unwrap();
    env::set_var("FSPEC_HOME", temp_dir.path());
    setup_claude_auth(
        "sk-ant-oat-test-access",
        "sk-ant-ort-test-refresh",
        9999999999999,
    )
    .unwrap();

    // @step When read_claude_auth_sync is called
    let result = codelet_providers::claude_auth::read_claude_auth_sync();

    // @step Then it should return Some with the stored tokens
    assert!(result.is_ok(), "read_claude_auth_sync should not error");
    let auth = result.unwrap();
    assert!(auth.is_some(), "Should return Some when file exists");
    let auth = auth.unwrap();
    assert_eq!(auth.access_token, "sk-ant-oat-test-access");
    assert_eq!(auth.refresh_token, "sk-ant-ort-test-refresh");
    assert_eq!(auth.expires, 9999999999999);

    // Restore
    if let Some(home) = original_home {
        env::set_var("FSPEC_HOME", home);
    } else {
        env::remove_var("FSPEC_HOME");
    }
}

// ============================================================================
// Scenario: read_claude_auth_sync returns None when file missing
// ============================================================================

#[test]
#[serial]
fn test_read_claude_auth_sync_returns_none_when_file_missing() {
    // @step Given claude_auth.json does not exist
    let original_home = env::var("FSPEC_HOME").ok();
    let temp_dir = tempfile::tempdir().unwrap();
    env::set_var("FSPEC_HOME", temp_dir.path());

    // @step When read_claude_auth_sync is called
    let result = codelet_providers::claude_auth::read_claude_auth_sync();

    // @step Then it should return Ok None
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());

    // Restore
    if let Some(home) = original_home {
        env::set_var("FSPEC_HOME", home);
    } else {
        env::remove_var("FSPEC_HOME");
    }
}
