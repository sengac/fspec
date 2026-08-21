//! SHA-256 verification for the update engine (UPD-002).
//!
//! The downloaded asset is verified against the SHA-256 digest GitHub
//! publishes for the release asset (the `digest` field on `assets[]` in the
//! release JSON, formatted `sha256:<hex>`) BEFORE it replaces the installed
//! binary (rule [1]). A mismatch is a [`super::UpdateError::ChecksumMismatch`]
//! and the installed binary is left untouched (rule [4]).

use sha2::{Digest, Sha256};
use tracing::debug;

use super::{UpdateConfig, UpdateError};

/// Verify the bytes in `downloaded` against the published digest for
/// `asset_name`.
pub async fn verify_asset(
    cfg: &UpdateConfig,
    asset_name: &str,
    downloaded: &std::path::Path,
) -> Result<(), UpdateError> {
    let bytes = std::fs::read(downloaded)
        .map_err(|e| UpdateError::ReplaceFailed(format!("read downloaded asset: {e}")))?;

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let actual = hex::encode(hasher.finalize());

    let expected = fetch_expected_digest(cfg, asset_name).await?;
    if !constant_time_eq(actual.as_bytes(), expected.as_bytes()) {
        return Err(UpdateError::ChecksumMismatch(asset_name.to_string()));
    }
    debug!(asset = asset_name, "update engine: checksum verified");
    Ok(())
}

/// Fetch the latest release and return the hex SHA-256 digest published for
/// `asset_name`.
///
/// GitHub exposes the per-asset digest as `assets[].digest` in the form
/// `sha256:<64-hex>`.
async fn fetch_expected_digest(
    cfg: &UpdateConfig,
    asset_name: &str,
) -> Result<String, UpdateError> {
    let url = format!(
        "{}/repos/{}/{}/releases/latest",
        cfg.base_url.trim_end_matches('/'),
        cfg.repo_owner,
        cfg.repo_name
    );
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header(reqwest::header::USER_AGENT, format!("fspec/{}", cfg.current_version))
        .send()
        .await
        .map_err(|e| UpdateError::Network(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(UpdateError::Network(format!(
            "release fetch returned {} for {url}",
            resp.status()
        )));
    }

    let value: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| UpdateError::Network(e.to_string()))?;

    let Some(assets) = value.get("assets").and_then(|a| a.as_array()) else {
        return Err(UpdateError::ChecksumMismatch(format!(
            "no assets[] in release for {asset_name}"
        )));
    };
    for asset in assets {
        let name = asset.get("name").and_then(|n| n.as_str()).unwrap_or("");
        if name != asset_name {
            continue;
        }
        let digest = asset
            .get("digest")
            .and_then(|d| d.as_str())
            .unwrap_or("");
        let hex = digest.strip_prefix("sha256:").unwrap_or(digest);
        if hex.len() == 64 {
            return Ok(hex.to_ascii_lowercase());
        }
        return Err(UpdateError::ChecksumMismatch(format!(
            "malformed digest for {asset_name}"
        )));
    }
    Err(UpdateError::ChecksumMismatch(format!(
        "no digest for {asset_name} in release"
    )))
}

/// Constant-time comparison to avoid a timing side-channel on the digest.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}
