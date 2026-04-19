//! Tests for CredentialStore<T> (PROV-060)
//!
//! Feature: spec/features/shared-oauth-building-blocks.feature
//! Scenario: Generic credential store reads and writes provider auth files

use crate::oauth::credential_store::CredentialStore;
use serde::{Deserialize, Serialize};

/// Minimal auth struct for testing (simulates Copilot)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct FakeCopilotAuth {
    github_oauth_token: String,
    copilot_token: Option<String>,
}

/// Minimal auth struct for testing (simulates Codex)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct FakeCodexAuth {
    openai_api_key: Option<String>,
    refresh_token: String,
}

/// Minimal auth struct for testing (simulates Claude)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct FakeClaudeAuth {
    access_token: String,
    refresh_token: String,
    expires: u64,
}

// @step Given a CredentialStore parameterized with a provider-specific auth type
// @step When credentials are written and then read back for Copilot, Codex, and Claude
// @step Then each provider's auth JSON file is correctly serialized and deserialized
// @step And the three separate read/write function pairs are replaced by the single generic implementation

#[tokio::test]
async fn credential_store_round_trips_copilot_auth() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("copilot_auth.json");

    // @step Given a CredentialStore parameterized with a provider-specific auth type
    let store = CredentialStore::<FakeCopilotAuth>::new(path);

    // @step When credentials are written and then read back for Copilot
    let original = FakeCopilotAuth {
        github_oauth_token: "gho_test_123".to_string(),
        copilot_token: Some("tid=abc".to_string()),
    };
    store.write(&original).await.unwrap();

    // @step Then each provider's auth JSON file is correctly serialized and deserialized
    let read_back = store.read().await.unwrap().unwrap();
    assert_eq!(read_back, original);
}

#[tokio::test]
async fn credential_store_round_trips_codex_auth() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("auth.json");

    // @step Given a CredentialStore parameterized with a provider-specific auth type
    let store = CredentialStore::<FakeCodexAuth>::new(path);

    // @step When credentials are written and then read back for Codex
    let original = FakeCodexAuth {
        openai_api_key: Some("sk-test".to_string()),
        refresh_token: "rt_codex_123".to_string(),
    };
    store.write(&original).await.unwrap();

    // @step Then each provider's auth JSON file is correctly serialized and deserialized
    let read_back = store.read().await.unwrap().unwrap();
    assert_eq!(read_back, original);
}

#[tokio::test]
async fn credential_store_round_trips_claude_auth() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("claude_auth.json");

    // @step Given a CredentialStore parameterized with a provider-specific auth type
    let store = CredentialStore::<FakeClaudeAuth>::new(path);

    // @step When credentials are written and then read back for Claude
    let original = FakeClaudeAuth {
        access_token: "at_claude_123".to_string(),
        refresh_token: "rt_claude_456".to_string(),
        expires: 1700000000,
    };
    store.write(&original).await.unwrap();

    // @step Then each provider's auth JSON file is correctly serialized and deserialized
    let read_back = store.read().await.unwrap().unwrap();
    assert_eq!(read_back, original);
}

#[tokio::test]
async fn credential_store_returns_none_for_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nonexistent.json");
    let store = CredentialStore::<FakeCopilotAuth>::new(path);
    let result = store.read().await.unwrap();
    assert!(result.is_none());
}

#[test]
fn credential_store_sync_read_write() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sync_test.json");
    let store = CredentialStore::<FakeClaudeAuth>::new(path);

    let original = FakeClaudeAuth {
        access_token: "sync_at".to_string(),
        refresh_token: "sync_rt".to_string(),
        expires: 42,
    };
    store.write_sync(&original).unwrap();
    let read_back = store.read_sync().unwrap().unwrap();
    assert_eq!(read_back, original);
}

#[tokio::test]
async fn credential_store_delete_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("delete_test.json");
    let store = CredentialStore::<FakeCopilotAuth>::new(path.clone());

    // Delete non-existent — should not error
    store.delete().await.unwrap();

    // Write then delete
    let auth = FakeCopilotAuth {
        github_oauth_token: "gho_del".to_string(),
        copilot_token: None,
    };
    store.write(&auth).await.unwrap();
    assert!(path.exists());
    store.delete().await.unwrap();
    assert!(!path.exists());
}

#[cfg(unix)]
#[tokio::test]
async fn credential_store_write_secure_enforces_0600() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("secure_test.json");
    let store = CredentialStore::<FakeCopilotAuth>::new(path.clone());

    let auth = FakeCopilotAuth {
        github_oauth_token: "gho_secure".to_string(),
        copilot_token: None,
    };
    store.write_secure(&auth).await.unwrap();

    let mode = std::fs::metadata(&path).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
}

#[tokio::test]
async fn credential_store_creates_parent_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sub").join("dir").join("auth.json");
    let store = CredentialStore::<FakeCodexAuth>::new(path.clone());

    let auth = FakeCodexAuth {
        openai_api_key: None,
        refresh_token: "nested".to_string(),
    };
    store.write(&auth).await.unwrap();
    assert!(path.exists());
}
