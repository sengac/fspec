//! GitHub API access for the update engine (UPD-002).
//!
//! Pure reqwest helpers: fetch the latest release, compare versions, and
//! download a release asset to a temp file. All errors map to
//! [`super::UpdateError::Network`] so callers get a single "unreachable"
//! failure mode for transport problems.

use std::path::Path;

use semver::Version;
use tracing::debug;

use super::{UpdateConfig, UpdateError};

/// A single release asset as returned by the GitHub API.
#[derive(Debug, Clone)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
}

/// The latest release as returned by the GitHub API.
#[derive(Debug, Clone)]
pub struct Release {
    pub tag: String,
    pub assets: Vec<Asset>,
}

/// Build the `User-Agent` header value. GitHub rejects empty user agents.
fn user_agent(current_version: &str) -> String {
    format!("fspec/{current_version}")
}

/// Fetch the latest release from `{base_url}/repos/{owner}/{repo}/releases/latest`.
pub async fn fetch_latest_release(cfg: &UpdateConfig) -> Result<Release, UpdateError> {
    let url = format!(
        "{}/repos/{}/{}/releases/latest",
        cfg.base_url.trim_end_matches('/'),
        cfg.repo_owner,
        cfg.repo_name
    );
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header(reqwest::header::USER_AGENT, user_agent(&cfg.current_version))
        .send()
        .await
        .map_err(|e| UpdateError::Network(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(UpdateError::Network(format!(
            "GitHub API returned {} for {url}",
            resp.status()
        )));
    }

    let value: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| UpdateError::Network(e.to_string()))?;

    let tag = value
        .get("tag_name")
        .and_then(|t| t.as_str())
        .map(str::to_string)
        .ok_or_else(|| UpdateError::Network("missing tag_name in release".into()))?;

    let assets = value
        .get("assets")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| {
                    let name = a.get("name")?.as_str()?.to_string();
                    let url = a.get("browser_download_url")?.as_str()?.to_string();
                    Some(Asset {
                        name,
                        browser_download_url: url,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(Release { tag, assets })
}

/// True when `candidate` is strictly newer than `current` (semver compare).
///
/// Non-semver strings fall back to a string inequality (newer tag != current
/// tag). This keeps the engine robust against pre-release suffixes.
pub fn is_newer_version(candidate: &str, current: &str) -> bool {
    match (
        Version::parse(candidate),
        Version::parse(current),
    ) {
        (Ok(c), Ok(cur)) => c > cur,
        _ => candidate != current,
    }
}

/// Download the named asset to a temp file in the install directory.
///
/// The temp file lives in the same directory as the install path so the
/// subsequent rename is atomic on the same filesystem.
pub async fn download_asset_to_temp(
    cfg: &UpdateConfig,
    asset_name: &str,
) -> Result<std::path::PathBuf, UpdateError> {
    let release = fetch_latest_release(cfg).await?;
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| UpdateError::NoAssetForTarget(asset_name.to_string()))?;

    // Resolve the download URL relative to the base URL when it is a
    // relative path (the mock server serves /assets/<name>).
    let url = if asset.browser_download_url.starts_with('/') {
        format!(
            "{}{}",
            cfg.base_url.trim_end_matches('/'),
            asset.browser_download_url
        )
    } else {
        asset.browser_download_url.clone()
    };

    let install_dir = cfg
        .install_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let tmp = install_dir.join(format!(".fspec-update-{asset_name}.tmp"));

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header(reqwest::header::USER_AGENT, user_agent(&cfg.current_version))
        .send()
        .await
        .map_err(|e| UpdateError::Network(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(UpdateError::Network(format!(
            "asset download returned {} for {url}",
            resp.status()
        )));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| UpdateError::Network(e.to_string()))?;

    std::fs::write(&tmp, &bytes)
        .map_err(|e| UpdateError::ReplaceFailed(format!("write temp file: {e}")))?;
    debug!(?tmp, "update engine: asset downloaded to temp");
    Ok(tmp)
}
