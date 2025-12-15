//! Codex OAuth Authentication Module
//!
//! Handles reading credentials from ~/.codex/auth.json or macOS keychain,
//! OAuth token refresh flow, and token exchange for OpenAI API keys.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// OAuth constants from Codex CLI
const CODEX_ISSUER: &str = "https://auth.openai.com";
const CODEX_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";

/// Keyring service name for macOS keychain
#[cfg(target_os = "macos")]
const KEYRING_SERVICE: &str = "Codex Auth";

/// Structure of the codex auth.json file
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

/// Response from token refresh endpoint
#[derive(Debug, Deserialize)]
struct RefreshResponse {
    id_token: String,
    access_token: String,
    refresh_token: String,
}

/// Response from token exchange endpoint
#[derive(Debug, Deserialize)]
struct ExchangeResponse {
    access_token: String,
}

/// Get the codex home directory
/// Uses CODEX_HOME env var if set, otherwise defaults to ~/.codex
fn get_codex_home() -> PathBuf {
    if let Ok(codex_home) = std::env::var("CODEX_HOME") {
        PathBuf::from(codex_home)
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/tmp"));
        PathBuf::from(home).join(".codex")
    }
}

/// Get the path to auth.json
pub fn get_auth_path() -> PathBuf {
    get_codex_home().join("auth.json")
}

/// Compute the keyring store key from codex home path
/// Format: cli|{first 16 chars of sha256 hash}
#[cfg(target_os = "macos")]
fn compute_store_key(codex_home: &PathBuf) -> String {
    use sha2::{Digest, Sha256};

    let canonical_path = codex_home
        .canonicalize()
        .unwrap_or_else(|_| codex_home.clone());
    let mut hasher = Sha256::new();
    hasher.update(canonical_path.to_string_lossy().as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    format!("cli|{}", &hash[..16])
}

/// Read credentials from macOS keychain
#[cfg(target_os = "macos")]
fn read_keychain_credentials() -> Result<Option<CodexAuthJson>> {
    use keyring::Entry;

    let codex_home = get_codex_home();
    let account = compute_store_key(&codex_home);

    match Entry::new(KEYRING_SERVICE, &account) {
        Ok(entry) => match entry.get_password() {
            Ok(password) => {
                let auth: CodexAuthJson = serde_json::from_str(&password)?;
                Ok(Some(auth))
            }
            Err(_) => Ok(None), // Not found in keychain
        },
        Err(_) => Ok(None),
    }
}

/// Read credentials from file
fn read_file_credentials() -> Result<Option<CodexAuthJson>> {
    let auth_path = get_auth_path();

    if !auth_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&auth_path)?;
    let auth: CodexAuthJson = serde_json::from_str(&content)?;
    Ok(Some(auth))
}

/// Read codex auth credentials
/// On macOS: checks keychain first, then falls back to file
/// On other platforms: reads from file only
pub fn read_codex_auth() -> Result<Option<CodexAuthJson>> {
    #[cfg(target_os = "macos")]
    {
        if let Some(auth) = read_keychain_credentials()? {
            return Ok(Some(auth));
        }
    }

    read_file_credentials()
}

/// Write auth data to auth.json
pub fn write_codex_auth(auth: &CodexAuthJson) -> Result<()> {
    let auth_path = get_auth_path();

    // Create parent directory if it doesn't exist
    if let Some(parent) = auth_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(auth)?;
    fs::write(&auth_path, content)?;
    Ok(())
}

/// Refresh OAuth tokens using refresh_token (async version for future use)
#[allow(dead_code)]
async fn refresh_tokens(refresh_token: &str) -> Result<RefreshResponse> {
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "client_id": CODEX_CLIENT_ID,
        "grant_type": "refresh_token",
        "refresh_token": refresh_token,
        "scope": "openid profile email offline_access",
    });

    let response = client
        .post(format!("{CODEX_ISSUER}/oauth/token"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Failed to refresh Codex tokens. Token may be expired. Run codex auth login to re-authenticate. Status: {}",
            response.status()
        ));
    }

    let refresh_response: RefreshResponse = response.json().await?;
    Ok(refresh_response)
}

/// Exchange id_token for OpenAI API key via token exchange grant (async version for future use)
#[allow(dead_code)]
async fn exchange_token_for_api_key(id_token: &str) -> Result<String> {
    let client = reqwest::Client::new();

    let params = [
        (
            "grant_type",
            "urn:ietf:params:oauth:grant-type:token-exchange",
        ),
        ("client_id", CODEX_CLIENT_ID),
        ("requested_token", "openai-api-key"),
        ("subject_token", id_token),
        (
            "subject_token_type",
            "urn:ietf:params:oauth:token-type:id_token",
        ),
    ];

    let response = client
        .post(format!("{CODEX_ISSUER}/oauth/token"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&params)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Failed to exchange token for API key. Status: {}",
            response.status()
        ));
    }

    let exchange_response: ExchangeResponse = response.json().await?;
    Ok(exchange_response.access_token)
}

/// Get OpenAI API key from Codex credentials (synchronous version)
/// Performs full flow: read → refresh → exchange → cache
pub fn get_codex_api_key_sync() -> Result<String> {
    let mut auth = read_codex_auth()?.ok_or_else(|| {
        anyhow!(
            "CODEX auth.json not found at {}. Run codex auth login to authenticate.",
            get_auth_path().display()
        )
    })?;

    // If cached API key exists, use it directly
    if let Some(api_key) = &auth.openai_api_key {
        return Ok(api_key.clone());
    }

    // No cached API key - need to refresh and exchange
    let tokens = auth.tokens.as_ref().ok_or_else(|| {
        anyhow!("No tokens found in auth.json. Run codex auth login to authenticate.")
    })?;

    // Use blocking reqwest client for synchronous operation
    let client = reqwest::blocking::Client::new();

    // Refresh tokens to get fresh id_token
    let body = serde_json::json!({
        "client_id": CODEX_CLIENT_ID,
        "grant_type": "refresh_token",
        "refresh_token": &tokens.refresh_token,
        "scope": "openid profile email offline_access",
    });

    let refresh_response = client
        .post(format!("{CODEX_ISSUER}/oauth/token"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()?;

    if !refresh_response.status().is_success() {
        return Err(anyhow!(
            "Failed to refresh Codex tokens. Token may be expired. Status: {}. Run codex auth login to re-authenticate.",
            refresh_response.status()
        ));
    }

    let refreshed: RefreshResponse = refresh_response.json()?;

    // Exchange id_token for API key
    let exchange_params = [
        (
            "grant_type",
            "urn:ietf:params:oauth:grant-type:token-exchange",
        ),
        ("subject_token", &refreshed.id_token),
        (
            "subject_token_type",
            "urn:ietf:params:oauth:token-type:id_token",
        ),
    ];

    let exchange_response = client
        .post(format!("{CODEX_ISSUER}/oauth/token"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&exchange_params)
        .send()?;

    if !exchange_response.status().is_success() {
        return Err(anyhow!(
            "Failed to exchange token for API key. Status: {}",
            exchange_response.status()
        ));
    }

    let exchange: ExchangeResponse = exchange_response.json()?;
    let api_key = exchange.access_token;

    // Update auth.json with new tokens and cached API key
    auth.tokens = Some(CodexTokens {
        id_token: refreshed.id_token,
        access_token: refreshed.access_token,
        refresh_token: refreshed.refresh_token,
        account_id: tokens.account_id.clone(),
    });
    auth.openai_api_key = Some(api_key.clone());
    write_codex_auth(&auth)?;

    Ok(api_key)
}

/// Get OpenAI API key from Codex credentials (async version for future use)
/// Performs full flow: read → refresh → exchange → cache
#[allow(dead_code)]
async fn get_codex_api_key() -> Result<String> {
    let mut auth = read_codex_auth()?.ok_or_else(|| {
        anyhow!(
            "CODEX auth.json not found at {}. Run codex auth login to authenticate.",
            get_auth_path().display()
        )
    })?;

    // If cached API key exists, use it directly
    if let Some(api_key) = &auth.openai_api_key {
        return Ok(api_key.clone());
    }

    // No cached API key - need to refresh and exchange
    let tokens = auth.tokens.as_ref().ok_or_else(|| {
        anyhow!("No tokens found in auth.json. Run codex auth login to authenticate.")
    })?;

    // Refresh tokens to get fresh id_token
    let refreshed = refresh_tokens(&tokens.refresh_token).await?;

    // Exchange id_token for API key
    let api_key = exchange_token_for_api_key(&refreshed.id_token).await?;

    // Update auth.json with new tokens and cached API key
    auth.tokens = Some(CodexTokens {
        id_token: refreshed.id_token,
        access_token: refreshed.access_token,
        refresh_token: refreshed.refresh_token,
        account_id: tokens.account_id.clone(),
    });
    auth.openai_api_key = Some(api_key.clone());
    auth.last_refresh = Some(chrono::Utc::now().to_rfc3339());

    // Write back to auth.json
    write_codex_auth(&auth)?;

    Ok(api_key)
}
