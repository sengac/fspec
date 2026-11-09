//! Feature: spec/features/move-credential-management-to-rust.feature
//!
//! This test module validates the acceptance criteria for CONFIG-005:
//! Move Credential Management to Rust, against codelet_sessions::credentials
//! (moved here from the removed codelet-napi crate in REFAC-027).
//!
//! Tests map directly to Gherkin scenarios in the feature file.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_sessions::credentials::{
    extract_provider_from_model, get_disk_read_count, init_credential_store_with_dir,
    refresh_credentials_on_resume, reset_credential_store, reset_disk_read_count,
    resolve_credential, resolve_credential_for_session,
};
use serial_test::serial;
use std::env;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Helper to create a temporary credentials directory with a credentials file
fn setup_credentials_file(data_dir: &Path, provider: &str, api_key: &str) {
    let cred_dir = data_dir.join("credentials");
    fs::create_dir_all(&cred_dir).unwrap();
    let cred_file = cred_dir.join("credentials.json");
    let content = format!(
        r#"{{
            "version": 1,
            "providers": {{
                "{provider}": {{
                    "apiKey": "{api_key}",
                    "lastUpdated": "2026-02-10T00:00:00Z"
                }}
            }}
        }}"#
    );
    fs::write(&cred_file, content).unwrap();
}

/// TEST-042: replace the credentials file with `api_key` and move its mtime
/// a minute past the previous one, so a reload sees a change without a sleep
/// (whatever the file system's mtime granularity).
fn rewrite_credentials_file_later(data_dir: &Path, provider: &str, api_key: &str) {
    let cred_file = data_dir.join("credentials").join("credentials.json");
    let previous = fs::metadata(&cred_file)
        .and_then(|metadata| metadata.modified())
        .unwrap();
    setup_credentials_file(data_dir, provider, api_key);
    fs::File::options()
        .write(true)
        .open(&cred_file)
        .and_then(|file| file.set_modified(previous + std::time::Duration::from_secs(60)))
        .unwrap();
}

/// Helper to create an empty credentials file
fn setup_empty_credentials_file(data_dir: &Path) {
    let cred_dir = data_dir.join("credentials");
    fs::create_dir_all(&cred_dir).unwrap();
    let cred_file = cred_dir.join("credentials.json");
    fs::write(&cred_file, r#"{"version": 1, "providers": {}}"#).unwrap();
}

/// Helper to create a .env file with an API key
fn setup_dotenv_file(project_dir: &Path, env_var: &str, api_key: &str) {
    let env_file = project_dir.join(".env");
    let content = format!("{env_var}={api_key}\n");
    fs::write(&env_file, content).unwrap();
}

/// Helper to clear all anthropic-related environment variables
fn clear_anthropic_env_vars() {
    env::remove_var("ANTHROPIC_API_KEY");
    env::remove_var("CLAUDE_CODE_OAUTH_TOKEN");
}

#[path = "credential_resolution/caching_and_resume.rs"]
mod caching_and_resume;
#[path = "credential_resolution/oauth_detection.rs"]
mod oauth_detection;
#[path = "credential_resolution/priority_chain.rs"]
mod priority_chain;
