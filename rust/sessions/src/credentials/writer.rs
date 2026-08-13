//! Credential Writer (RPC-054)
//!
//! Persists provider API-key credentials to
//! `<data_dir>/credentials/credentials.json`, mirroring the TypeScript
//! `saveCredential` / `deleteCredential` write path
//! (`src/utils/credentials.ts`). The Rust-native TUI has no TypeScript
//! frontend to own the write path, so these functions ARE the write path.
//!
//! Read-modify-write over the existing `CredentialsFile` shape (already
//! `camelCase` via serde). After every mutation the in-memory
//! `CredentialStore` cache + provider env vars are refreshed via
//! `credentials_reload()` so live sessions observe the change without a
//! restart (mirrors the TS `credentialsReload()` NAPI notification).

use super::store::credentials_reload;
use super::types::{CredentialsFile, ProviderCredential};
use chrono::Utc;
use std::path::{Path, PathBuf};

/// Path to the credentials file under a given data directory.
fn credentials_path(data_dir: &Path) -> PathBuf {
    data_dir.join("credentials").join("credentials.json")
}

/// Load the existing credentials file, treating a missing or empty file as
/// `{ version: 1, providers: {} }` (matches TS `loadCredentials`).
fn load_or_default(path: &Path) -> Result<CredentialsFile, String> {
    if !path.exists() {
        return Ok(CredentialsFile {
            version: 1,
            providers: Default::default(),
        });
    }
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read credentials file: {e}"))?;
    if content.trim().is_empty() {
        return Ok(CredentialsFile {
            version: 1,
            providers: Default::default(),
        });
    }
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse credentials file: {e}"))
}

/// Write the credentials file with secure permissions (dir 0700, file 0600
/// on unix). chmod failures are swallowed to match the TS behaviour where
/// the directory/file may have been torn down during tests.
fn write_credentials_file(path: &Path, creds: &CredentialsFile) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create credentials directory: {e}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700));
        }
    }
    let json = serde_json::to_string_pretty(creds)
        .map_err(|e| format!("Failed to serialize credentials: {e}"))?;
    std::fs::write(path, json).map_err(|e| format!("Failed to write credentials file: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// Persist an api-key credential for `provider_id` under `data_dir`.
///
/// Creates the credentials directory/file on demand, updates the provider
/// entry in place (preserving `version` and other providers), stamps
/// `last_updated` with the current time, then refreshes the in-memory store.
/// An empty `api_key` is rejected without touching disk.
pub fn save_credential_with_dir(
    data_dir: &Path,
    provider_id: &str,
    api_key: &str,
) -> Result<(), String> {
    if api_key.is_empty() {
        return Err("api_key input requires a non-empty api_key".to_string());
    }
    let path = credentials_path(data_dir);
    let mut creds = load_or_default(&path)?;
    if creds.version == 0 {
        creds.version = 1;
    }
    creds.providers.insert(
        provider_id.to_string(),
        ProviderCredential {
            api_key: api_key.to_string(),
            last_updated: Utc::now(),
        },
    );
    write_credentials_file(&path, &creds)?;
    // Best-effort cache + env-var refresh (mirrors TS credentialsReload()).
    let _ = credentials_reload();
    Ok(())
}

/// Remove an api-key credential for `provider_id` under `data_dir`.
///
/// A missing file or absent provider is a successful no-op (matches TS:
/// `delete` on an absent key writes the file back unchanged). Deleting the
/// last provider leaves `{ version: 1, providers: {} }` on disk.
pub fn delete_credential_with_dir(data_dir: &Path, provider_id: &str) -> Result<(), String> {
    let path = credentials_path(data_dir);
    if !path.exists() {
        return Ok(());
    }
    let mut creds = load_or_default(&path)?;
    if creds.version == 0 {
        creds.version = 1;
    }
    creds.providers.remove(provider_id);
    write_credentials_file(&path, &creds)?;
    let _ = credentials_reload();
    Ok(())
}

/// Convenience: persist an api-key credential using the globally configured
/// data directory (`codelet_common::get_data_dir`). Used by the embedded
/// backend `set_provider_credentials` RPC handler.
pub fn save_credential(provider_id: &str, api_key: &str) -> Result<(), String> {
    let data_dir = codelet_common::get_data_dir()?;
    save_credential_with_dir(&data_dir, provider_id, api_key)
}

/// Convenience: remove an api-key credential using the globally configured
/// data directory. Used by the embedded backend `delete_provider_credentials`
/// RPC handler.
pub fn delete_credential(provider_id: &str) -> Result<(), String> {
    let data_dir = codelet_common::get_data_dir()?;
    delete_credential_with_dir(&data_dir, provider_id)
}
