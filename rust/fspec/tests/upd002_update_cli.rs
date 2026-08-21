//! UPD-002 — `fspec update` CLI subcommand.
//!
//! Feature: spec/features/in-place-self-update-cli-command.feature
//!
//! The `fspec update` subcommand calls the SAME shared
//! `codelet_fspec_core::update` engine as the TUI `/update` command
//! (rule [0]: one engine, no duplication). `--check` is scriptable: exit 0
//! when current, exit 1 when a newer release is available. Tests point the
//! engine at a local mock GitHub API via the `FSPEC_UPDATE_BASE_URL` env
//! override.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use flate2::write::GzEncoder;
use flate2::Compression;
use sha2::{Digest, Sha256};
use tar::Builder;
use tempfile::tempdir;

use codelet_fspec_core::update::current_target;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Mock GitHub API (axum on 127.0.0.1:0) — same shape as the engine test.
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

async fn start_mock(tag: &str, asset_bytes: &[u8]) -> String {
    let asset_name = format!("fspec-{}.tar.gz", current_target());
    let mut hasher = Sha256::new();
    hasher.update(asset_bytes);
    let digest = hex::encode(hasher.finalize());
    let state: MockStateArc = Arc::new(std::sync::Mutex::new(MockState {
        tag: tag.to_string(),
        asset_name: asset_name.clone(),
        asset_bytes: asset_bytes.to_vec(),
        digest,
    }));
    let app = axum::Router::new()
        .route("/repos/sengac/fspec/releases/latest", axum::routing::get(releases_latest))
        .route("/assets/{name}", axum::routing::get(asset))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock listener");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve mock");
    });
    format!("http://{addr}")
}

fn run_update(cwd: &Path, base_url: &str, check: bool) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("update");
    if check {
        cmd.arg("--check");
    }
    cmd.current_dir(cwd);
    cmd.env("FSPEC_UPDATE_BASE_URL", base_url);
    let output = cmd.output().expect("spawn fspec update");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

// =============================================================================
// Scenario: fspec update --check reports availability via exit code
// =============================================================================

// Multi-thread runtime: the mock server is a tokio task and `cmd.output()`
// blocks the test thread — a current-thread runtime would deadlock (the
// server task can't service the child's HTTP request while the test thread
// is blocked in cmd.output()).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_fspec_update_check_reports_availability_via_exit_code() {
    // @step Given fspec is installed at an older version
    // The test binary's `fspec --version` reports the workspace version
    // (0.10.0). We make the mock serve a NEWER tag so `--check` sees an
    // update available.
    let asset = make_targz(b"new-binary");
    let base = start_mock("v0.11.0", &asset).await;
    let cwd = tempdir().expect("tempdir");

    // @step When the user runs `fspec update --check`
    let (code, stdout, _stderr) = run_update(cwd.path(), &base, true);

    // @step Then it prints the latest available version
    assert!(
        stdout.contains("0.11.0"),
        "--check must print the latest available version (0.11.0), got: {stdout}"
    );

    // @step And it exits with code 1
    assert_eq!(code, 1, "--check must exit 1 when a newer release is available");

    // @step And when fspec is installed at the latest version, `fspec update --check` exits with code 0
    // Serve a tag at or below the running version → up to date → exit 0.
    let base_current = start_mock("v0.10.0", &asset).await;
    let (code_current, _out, _err) = run_update(cwd.path(), &base_current, true);
    assert_eq!(
        code_current, 0,
        "--check must exit 0 when the running version is the latest"
    );
}

// =============================================================================
// Scenario: fspec update installs the latest release in place
// =============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_fspec_update_installs_the_latest_release_in_place() {
    // @step Given fspec is installed at an older version
    // (the running binary reports 0.10.0; the mock serves v0.11.0)

    // @step And a newer release exists with an asset for the current platform
    let new_binary = b"new-binary-0.11.0";
    let asset = make_targz(new_binary);
    let base = start_mock("v0.11.0", &asset).await;
    let dir = tempdir().expect("tempdir");
    let install = dir.path().join("fspec");
    std::fs::write(&install, b"old-binary").expect("write installed binary");

    // @step When the user runs `fspec update`
    // Point the engine at the temp install path via the env override so the
    // test never touches the real binary.
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("update");
    cmd.current_dir(dir.path());
    cmd.env("FSPEC_UPDATE_BASE_URL", &base);
    cmd.env("FSPEC_UPDATE_INSTALL_PATH", install.to_str().expect("install path"));
    let output = cmd.output().expect("spawn fspec update");

    // @step Then it installs the new binary
    assert!(
        output.status.success(),
        "fspec update must exit 0 on success; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read(&install).expect("read installed"),
        new_binary,
        "installed binary must be the downloaded one"
    );

    // @step And it prints a success line naming the new version
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("0.11.0"),
        "success line must name the new version (0.11.0), got: {stdout}"
    );
}

// =============================================================================
// Scenario: /update and fspec update share one update engine
// =============================================================================

#[test]
fn scenario_update_and_fspec_update_share_one_update_engine() {
    // @step Given the /update TUI command and the `fspec update` CLI subcommand are both implemented
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let update_cmd = manifest.join("src/update_cmd.rs");
    let src = std::fs::read_to_string(&update_cmd).expect("read update_cmd.rs");

    // @step When both are exercised against the same release
    // (structural: the CLI module must call the shared engine)

    // @step Then both use the same shared download-verify-replace engine
    assert!(
        src.contains("codelet_fspec_core::update") || src.contains("update::"),
        "update_cmd.rs must call the shared codelet_fspec_core::update engine"
    );

    // @step And no download or replacement logic is duplicated between them
    assert!(
        !src.contains("reqwest::") && !src.contains("Sha256") && !src.contains("self_replace"),
        "update_cmd.rs must NOT contain its own download/verify/replace logic"
    );
}
