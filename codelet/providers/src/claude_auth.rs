//! Claude OAuth Authentication Persistence Module (PROV-021, PROV-026)
//!
//! Handles reading/writing Claude OAuth credentials to
//! ~/.config/codelet/claude_auth.json.
//!
//! Mirrors codex_auth.rs pattern but simpler — no keychain support,
//! no id_token, no account_id.
//!
//! Provides both async (tokio::fs) and sync (std::fs) readers:
//! - read_claude_auth() — async, for NAPI bindings
//! - read_claude_auth_sync() — sync, for credentials.rs detection and manager.rs routing

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Claude auth credentials — used for both persistence and as the return type.
///
/// Unlike Codex, there is no `id_token` or `account_id` — just
/// access/refresh tokens and an expiry timestamp (milliseconds).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeAuthJson {
    pub access_token: String,
    pub refresh_token: String,
    /// Expiry timestamp in milliseconds since Unix epoch
    pub expires: u64,
}

/// Get the codelet config directory
/// Uses CODELET_HOME env var if set, otherwise defaults to ~/.config/codelet
fn get_codelet_home() -> PathBuf {
    if let Ok(codelet_home) = std::env::var("CODELET_HOME") {
        PathBuf::from(codelet_home)
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/tmp"));
        PathBuf::from(home).join(".config").join("codelet")
    }
}

/// Get the path to claude_auth.json
pub fn get_claude_auth_path() -> PathBuf {
    get_codelet_home().join("claude_auth.json")
}

/// Read Claude auth credentials from file (async)
pub async fn read_claude_auth() -> Result<Option<ClaudeAuthJson>> {
    let auth_path = get_claude_auth_path();

    if !auth_path.exists() {
        return Ok(None);
    }

    let content = tokio::fs::read_to_string(&auth_path).await?;
    let auth: ClaudeAuthJson = serde_json::from_str(&content)?;
    Ok(Some(auth))
}

/// Read Claude auth credentials from file (sync)
///
/// Uses std::fs (not tokio::fs) for use in sync contexts like
/// credentials.rs detection and manager.rs get_claude().
/// Mirrors codex_auth::read_codex_auth() which is already sync.
pub fn read_claude_auth_sync() -> Result<Option<ClaudeAuthJson>> {
    let auth_path = get_claude_auth_path();

    if !auth_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&auth_path)?;
    let auth: ClaudeAuthJson = serde_json::from_str(&content)?;
    Ok(Some(auth))
}

/// Write auth data to claude_auth.json
pub async fn write_claude_auth(auth: &ClaudeAuthJson) -> Result<()> {
    let auth_path = get_claude_auth_path();

    // Create parent directory if it doesn't exist
    if let Some(parent) = auth_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let content = serde_json::to_string_pretty(auth)?;
    tokio::fs::write(&auth_path, content).await?;
    Ok(())
}
