//! BUG-619: a wss:// connect from fspec's own client chooses the ring crypto
//! provider, so it never panics while aws-lc-rs is linked beside it.
//!
//! Feature: spec/features/wss-connects-choose-the-ring-crypto-provider.feature
//!
//! This test binary links the fspec crate's dependencies, nanograph's AWS
//! SDK (aws-lc-rs) among them, so both rustls providers are present, as in
//! the fspec binary.

mod common;

use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::bind_and_serve_tls;
use codelet_rpc_server::tls::load_tls_acceptor;
use common::{fspec_bin, make_workspace};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/tls")
        .join(name)
}

/// A TLS daemon listener on 127.0.0.1 serving the test certificate (for
/// localhost, signed by a CA no client trusts).
async fn tls_listener() -> Result<(tempfile::TempDir, u16), Box<dyn std::error::Error>> {
    let (workspace, _path) = make_workspace(&[("TLS-619", "listener", "backlog")]);
    let service = Arc::new(SharedFspecService::new(Arc::new(WorkUnitsWatcher::new(
        workspace.path(),
    )?)));
    let acceptor = load_tls_acceptor(&fixture("server.pem"), &fixture("server.key"))?;
    let (address, _stats, _join) = bind_and_serve_tls("127.0.0.1:0", service, acceptor).await?;
    Ok((workspace, address.port()))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fspec_status_reports_a_tls_failure_instead_of_panicking() -> TestResult {
    // @step Given a TLS daemon listener on localhost whose certificate the client does not trust
    let (_workspace, port) = tls_listener().await?;
    let home = tempfile::tempdir()?;

    // @step When "fspec status --connect wss://localhost:<port>" runs
    let output = tokio::task::spawn_blocking({
        let home = home.path().to_path_buf();
        move || {
            Command::new(fspec_bin())
                .args(["status", "--connect", &format!("wss://localhost:{port}")])
                .env("HOME", &home)
                .env("XDG_RUNTIME_DIR", &home)
                .env_remove("FSPEC_DAEMON_TOKEN")
                .output()
        }
    })
    .await??;

    // @step Then it exits with code 1 and says the connection failed
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("connect failed"), "{stderr}");
    // @step And its stderr names no CryptoProvider panic
    assert!(
        !stderr.contains("CryptoProvider") && !stderr.contains("panicked"),
        "{stderr}"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_connect_chooses_ring_when_both_crypto_providers_are_linked() -> TestResult {
    // @step Given a test binary that links both the ring and the aws-lc-rs rustls providers and installed no default
    assert!(rustls::crypto::CryptoProvider::get_default().is_none());
    let (_workspace, port) = tls_listener().await?;

    // @step When ws_connect dials wss://localhost:<port> of a TLS listener with an untrusted certificate
    let url = format!("wss://localhost:{port}");
    let connected = tokio::spawn(async move {
        codelet_rpc_server::ws_connect(&url, None)
            .await
            .map(|_| ())
            .map_err(|error| format!("{error:#}"))
    })
    .await
    .map_err(|join| format!("ws_connect panicked: {join}"))?;

    // @step Then it returns an error about the certificate
    let error = connected
        .err()
        .ok_or("the untrusted certificate was accepted")?;
    assert!(error.to_lowercase().contains("certificate"), "{error}");
    // @step And the process default crypto provider is ring
    let installed = rustls::crypto::CryptoProvider::get_default().ok_or("no default provider")?;
    let ring = rustls::crypto::ring::default_provider();
    assert_eq!(
        format!("{:?}", installed.cipher_suites),
        format!("{:?}", ring.cipher_suites)
    );
    Ok(())
}
