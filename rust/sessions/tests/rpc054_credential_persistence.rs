//! Feature: spec/features/provider-credential-persistence.feature
//!
//! RPC-054 (reopened 2026-06-19): the provider screen must actually persist
//! API-key credentials to <data_dir>/credentials/credentials.json, mirroring
//! the TypeScript `saveCredential` / `deleteCredential` write path. These
//! tests exercise the new `save_credential_with_dir` / `delete_credential_with_dir`
//! writer functions directly against a temp directory — fully offline, no
//! network, no $HOME mutation.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_sessions::credentials::{
    delete_credential_with_dir, save_credential_with_dir, CredentialStore, CredentialsFile,
};
use serial_test::serial;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn cred_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("credentials").join("credentials.json")
}

fn read_file(data_dir: &Path) -> CredentialsFile {
    let content = fs::read_to_string(cred_path(data_dir)).expect("credentials.json should exist");
    serde_json::from_str(&content).expect("credentials.json should be valid JSON")
}

fn seed(data_dir: &Path, json: &str) {
    let dir = data_dir.join("credentials");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("credentials.json"), json).unwrap();
}

#[test]
fn save_api_key_on_machine_with_no_credentials_file() {
    // @step Given FSPEC_USER_DIR points at an empty temp directory with no credentials.json
    let temp = TempDir::new().unwrap();
    let data_dir = temp.path();
    assert!(!cred_path(data_dir).exists());

    // @step When set_provider_credentials is called for "mistral" with an api_key input "sk-test-123"
    save_credential_with_dir(data_dir, "mistral", "sk-test-123").unwrap();

    // @step Then credentials.json is created at <FSPEC_USER_DIR>/credentials/credentials.json
    assert!(cred_path(data_dir).exists());

    // @step And the file contains version 1 and providers.mistral.apiKey equal to "sk-test-123"
    let file = read_file(data_dir);
    assert_eq!(file.version, 1);
    assert_eq!(
        file.providers.get("mistral").unwrap().api_key,
        "sk-test-123"
    );

    // @step And providers.mistral.lastUpdated is a non-empty ISO-8601 timestamp
    let raw = fs::read_to_string(cred_path(data_dir)).unwrap();
    assert!(raw.contains("lastUpdated"));
    assert!(raw.contains('T') && raw.contains('Z') || raw.contains('+'));

    // @step And on unix the file mode is 0600 and the credentials directory mode is 0700
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let file_mode = fs::metadata(cred_path(data_dir))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        let dir_mode = fs::metadata(data_dir.join("credentials"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(file_mode, 0o600, "file should be 0600");
        assert_eq!(dir_mode, 0o700, "dir should be 0700");
    }
}

#[test]
fn saving_new_provider_preserves_existing_entries() {
    // @step Given credentials.json already contains an "openai" provider entry
    let temp = TempDir::new().unwrap();
    let data_dir = temp.path();
    seed(
        data_dir,
        r#"{"version":1,"providers":{"openai":{"apiKey":"oa-key","lastUpdated":"2026-02-10T00:00:00Z"}}}"#,
    );

    // @step When set_provider_credentials is called for "groq" with an api_key input "gk-123"
    save_credential_with_dir(data_dir, "groq", "gk-123").unwrap();

    // @step Then providers.groq.apiKey equals "gk-123"
    let file = read_file(data_dir);
    assert_eq!(file.providers.get("groq").unwrap().api_key, "gk-123");

    // @step And the existing providers.openai entry is unchanged
    assert_eq!(file.providers.get("openai").unwrap().api_key, "oa-key");

    // @step And the version field is still 1
    assert_eq!(file.version, 1);
}

#[test]
fn saving_same_provider_again_replaces_key_in_place() {
    // @step Given credentials.json contains providers.mistral with apiKey "old-key"
    let temp = TempDir::new().unwrap();
    let data_dir = temp.path();
    seed(
        data_dir,
        r#"{"version":1,"providers":{"mistral":{"apiKey":"old-key","lastUpdated":"2026-02-10T00:00:00Z"}}}"#,
    );

    // @step When set_provider_credentials is called for "mistral" with an api_key input "new-key"
    save_credential_with_dir(data_dir, "mistral", "new-key").unwrap();

    // @step Then providers.mistral.apiKey equals "new-key"
    let file = read_file(data_dir);
    assert_eq!(file.providers.get("mistral").unwrap().api_key, "new-key");

    // @step And providers.mistral.lastUpdated is refreshed
    assert_ne!(
        file.providers
            .get("mistral")
            .unwrap()
            .last_updated
            .to_rfc3339(),
        "2026-02-10T00:00:00+00:00"
    );

    // @step And there is exactly one "mistral" entry under providers
    assert_eq!(file.providers.len(), 1);
}

#[test]
#[serial]
fn saved_credential_is_immediately_readable_through_store() {
    // @step Given FSPEC_USER_DIR points at an empty temp directory
    let temp = TempDir::new().unwrap();
    let data_dir = temp.path();

    // @step When set_provider_credentials is called for "mistral" with an api_key input "sk-test-123"
    save_credential_with_dir(data_dir, "mistral", "sk-test-123").unwrap();

    // @step Then a credential store reading that directory returns "sk-test-123" for "mistral"
    let mut store = CredentialStore::new(data_dir).unwrap();
    assert_eq!(
        store.get_api_key("mistral").unwrap(),
        Some("sk-test-123".to_string())
    );
}

#[test]
fn deleting_one_provider_leaves_others_intact() {
    // @step Given credentials.json contains both "mistral" and "groq" provider entries
    let temp = TempDir::new().unwrap();
    let data_dir = temp.path();
    seed(
        data_dir,
        r#"{"version":1,"providers":{"mistral":{"apiKey":"m","lastUpdated":"2026-02-10T00:00:00Z"},"groq":{"apiKey":"g","lastUpdated":"2026-02-10T00:00:00Z"}}}"#,
    );

    // @step When delete_provider_credentials is called for "mistral"
    delete_credential_with_dir(data_dir, "mistral").unwrap();

    // @step Then providers.mistral is removed from credentials.json
    let file = read_file(data_dir);
    assert!(!file.providers.contains_key("mistral"));

    // @step And the "groq" entry and the version field remain
    assert_eq!(file.providers.get("groq").unwrap().api_key, "g");
    assert_eq!(file.version, 1);

    // @step And get_stored_api_key_with_dir("mistral", <FSPEC_USER_DIR>) returns None
    let mut store = CredentialStore::new(data_dir).unwrap();
    assert_eq!(store.get_api_key("mistral").unwrap(), None);
}

#[test]
fn deleting_last_provider_leaves_empty_providers_map() {
    // @step Given credentials.json contains only a "mistral" provider entry
    let temp = TempDir::new().unwrap();
    let data_dir = temp.path();
    seed(
        data_dir,
        r#"{"version":1,"providers":{"mistral":{"apiKey":"m","lastUpdated":"2026-02-10T00:00:00Z"}}}"#,
    );

    // @step When delete_provider_credentials is called for "mistral"
    delete_credential_with_dir(data_dir, "mistral").unwrap();

    // @step Then credentials.json still exists on disk
    assert!(cred_path(data_dir).exists());

    // @step And it contains version 1 and an empty providers map
    let file = read_file(data_dir);
    assert_eq!(file.version, 1);
    assert!(file.providers.is_empty());
}

#[test]
fn empty_api_key_is_rejected_and_nothing_written() {
    // @step Given FSPEC_USER_DIR points at an empty temp directory with no credentials.json
    let temp = TempDir::new().unwrap();
    let data_dir = temp.path();

    // @step When set_provider_credentials is called for "mistral" with an empty api_key input
    let result = save_credential_with_dir(data_dir, "mistral", "");

    // @step Then the call returns an error
    assert!(result.is_err());

    // @step And no credentials.json file is created
    assert!(!cred_path(data_dir).exists());
}

#[test]
fn deleting_absent_provider_is_successful_noop() {
    // @step Given credentials.json contains only a "groq" provider entry
    let temp = TempDir::new().unwrap();
    let data_dir = temp.path();
    seed(
        data_dir,
        r#"{"version":1,"providers":{"groq":{"apiKey":"g","lastUpdated":"2026-02-10T00:00:00Z"}}}"#,
    );

    // @step When delete_provider_credentials is called for "mistral"
    let result = delete_credential_with_dir(data_dir, "mistral");

    // @step Then the call succeeds
    assert!(result.is_ok());

    // @step And the "groq" entry is still present and unchanged
    let file = read_file(data_dir);
    assert_eq!(file.providers.get("groq").unwrap().api_key, "g");
}
