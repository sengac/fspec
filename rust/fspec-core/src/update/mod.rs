//! UPD-002 — shared in-place self-update engine.
//!
//! Feature: spec/features/in-place-self-update-engine.feature
//!
//! The single source of truth for both the `fspec update` CLI subcommand and
//! the TUI `/update` slash command (rule [0]: one engine, no duplication).
//!
//! Manual reqwest+sha2 download-verify-replace path (NOT the `self_update`
//! crate) so the engine can be pointed at a local mock GitHub API via a
//! `base_url` override. `self-replace` is used only for the Windows
//! locked-.exe rename.
//!
//! Pipeline: GET latest release → pick the current-platform asset →
//! download to a temp file → verify SHA-256 against the published
//! `SHA256SUMS` → extract the binary → atomic replace. A failed download or
//! checksum mismatch leaves the installed binary untouched (rule [4]).

mod github;
mod replace;
mod verify;

use std::path::PathBuf;

use thiserror::Error;
use tracing::debug;

/// Error type for the update engine (workspace standard: thiserror derive,
/// no `unwrap()`/`panic!()` in production code).
#[derive(Error, Debug)]
pub enum UpdateError {
    #[error("no network / GitHub API unreachable: {0}")]
    Network(String),
    #[error("no release asset found for target {0}")]
    NoAssetForTarget(String),
    #[error("checksum mismatch for asset {0}")]
    ChecksumMismatch(String),
    #[error("failed to replace binary: {0}")]
    ReplaceFailed(String),
}

/// Information about the latest release, as reported by
/// [`UpdateConfig::check_latest`].
#[derive(Debug, Clone)]
pub struct ReleaseInfo {
    /// Release tag, e.g. `v0.10.0`.
    pub tag: String,
    /// Tag with the leading `v` stripped, e.g. `0.10.0`.
    pub version: String,
    /// True when `version` is strictly newer than the configured
    /// `current_version`.
    pub is_newer: bool,
    /// Name of the release asset for the current platform, if present.
    pub asset_name: Option<String>,
}

/// The result of [`UpdateConfig::perform_update`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateOutcome {
    /// The running version is already the latest release; nothing changed.
    UpToDate { version: String },
    /// A newer release was downloaded, verified, and installed.
    Updated {
        version: String,
        /// True when the running process keeps the old inode (always true
        /// for in-place replacement) and the user must restart to activate.
        restart_required: bool,
    },
    /// The update failed; the installed binary is untouched.
    Failed { message: String },
}

/// Configuration for the update engine. Construct via
/// [`UpdateConfig::for_production`] (CLI/TUI default) or directly in tests
/// (pointing `base_url` at a mock GitHub API).
#[derive(Debug, Clone)]
pub struct UpdateConfig {
    /// Base URL of the GitHub API (no trailing slash). Production:
    /// `https://api.github.com`, overridable via `FSPEC_UPDATE_BASE_URL`.
    pub base_url: String,
    /// GitHub repository owner.
    pub repo_owner: String,
    /// GitHub repository name.
    pub repo_name: String,
    /// Binary name (release assets are named `<bin_name>-<target>.<ext>`).
    pub bin_name: String,
    /// The running version, e.g. `0.10.0` (no leading `v`).
    pub current_version: String,
    /// Override the detected target triple (tests). `None` → auto-detect.
    pub target: Option<String>,
    /// The installed binary path to replace. Production:
    /// `std::env::current_exe()`, overridable via `FSPEC_UPDATE_INSTALL_PATH`.
    pub install_path: PathBuf,
}

impl UpdateConfig {
    /// Production configuration: GitHub API base URL (with the
    /// `FSPEC_UPDATE_BASE_URL` test override), the `sengac/fspec` repo, the
    /// running binary as the install target.
    pub fn for_production(current_version: impl Into<String>) -> Self {
        let base_url = std::env::var("FSPEC_UPDATE_BASE_URL")
            .unwrap_or_else(|_| "https://api.github.com".into());
        let install_path = std::env::var("FSPEC_UPDATE_INSTALL_PATH")
            .ok()
            .map(PathBuf::from)
            .or_else(|| std::env::current_exe().ok())
            .unwrap_or_else(|| PathBuf::from("fspec"));
        Self {
            base_url,
            repo_owner: "sengac".into(),
            repo_name: "fspec".into(),
            bin_name: "fspec".into(),
            current_version: current_version.into(),
            target: None,
            install_path,
        }
    }

    /// The effective target triple (override or auto-detect).
    pub fn effective_target(&self) -> String {
        self.target
            .clone()
            .unwrap_or_else(|| current_target().to_string())
    }

    /// The expected release-asset name for the effective target.
    pub fn expected_asset_name(&self) -> String {
        format!("{}-{}.tar.gz", self.bin_name, self.effective_target())
    }

    /// Check the latest release. Returns [`ReleaseInfo`]; a network failure
    /// is an [`UpdateError::Network`].
    pub async fn check_latest(&self) -> Result<ReleaseInfo, UpdateError> {
        let release = github::fetch_latest_release(self).await?;
        let version = release.tag.trim_start_matches('v').to_string();
        let is_newer = github::is_newer_version(&version, &self.current_version);
        let asset_name = release
            .assets
            .iter()
            .find(|a| a.name == self.expected_asset_name())
            .map(|a| a.name.clone());
        debug!(
            tag = %release.tag,
            current = %self.current_version,
            is_newer,
            "update engine: latest release checked"
        );
        Ok(ReleaseInfo {
            tag: release.tag,
            version,
            is_newer,
            asset_name,
        })
    }

    /// Perform the update: check → (if newer) download → verify → replace.
    ///
    /// Returns [`UpdateOutcome::UpToDate`] when nothing is newer,
    /// [`UpdateOutcome::Updated`] on success, and — on any failure — an
    /// [`UpdateError`] with the installed binary left untouched (rule [4]).
    pub async fn perform_update(&self) -> Result<UpdateOutcome, UpdateError> {
        let info = self.check_latest().await?;
        if !info.is_newer {
            return Ok(UpdateOutcome::UpToDate {
                version: info.version,
            });
        }
        let asset_name = info
            .asset_name
            .clone()
            .ok_or_else(|| UpdateError::NoAssetForTarget(self.effective_target()))?;

        // 1. Download the asset to a temp file in the install directory
        //    (rename across filesystems fails).
        let tmp = github::download_asset_to_temp(self, &asset_name).await?;

        // 2. Verify the published SHA-256 digest BEFORE touching the binary.
        verify::verify_asset(self, &asset_name, &tmp).await?;

        // 3. Extract the binary from the archive into a second temp file.
        let extracted = replace::extract_binary(&tmp, &self.install_path).await?;

        // 4. Atomic replace (unix rename / Windows self-replace).
        replace::replace_binary(&extracted, &self.install_path)?;

        // 5. Clean up temp files.
        let _ = std::fs::remove_file(&tmp);
        let _ = std::fs::remove_file(&extracted);

        Ok(UpdateOutcome::Updated {
            version: info.version,
            restart_required: true,
        })
    }
}

/// Detect the current target triple at runtime (cfg!-based).
///
/// Matches the UPD-001 release asset naming contract exactly:
/// `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`,
/// `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`,
/// `aarch64-apple-darwin` (and `x86_64-apple-darwin`).
pub fn current_target() -> &'static str {
    if cfg!(target_os = "windows") {
        if cfg!(target_arch = "aarch64") {
            "aarch64-pc-windows-msvc"
        } else {
            "x86_64-pc-windows-msvc"
        }
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "aarch64-apple-darwin"
        } else {
            "x86_64-apple-darwin"
        }
    } else {
        if cfg!(target_arch = "aarch64") {
            "aarch64-unknown-linux-gnu"
        } else {
            "x86_64-unknown-linux-gnu"
        }
    }
}
