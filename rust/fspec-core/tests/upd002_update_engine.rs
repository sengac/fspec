//! UPD-002 — shared in-place self-update engine.
//!
//! Feature: spec/features/in-place-self-update-engine.feature
//!
//! The engine lives in `codelet_fspec_core::update`. It is the single source
//! of truth for both the `fspec update` CLI subcommand and the TUI `/update`
//! command (rule [0]). Tests point the engine at a local mock GitHub API
//! (axum on 127.0.0.1:0) via a `base_url` override — "redirect, don't
//! intercept" (workspace test philosophy).
//!
//! Each scenario maps 1:1 to a Gherkin scenario; @step comments mirror the
//! step text verbatim.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::sync::Arc;

use flate2::write::GzEncoder;
use flate2::Compression;
use sha2::{Digest, Sha256};
use tar::Builder;
use tempfile::tempdir;

use codelet_fspec_core::update::{current_target, UpdateConfig, UpdateError, UpdateOutcome};

// ─────────────────────────────────────────────────────────────────────────
// Mock GitHub API (axum on 127.0.0.1:0)
// ─────────────────────────────────────────────────────────────────────────

struct MockState {
    tag: String,
    asset_name: String,
    asset_bytes: Vec<u8>,
    digest: String,
}

type MockStateArc = Arc<std::sync::Mutex<MockState>>;

async fn releases_latest(
    axum::extract::State(s): axum::extract::State<MockStateArc>,
) -> axum::Json<serde_json::Value> {
    let st = s.lock().expect("mock state lock");
    axum::Json(serde_json::json!({
        "tag_name": st.tag,
        "name": format!("fspec {}", st.tag),
        "assets": [
            { "name": st.asset_name,
              "digest": format!("sha256:{}", st.digest),
              "browser_download_url": format!("/assets/{}", st.asset_name) }
        ]
    }))
}

async fn asset(
    axum::extract::State(s): axum::extract::State<MockStateArc>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> (
    [(axum::http::header::HeaderName, axum::http::header::HeaderValue); 1],
    Vec<u8>,
) {
    let st = s.lock().expect("mock state lock");
    if name != st.asset_name {
        panic!("mock served unexpected asset {name}");
    }
    (
        [(
            axum::http::header::CONTENT_TYPE,
            axum::http::header::HeaderValue::from_static("application/octet-stream"),
        )],
        st.asset_bytes.clone(),
    )
}

/// Build a tar.gz (in memory) whose single `fspec` entry is `binary_bytes`.
fn make_targz(binary_bytes: &[u8]) -> Vec<u8> {
    let mut tar_bytes = Vec::new();
    {
        let enc = GzEncoder::new(&mut tar_bytes, Compression::fast());
        let mut b = Builder::new(enc);
        let mut hdr = tar::Header::new_gnu();
        hdr.set_size(binary_bytes.len() as u64);
        hdr.set_mode(0o755);
        hdr.set_cksum();
        b.append_data(&mut hdr, "fspec", binary_bytes)
            .expect("append fspec entry");
        b.into_inner().expect("finish tar builder");
    }
    tar_bytes
}

/// Start the mock server; returns (base_url, state handle).
async fn start_mock(tag: &str, asset_bytes: &[u8], correct_digest: bool) -> (String, MockStateArc) {
    let asset_name = format!("fspec-{}.tar.gz", current_target());
    let mut hasher = Sha256::new();
    hasher.update(asset_bytes);
    let real_hex = hex::encode(hasher.finalize());
    let fake_hex = "0".repeat(64);
    let digest = if correct_digest { real_hex } else { fake_hex };
    let state: MockStateArc = Arc::new(std::sync::Mutex::new(MockState {
        tag: tag.to_string(),
        asset_name: asset_name.clone(),
        asset_bytes: asset_bytes.to_vec(),
        digest,
    }));
    let app = axum::Router::new()
        .route("/repos/sengac/fspec/releases/latest", axum::routing::get(releases_latest))
        .route("/assets/{name}", axum::routing::get(asset))
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock listener");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve mock");
    });
    (format!("http://{addr}"), state)
}

fn config_for(base: &str, current: &str, install: &Path) -> UpdateConfig {
    UpdateConfig {
        base_url: base.to_string(),
        repo_owner: "sengac".to_string(),
        repo_name: "fspec".to_string(),
        bin_name: "fspec".to_string(),
        current_version: current.to_string(),
        target: None, // use the host target (matches the mock asset name)
        install_path: install.to_path_buf(),
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Engine reports up-to-date when already on the latest release
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_engine_reports_up_to_date_when_already_on_the_latest_release() {
    // @step Given the engine is configured at the latest released version
    let (base, _state) = start_mock("v0.10.0", b"new-binary", true).await;
    let dir = tempdir().expect("tempdir");
    let install = dir.path().join("fspec");
    fs::write(&install, b"old-binary").expect("write installed binary");
    let before = fs::read(&install).expect("read before");
    let cfg = config_for(&base, "0.10.0", &install);

    // @step When the engine checks for the latest release
    let info = cfg.check_latest().await.expect("check_latest");

    // @step Then it reports up-to-date
    assert_eq!(info.version, "0.10.0");
    assert!(!info.is_newer, "same version must not be newer");

    // @step And the installed binary is unchanged
    assert_eq!(fs::read(&install).expect("read after"), before);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Engine installs the latest release in place
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_engine_installs_the_latest_release_in_place() {
    // @step Given the engine is configured at an older version
    let new_binary = b"new-binary-0.10.0";
    let asset = make_targz(new_binary);
    let (base, _state) = start_mock("v0.10.0", &asset, true).await;
    let dir = tempdir().expect("tempdir");
    let install = dir.path().join("fspec");
    fs::write(&install, b"old-binary-0.9.3").expect("write installed binary");
    let cfg = config_for(&base, "0.9.3", &install);

    // @step And a newer release exists with an asset for the current platform
    let info = cfg.check_latest().await.expect("check_latest");
    assert!(info.is_newer, "0.10.0 is newer than 0.9.3");

    // @step When the engine performs an update
    let outcome = cfg.perform_update().await.expect("perform_update");

    // @step Then it reports the new version
    assert!(
        matches!(outcome, UpdateOutcome::Updated { ref version, .. } if version == "0.10.0"),
        "expected Updated(0.10.0), got {outcome:?}"
    );

    // @step And the installed binary is replaced with the downloaded binary
    assert_eq!(
        fs::read(&install).expect("read installed"),
        new_binary,
        "installed binary must be the downloaded one"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Engine fails safely with no network
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_engine_fails_safely_with_no_network() {
    // @step Given the engine is configured at an older version
    let dir = tempdir().expect("tempdir");
    let install = dir.path().join("fspec");
    fs::write(&install, b"old-binary").expect("write installed binary");
    let before = fs::read(&install).expect("read before");
    // A closed port: nothing is listening.
    let cfg = config_for("http://127.0.0.1:1", "0.9.3", &install);

    // @step And the network is unreachable
    // (the closed port above guarantees an unreachable endpoint)

    // @step When the engine performs an update
    let err = cfg.perform_update().await.expect_err("must fail");

    // @step Then it returns a network error
    assert!(
        matches!(err, UpdateError::Network(_)),
        "expected Network error, got {err:?}"
    );

    // @step And the installed binary is unchanged
    assert_eq!(fs::read(&install).expect("read after"), before);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Engine verifies the checksum before replacing the binary
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_engine_verifies_the_checksum_before_replacing_the_binary() {
    // @step Given a newer release exists with an asset for the current platform
    let new_binary = b"new-binary";
    let asset = make_targz(new_binary);
    // correct_digest = false → the published digest does not match.
    let (base, _state) = start_mock("v0.10.0", &asset, false).await;

    // @step And the published SHA-256 digest does not match the asset content
    // (the mock release carries an all-zero digest for the asset)
    let dir = tempdir().expect("tempdir");
    let install = dir.path().join("fspec");
    fs::write(&install, b"old-binary").expect("write installed binary");
    let before = fs::read(&install).expect("read before");
    let cfg = config_for(&base, "0.9.3", &install);

    // @step When the engine performs an update
    let err = cfg.perform_update().await.expect_err("must fail on bad checksum");

    // @step Then it returns a checksum mismatch error
    assert!(
        matches!(err, UpdateError::ChecksumMismatch(_)),
        "expected ChecksumMismatch, got {err:?}"
    );

    // @step And the installed binary is unchanged
    assert_eq!(
        fs::read(&install).expect("read after"),
        before,
        "a checksum failure must leave the installed binary untouched"
    );
}
