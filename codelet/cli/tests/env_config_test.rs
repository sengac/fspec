#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/environment-configuration-loading.feature
//!
//! Tests for Environment Configuration Loading - CORE-006
//!
//! These tests verify that .env files are loaded correctly on startup
//! and that environment variable precedence is respected.

use std::env;
use std::fs;
use std::sync::Mutex;
use tempfile::TempDir;

// Global mutex to ensure tests run serially (env vars are process-global)
static ENV_MUTEX: Mutex<()> = Mutex::new(());

// ==========================================
// SCENARIO 1: Load API key from .env file on startup
// ==========================================

/// Scenario: Load API key from .env file on startup
#[test]
fn test_load_api_key_from_env_file() {
    let _lock = ENV_MUTEX.lock().unwrap();

    // @step Given a .env file exists with CLAUDE_CODE_OAUTH_TOKEN set
    let temp_dir = TempDir::new().unwrap();
    let env_file = temp_dir.path().join(".env");
    fs::write(
        &env_file,
        "CLAUDE_CODE_OAUTH_TOKEN=test-token-from-env-file\n",
    )
    .unwrap();

    // @step And no environment variables are exported
    env::remove_var("CLAUDE_CODE_OAUTH_TOKEN");
    env::remove_var("ANTHROPIC_API_KEY");

    // @step When the user runs codelet
    // Change to temp dir and load .env (simulates startup behavior)
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();
    // dotenvy::dotenv() loads .env from current directory
    let result = dotenvy::dotenv();
    // Restore original directory
    env::set_current_dir(&original_dir).unwrap();

    // @step Then the agent should authenticate using the token from .env
    assert!(result.is_ok(), "dotenvy should load .env file successfully");
    assert_eq!(
        env::var("CLAUDE_CODE_OAUTH_TOKEN").unwrap(),
        "test-token-from-env-file",
        "Token should be loaded from .env file"
    );

    // Cleanup
    env::remove_var("CLAUDE_CODE_OAUTH_TOKEN");
}

// ==========================================
// SCENARIO 2: Run without .env file using exported environment variable
// ==========================================

/// Scenario: Run without .env file using exported environment variable
#[test]
fn test_run_without_env_file() {
    let _lock = ENV_MUTEX.lock().unwrap();

    // @step Given no .env file exists in the current directory
    let temp_dir = TempDir::new().unwrap();
    // Don't create .env file

    // @step And ANTHROPIC_API_KEY is exported in the shell
    env::set_var("ANTHROPIC_API_KEY", "exported-api-key");

    // @step When the user runs codelet
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();
    // dotenvy::dotenv() should silently fail when no .env exists
    let result = dotenvy::dotenv();
    env::set_current_dir(&original_dir).unwrap();

    // @step Then the agent should authenticate using the exported API key
    // dotenvy returns Err when no .env file, but that's fine - we ignore it
    assert!(
        result.is_err(),
        "dotenvy should return Err when no .env exists"
    );
    assert_eq!(
        env::var("ANTHROPIC_API_KEY").unwrap(),
        "exported-api-key",
        "Exported API key should still be available"
    );

    // Cleanup
    env::remove_var("ANTHROPIC_API_KEY");
}

// ==========================================
// SCENARIO 3: Exported environment variable takes precedence over .env
// ==========================================

/// Scenario: Exported environment variable takes precedence over .env
#[test]
fn test_exported_env_takes_precedence() {
    let _lock = ENV_MUTEX.lock().unwrap();

    // @step Given a .env file exists with ANTHROPIC_API_KEY=key-from-env-file
    let temp_dir = TempDir::new().unwrap();
    let env_file = temp_dir.path().join(".env");
    fs::write(&env_file, "ANTHROPIC_API_KEY=key-from-env-file\n").unwrap();

    // @step And ANTHROPIC_API_KEY=key-from-shell is exported
    env::set_var("ANTHROPIC_API_KEY", "key-from-shell");

    // @step When the user runs codelet
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();
    // dotenvy::dotenv() should NOT override existing env vars
    let _ = dotenvy::dotenv();
    env::set_current_dir(&original_dir).unwrap();

    // @step Then the agent should use key-from-shell, not key-from-env-file
    assert_eq!(
        env::var("ANTHROPIC_API_KEY").unwrap(),
        "key-from-shell",
        "Exported env var should take precedence over .env"
    );

    // Cleanup
    env::remove_var("ANTHROPIC_API_KEY");
}
