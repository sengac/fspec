use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub use super::auth_storage::{
    clear_codex_oauth, delete_codex_auth, read_codex_auth, write_codex_auth, AuthError,
};
pub use super::credential_refresh::refresh_codex_tokens;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexAuthJson {
    #[serde(rename = "OPENAI_API_KEY", skip_serializing_if = "Option::is_none")]
    pub openai_api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<CodexTokens>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_refresh: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexTokens {
    pub id_token: String,
    pub access_token: String,
    pub refresh_token: String,
    pub account_id: String,
}

pub fn get_auth_path() -> PathBuf {
    if let Some(home) = std::env::var_os("CODEX_HOME") {
        PathBuf::from(home).join("auth.json")
    } else {
        let home = std::env::var_os("HOME").unwrap_or_else(|| "/tmp".into());
        PathBuf::from(home).join(".codex/auth.json")
    }
}

pub fn get_codex_api_key_sync() -> Result<String, AuthError> {
    super::legacy_auth::api_key()
}
