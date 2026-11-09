//! RPC-456 — the daemon accepts remote clients over TLS with a required
//! token.
//!
//! Feature: spec/features/remote-daemon-over-tls-with-required-token.feature

mod common;

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::tls::{check_bind_security, load_tls_acceptor};
use codelet_rpc_server::{bind_and_serve_tls, ws_client_connect};
use common::{fspec_bin, make_workspace, ChildGuard};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::CertificateDer;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::Connector;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const TOKEN: &str = "rpc456-test-token";

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/tls")
        .join(name)
}

/// A rustls client connector trusting only the test CA.
fn trusting_connector() -> Result<Connector, Box<dyn std::error::Error>> {
    let mut roots = rustls::RootCertStore::empty();
    for certificate in CertificateDer::pem_file_iter(fixture("ca.pem"))? {
        roots.add(certificate?)?;
    }
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()?
    .with_root_certificates(roots)
    .with_no_client_auth();
    Ok(Connector::Rustls(Arc::new(config)))
}

/// Connect to `wss://localhost:<port>` trusting the test CA, presenting
/// `bearer` when given.
async fn wss_connect(
    port: u16,
    bearer: Option<&str>,
) -> Result<codelet_rpc_server::FspecWsClient, Box<dyn std::error::Error>> {
    let mut request = format!("wss://localhost:{port}").into_client_request()?;
    if let Some(token) = bearer {
        request
            .headers_mut()
            .insert("authorization", format!("Bearer {token}").parse()?);
    }
    let (ws, _response) = tokio::time::timeout(
        Duration::from_secs(10),
        tokio_tungstenite::connect_async_tls_with_config(
            request,
            None,
            false,
            Some(trusting_connector()?),
        ),
    )
    .await??;
    Ok(ws_client_connect(ws).await?)
}

/// A TLS daemon with the RPC-447 token armed, on an ephemeral loopback port.
async fn tls_daemon() -> Result<(tempfile::TempDir, u16), Box<dyn std::error::Error>> {
    let (workspace, _path) = make_workspace(&[("TLS-1", "remote", "backlog")]);
    let service = Arc::new(
        SharedFspecService::new(Arc::new(WorkUnitsWatcher::new(workspace.path())?))
            .with_cwd(workspace.path().to_path_buf()),
    );
    service.set_auth_token(TOKEN.to_owned());
    let acceptor = load_tls_acceptor(&fixture("server.pem"), &fixture("server.key"))?;
    let (addr, _stats, _join) = bind_and_serve_tls("127.0.0.1:0", service, acceptor).await?;
    Ok((workspace, addr.port()))
}

#[test]
fn a_network_bind_without_tls_or_a_token_is_refused() {
    // @step Given a daemon asked to bind "0.0.0.0:0"
    let bind = "0.0.0.0:0";
    // @step When it has no TLS certificate, or no token
    let without_tls = check_bind_security(bind, false, true);
    let without_token = check_bind_security(bind, true, false);
    // @step Then it refuses to start and names what is missing
    let without_tls = without_tls
        .expect_err("a bind without TLS must be refused")
        .to_string();
    assert!(without_tls.contains("--tls-cert"), "{without_tls}");
    let without_token = without_token
        .expect_err("a bind without a token must be refused")
        .to_string();
    assert!(
        without_token.contains("FSPEC_DAEMON_TOKEN"),
        "{without_token}"
    );
    assert!(check_bind_security(bind, true, true).is_ok());
}

#[test]
fn a_loopback_bind_needs_neither_tls_nor_a_token() {
    // @step Given a daemon asked to bind "127.0.0.1:0" without TLS or a token
    // @step Then the bind is allowed
    for bind in ["127.0.0.1:0", "[::1]:3777", "localhost:3777"] {
        assert!(check_bind_security(bind, false, false).is_ok(), "{bind}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_tls_daemon_serves_a_wss_client_that_presents_the_token() -> TestResult {
    // @step Given a daemon serving TLS with a token
    let (_workspace, port) = tls_daemon().await?;
    // @step When a wss client that trusts the certificate connects with the token
    let client = wss_connect(port, Some(TOKEN)).await?;
    // @step Then the client's health request is answered
    let health = client.client().health(tarpc::context::current()).await?;
    assert!(health.uptime_secs >= 0);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_tls_daemon_rejects_a_client_without_the_token() -> TestResult {
    // @step Given a daemon serving TLS with a token
    let (_workspace, port) = tls_daemon().await?;
    // @step When a wss client connects without the token
    let refused = wss_connect(port, None).await;
    // @step Then the connection is rejected as unauthorized
    let error = match refused {
        Ok(_) => return Err("a client without the token was served".into()),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("401"), "{error}");
    Ok(())
}

/// `fspec daemon` with an isolated home and runtime directory.
fn daemon_command(home: &std::path::Path, workspace: &std::path::Path) -> common::IsolatedCommand {
    let mut command = common::IsolatedCommand::new(fspec_bin());
    command
        .arg("--workspace")
        .arg(workspace)
        .arg("daemon")
        .env("HOME", home)
        .env("FSPEC_USER_DIR", home.join(".fspec"))
        .env("XDG_RUNTIME_DIR", home)
        .env("FSPEC_DAEMON_TOKEN", TOKEN)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

#[test]
fn the_daemon_command_refuses_a_network_bind_without_tls() -> TestResult {
    // @step Given the token is set
    let home = tempfile::tempdir()?;
    let (workspace, _path) = make_workspace(&[("TLS-2", "refused", "backlog")]);
    // @step When "fspec daemon --bind 0.0.0.0:0" runs without --tls-cert and --tls-key
    let output = daemon_command(home.path(), workspace.path())
        .args(["--bind", "0.0.0.0:0"])
        .output()?;
    // @step Then it exits with an error naming the missing TLS certificate
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--tls-cert"), "{stderr}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_daemon_command_serves_a_network_bind_with_tls_and_a_token() -> TestResult {
    // @step Given a certificate, its key and a token
    let home = tempfile::tempdir()?;
    let (workspace, _path) = make_workspace(&[("TLS-3", "served", "backlog")]);
    // BUG-595: a TLS daemon on an unspecified bind names its certificate's host.
    // @step When "fspec daemon --bind 0.0.0.0:0 --tls-cert --tls-key --tls-name localhost" runs
    let mut child = daemon_command(home.path(), workspace.path())
        .args(["--bind", "0.0.0.0:0", "--tls-cert"])
        .arg(fixture("server.pem"))
        .arg("--tls-key")
        .arg(fixture("server.key"))
        .args(["--tls-name", "localhost"])
        .spawn()?;
    let stdout = child.stdout.take().ok_or("no daemon stdout")?;
    let _guard = ChildGuard(child);
    let port: u16 = tokio::task::spawn_blocking(move || {
        let mut line = String::new();
        BufReader::new(stdout).read_line(&mut line).map(|_| line)
    })
    .await??
    .trim()
    .parse()?;
    // @step Then a wss client presenting the token is answered
    let client = wss_connect(port, Some(TOKEN)).await?;
    client.client().health(tarpc::context::current()).await?;
    Ok(())
}
