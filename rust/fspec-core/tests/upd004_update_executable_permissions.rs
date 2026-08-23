//! UPD-004 — updater must set executable permissions on the replaced binary.
//!
//! Feature: spec/features/updater-does-not-set-executable-permissions-on-the-replaced-binary-unix.feature
//!
//! Regression: the engine extracted the binary to a temp file with default
//! umask permissions (0o644) and `std::fs::rename` preserved that mode, so
//! after a successful update the installed binary was NOT executable and the
//! next `fspec` invocation failed with `permission denied`. The test points
//! the engine at a local mock GitHub API (axum on 127.0.0.1:0) via the
//! `base_url` override — "redirect, don't intercept".
//!
//! Unix-only by nature (the kernel refuses to exec a non-+x file); on
//! Windows the scenario is a no-op assertion.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[cfg(unix)]
mod unix {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::Arc;

    use flate2::write::GzEncoder;
    use flate2::Compression;
    use sha2::{Digest, Sha256};
    use tar::Builder;
    use tempfile::tempdir;

    use codelet_fspec_core::update::{current_target, UpdateConfig, UpdateOutcome};

    // ─────────────────────────────────────────────────────────────────────
    // Mock GitHub API (axum on 127.0.0.1:0) — same shape as the UPD-002
    // engine test.
    // ─────────────────────────────────────────────────────────────────────

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

    /// Build a tar.gz (in memory) whose single `fspec` entry is
    /// `binary_bytes`.
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

    /// Start the mock server; returns the base URL.
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
            .route(
                "/repos/sengac/fspec/releases/latest",
                axum::routing::get(releases_latest),
            )
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

    // ─────────────────────────────────────────────────────────────────────
    // Scenario: Engine installs an executable binary on Unix
    // ─────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn scenario_engine_installs_an_executable_binary_on_unix() {
        // @step Given the engine is configured at an older version
        let new_binary = b"new-binary-0.10.0";
        let asset = make_targz(new_binary);
        let base = start_mock("v0.10.0", &asset).await;
        let dir = tempdir().expect("tempdir");
        let install = dir.path().join("fspec");
        fs::write(&install, b"old-binary-0.9.3").expect("write installed binary");
        let cfg = UpdateConfig {
            base_url: base,
            repo_owner: "sengac".to_string(),
            repo_name: "fspec".to_string(),
            bin_name: "fspec".to_string(),
            current_version: "0.9.3".to_string(),
            target: None, // use the host target (matches the mock asset name)
            install_path: install.clone(),
        };

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

        // @step And the installed binary is executable
        let meta = fs::metadata(&install).expect("read installed metadata");
        let mode = meta.permissions().mode() & 0o777;
        assert_eq!(
            mode,
            0o755,
            "installed binary must be executable (0o755), got 0o{mode:o}"
        );
    }
}
