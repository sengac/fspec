//! GitHub Copilot Authentication Persistence Module (PROV-053 / PROV-054 /
//! PROV-057).
//!
//! Handles reading/writing GitHub Copilot OAuth credentials to
//! `~/.fspec/credentials/copilot_auth.json` (mode 0600 on Unix).
//!
//! ## PROV-057 schema change — two-token model
//!
//! GitHub Copilot uses a **two-token** system. The long-lived `gho_*` token
//! must be exchanged at `GET /copilot_internal/v2/token` for a short-lived
//! (~25 min) Copilot token, and only that short-lived token is accepted by
//! `api.githubcopilot.com`. The schema therefore tracks both tokens
//! separately. Legacy `copilot_auth.json` files written by the pre-PROV-057
//! flow are still readable — deserialization accepts the old `access_token`
//! / `refresh_token` fields as aliases for `github_oauth_token` so users do
//! not need to re-authenticate after upgrading.
//!
//! Provides both async and sync readers:
//! - [`read_copilot_auth`] — async, for NAPI bindings
//! - [`read_copilot_auth_sync`] — sync, for `credentials.rs` detection and
//!   `manager.rs::get_github_copilot`

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Filename inside the fspec credentials directory for the Copilot
/// credential file.
pub const COPILOT_AUTH_FILENAME: &str = "copilot_auth.json";

/// Sentinel `expires` value used to encode "never expires" for the
/// long-lived GitHub OAuth token slot. Exposed as a constant so call sites
/// do not depend on the magic number `0`.
pub const COPILOT_TOKEN_NEVER_EXPIRES: u64 = 0;

/// GitHub Copilot OAuth credentials persisted to disk.
///
/// PROV-057: this struct tracks the **two** tokens in the Copilot model
/// separately — the long-lived `gho_*` / `ghu_*` GitHub OAuth token and the
/// short-lived Copilot API token minted from
/// `GET /copilot_internal/v2/token`. The legacy fields `access_token`,
/// `refresh_token`, `expires` are accepted for backward compatibility on
/// read (they all alias `github_oauth_token`) so pre-PROV-057 credential
/// files remain loadable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(from = "CopilotAuthJsonWire", into = "CopilotAuthJsonWire")]
pub struct CopilotAuthJson {
    /// Long-lived GitHub OAuth token returned by the device-flow
    /// `/login/oauth/access_token` endpoint. Does not expire on its own.
    pub github_oauth_token: String,
    /// Short-lived Copilot API token minted from the
    /// `/copilot_internal/v2/token` exchange. `None` means the exchange
    /// has not yet been performed (e.g. fresh login).
    pub copilot_token: Option<String>,
    /// Unix seconds at which the cached `copilot_token` expires. `None`
    /// when `copilot_token` is `None`.
    pub copilot_token_expires_at: Option<u64>,
    /// The `endpoints.api` URL returned by the most recent token
    /// exchange. Trusted over any hard-coded base URL so GitHub Enterprise
    /// deployments route to `copilot-api.<host>` automatically.
    pub endpoints_api: Option<String>,
    /// Optional Enterprise host (normalized form, e.g. `"ghe.example.com"`).
    /// Absent or `None` for github.com deployments.
    pub enterprise_url: Option<String>,
}

impl CopilotAuthJson {
    /// Create a fresh post-login `CopilotAuthJson` from the long-lived
    /// GitHub OAuth token. The Copilot token fields are left `None` and
    /// filled in by the first token exchange.
    #[must_use]
    pub fn from_github_oauth_token(
        github_oauth_token: String,
        enterprise_url: Option<String>,
    ) -> Self {
        Self {
            github_oauth_token,
            copilot_token: None,
            copilot_token_expires_at: None,
            endpoints_api: None,
            enterprise_url,
        }
    }
}

/// Wire format for `copilot_auth.json`.
///
/// Accepts both the PROV-057 schema and the legacy PROV-054 schema
/// (`access_token`, `refresh_token`, `expires`). Whichever field is present
/// on read becomes the source for `github_oauth_token`. On write only the
/// PROV-057 fields are emitted — the legacy fields are never re-emitted.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
struct CopilotAuthJsonWire {
    #[serde(skip_serializing_if = "Option::is_none")]
    github_oauth_token: Option<String>,

    // Legacy fields (PROV-054). Skipped on serialize so writes use the new
    // schema only. Still accepted on deserialize so old files keep working.
    #[serde(skip_serializing)]
    access_token: Option<String>,
    #[serde(skip_serializing)]
    refresh_token: Option<String>,
    #[serde(skip_serializing)]
    expires: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    copilot_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    copilot_token_expires_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    endpoints_api: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enterprise_url: Option<String>,
}

impl From<CopilotAuthJsonWire> for CopilotAuthJson {
    fn from(wire: CopilotAuthJsonWire) -> Self {
        let github_oauth_token = wire
            .github_oauth_token
            .or(wire.access_token)
            .or(wire.refresh_token)
            .unwrap_or_default();
        Self {
            github_oauth_token,
            copilot_token: wire.copilot_token,
            copilot_token_expires_at: wire.copilot_token_expires_at,
            endpoints_api: wire.endpoints_api,
            enterprise_url: wire.enterprise_url,
        }
    }
}

impl From<CopilotAuthJson> for CopilotAuthJsonWire {
    fn from(auth: CopilotAuthJson) -> Self {
        Self {
            github_oauth_token: Some(auth.github_oauth_token),
            access_token: None,
            refresh_token: None,
            expires: None,
            copilot_token: auth.copilot_token,
            copilot_token_expires_at: auth.copilot_token_expires_at,
            endpoints_api: auth.endpoints_api,
            enterprise_url: auth.enterprise_url,
        }
    }
}

/// Resolve the fspec credentials directory.
///
/// Delegates to [`crate::oauth::fspec_home`] — the single source of
/// truth for `$FSPEC_HOME || $HOME/.fspec/credentials`. Shared with
/// `claude_auth`, `cred_module`, and scripted OAuth providers.
fn get_fspec_home() -> PathBuf {
    crate::oauth::fspec_home()
}

/// Resolve the absolute path to `copilot_auth.json` inside the fspec
/// credentials directory.
pub fn get_copilot_auth_path() -> PathBuf {
    get_fspec_home().join(COPILOT_AUTH_FILENAME)
}

/// Read Copilot auth credentials from file (async).
///
/// Returns `Ok(None)` if the file does not exist.
pub async fn read_copilot_auth() -> Result<Option<CopilotAuthJson>> {
    let path = get_copilot_auth_path();
    if !path.exists() {
        return Ok(None);
    }
    let content = tokio::fs::read_to_string(&path)
        .await
        .with_context(|| format!("failed to read {}", path.display()))?;
    let value: CopilotAuthJson = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(Some(value))
}

/// Read Copilot auth credentials from file (sync).
///
/// Used by `credentials.rs::detect` and `manager.rs::get_github_copilot`
/// from sync contexts that cannot await.
pub fn read_copilot_auth_sync() -> Result<Option<CopilotAuthJson>> {
    let path = get_copilot_auth_path();
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let value: CopilotAuthJson = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(Some(value))
}

/// Write Copilot auth credentials with file mode 0600 on Unix.
///
/// Per PROV-054 Rule 9 the credential file MUST be persisted with file
/// permissions `0o600` to protect the OAuth access token from other users
/// on shared systems. The parent fspec credentials directory is created if
/// it does not exist.
pub async fn write_copilot_auth(auth: &CopilotAuthJson) -> Result<()> {
    let path = get_copilot_auth_path();
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let content = serde_json::to_string_pretty(auth)?;
    tokio::fs::write(&path, content)
        .await
        .with_context(|| format!("failed to write {}", path.display()))?;
    enforce_mode_0600(&path).await?;
    Ok(())
}

/// Delete the Copilot credential file (idempotent logout).
///
/// Returns `Ok(())` even if the file does not exist so `logout` can be
/// invoked safely from the CLI without "file not found" noise.
pub async fn delete_copilot_auth() -> Result<()> {
    let path = get_copilot_auth_path();
    if !path.exists() {
        return Ok(());
    }
    tokio::fs::remove_file(&path)
        .await
        .with_context(|| format!("failed to delete {}", path.display()))?;
    Ok(())
}

/// Apply `0o600` permissions to a credential file on Unix hosts.
///
/// No-op on non-Unix platforms so Windows builds link without cfg plumbing.
#[cfg(unix)]
async fn enforce_mode_0600(path: &std::path::Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = tokio::fs::metadata(path).await?.permissions();
    perms.set_mode(0o600);
    tokio::fs::set_permissions(path, perms).await?;
    Ok(())
}

#[cfg(not(unix))]
async fn enforce_mode_0600(_path: &std::path::Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use serial_test::serial;

    /// Per-test guard that points `FSPEC_HOME` at a fresh tempdir so writes
    /// never escape into the user's real `~/.fspec`. Restores the previous
    /// value on drop.
    struct FspecHomeGuard {
        _tempdir: tempfile::TempDir,
        original: Option<String>,
    }

    impl FspecHomeGuard {
        fn new() -> Self {
            let tempdir = tempfile::tempdir().expect("tempdir");
            let original = std::env::var("FSPEC_HOME").ok();
            std::env::set_var("FSPEC_HOME", tempdir.path());
            Self {
                _tempdir: tempdir,
                original,
            }
        }
    }

    impl Drop for FspecHomeGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(val) => std::env::set_var("FSPEC_HOME", val),
                None => std::env::remove_var("FSPEC_HOME"),
            }
        }
    }

    fn sample_auth() -> CopilotAuthJson {
        CopilotAuthJson::from_github_oauth_token("gho_round_trip".to_string(), None)
    }

    #[tokio::test]
    #[serial]
    async fn read_returns_none_when_file_missing() {
        let _guard = FspecHomeGuard::new();
        let result = read_copilot_auth().await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    #[serial]
    async fn write_then_read_round_trips_the_credential() {
        let _guard = FspecHomeGuard::new();
        let original = sample_auth();
        write_copilot_auth(&original).await.unwrap();
        let read_back = read_copilot_auth()
            .await
            .unwrap()
            .expect("credential should be present after write");
        assert_eq!(read_back, original);
    }

    #[tokio::test]
    #[serial]
    async fn sync_reader_returns_the_same_content_as_async() {
        let _guard = FspecHomeGuard::new();
        let original = sample_auth();
        write_copilot_auth(&original).await.unwrap();
        let read_back = read_copilot_auth_sync().unwrap().unwrap();
        assert_eq!(read_back, original);
    }

    #[tokio::test]
    #[serial]
    async fn delete_is_idempotent_on_missing_file() {
        let _guard = FspecHomeGuard::new();
        delete_copilot_auth().await.unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn delete_removes_existing_file() {
        let _guard = FspecHomeGuard::new();
        let original = sample_auth();
        write_copilot_auth(&original).await.unwrap();
        let path = get_copilot_auth_path();
        assert!(path.exists());
        delete_copilot_auth().await.unwrap();
        assert!(!path.exists());
    }

    #[cfg(unix)]
    #[tokio::test]
    #[serial]
    async fn write_enforces_mode_0600_on_unix() {
        use std::os::unix::fs::PermissionsExt;
        let _guard = FspecHomeGuard::new();
        let original = sample_auth();
        write_copilot_auth(&original).await.unwrap();
        let path = get_copilot_auth_path();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        // Strip file-type bits, compare only the permission bits.
        assert_eq!(mode & 0o777, 0o600);
    }

    #[tokio::test]
    #[serial]
    async fn path_filename_matches_constant() {
        let _guard = FspecHomeGuard::new();
        let path = get_copilot_auth_path();
        assert_eq!(path.file_name().unwrap(), COPILOT_AUTH_FILENAME);
    }

    // =====================================================================
    // PROV-057 schema tests
    // Feature: spec/features/github-copilot-end-to-end-integration.feature
    // =====================================================================

    #[tokio::test]
    #[serial]
    async fn schema_separates_github_oauth_token_from_copilot_token() {
        // Scenario: CopilotAuthJson schema separates GitHub OAuth and Copilot tokens
        let _guard = FspecHomeGuard::new();

        // @step Given a successful Copilot OAuth login completes
        let auth = CopilotAuthJson {
            github_oauth_token: "gho_schema_sep".to_string(),
            copilot_token: None,
            copilot_token_expires_at: None,
            endpoints_api: None,
            enterprise_url: None,
        };

        // @step When copilot_auth.json is written to ~/.fspec/credentials
        write_copilot_auth(&auth).await.unwrap();
        let path = get_copilot_auth_path();
        let raw = std::fs::read_to_string(&path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&raw).unwrap();

        // @step Then the file contains a non-empty "github_oauth_token" field starting with "gho_" or "ghu_"
        let gh = json
            .get("github_oauth_token")
            .and_then(|v| v.as_str())
            .expect("github_oauth_token must be present");
        assert!(!gh.is_empty());
        assert!(
            gh.starts_with("gho_") || gh.starts_with("ghu_"),
            "github_oauth_token must start with gho_ or ghu_, got {gh}"
        );

        // @step And the file contains a "copilot_token" field that may be initially absent
        // @step And the file contains a "copilot_token_expires_at" field that may be initially absent
        // @step And the file contains an "endpoints_api" field that may be initially absent
        let round_trip = read_copilot_auth().await.unwrap().unwrap();
        assert!(round_trip.copilot_token.is_none());
        assert!(round_trip.copilot_token_expires_at.is_none());
        assert!(round_trip.endpoints_api.is_none());

        // @step And the file mode is 0600 on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    #[tokio::test]
    #[serial]
    async fn schema_round_trips_all_prov_057_fields() {
        let _guard = FspecHomeGuard::new();
        let auth = CopilotAuthJson {
            github_oauth_token: "gho_full".to_string(),
            copilot_token: Some("tid=abc;exp=1234;:sig".to_string()),
            copilot_token_expires_at: Some(1_700_000_000),
            endpoints_api: Some("https://api.githubcopilot.com".to_string()),
            enterprise_url: None,
        };
        write_copilot_auth(&auth).await.unwrap();
        let read_back = read_copilot_auth().await.unwrap().unwrap();
        assert_eq!(read_back, auth);
    }

    #[tokio::test]
    #[serial]
    async fn legacy_pre_prov_057_credential_file_is_readable() {
        // Backward-compat: a copilot_auth.json written before PROV-057
        // used access_token / refresh_token / expires. Those files must
        // still load as a PROV-057 CopilotAuthJson with the token moved
        // into github_oauth_token.
        let _guard = FspecHomeGuard::new();
        let path = get_copilot_auth_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let legacy_json = r#"{
            "access_token": "ghu_legacy",
            "refresh_token": "ghu_legacy",
            "expires": 0,
            "enterprise_url": "ghe.example.com"
        }"#;
        std::fs::write(&path, legacy_json).unwrap();

        let loaded = read_copilot_auth().await.unwrap().unwrap();
        assert_eq!(loaded.github_oauth_token, "ghu_legacy");
        assert_eq!(loaded.enterprise_url.as_deref(), Some("ghe.example.com"));
        assert!(loaded.copilot_token.is_none());
        assert!(loaded.copilot_token_expires_at.is_none());
        assert!(loaded.endpoints_api.is_none());
    }
}
