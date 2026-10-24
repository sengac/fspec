//! Shared filesystem and credential fixtures for OAuth integration tests.

#![allow(dead_code)]

use base64::Engine;

pub fn build_test_jwt(account_id: &str) -> String {
    let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(r#"{"typ":"JWT","alg":"none"}"#.as_bytes());
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(format!(r#"{{"chatgpt_account_id":"{account_id}"}}"#).as_bytes());
    format!("{header}.{payload}.stub_signature")
}

pub fn build_token_response_json(
    id_token: &str,
    access_token: &str,
    refresh_token: &str,
) -> serde_json::Value {
    serde_json::json!({
        "id_token": id_token,
        "access_token": access_token,
        "refresh_token": refresh_token,
        "expires_in": 3600
    })
}

pub struct CodexHomeGuard {
    original: Option<String>,
}

impl Drop for CodexHomeGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(value) => std::env::set_var("CODEX_HOME", value),
            None => std::env::remove_var("CODEX_HOME"),
        }
    }
}

pub fn setup_codex_home() -> std::io::Result<(tempfile::TempDir, CodexHomeGuard)> {
    let temp_dir = tempfile::TempDir::new()?;
    let guard = CodexHomeGuard {
        original: std::env::var("CODEX_HOME").ok(),
    };
    std::env::set_var("CODEX_HOME", temp_dir.path());
    Ok((temp_dir, guard))
}

pub struct FspecHomeGuard {
    original: Option<String>,
}

impl Drop for FspecHomeGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(value) => std::env::set_var("FSPEC_HOME", value),
            None => std::env::remove_var("FSPEC_HOME"),
        }
    }
}

pub fn setup_fspec_home() -> std::io::Result<(tempfile::TempDir, FspecHomeGuard)> {
    let temp_dir = tempfile::TempDir::new()?;
    let guard = FspecHomeGuard {
        original: std::env::var("FSPEC_HOME").ok(),
    };
    std::env::set_var("FSPEC_HOME", temp_dir.path());
    Ok((temp_dir, guard))
}
