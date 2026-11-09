//! BUG-667 — `WebSocketFspecBackend::connect_with_supervisor` makes its
//! first connect with the same trust as every other client connect, so a
//! private-CA daemon is reachable with FSPEC_DAEMON_CA and refused without.
//!
//! Feature: spec/features/connect-with-supervisor-trusts-the-daemon-ca.feature
//!
//! Both tests set or remove FSPEC_DAEMON_CA for the whole process, so they
//! run one at a time.

use std::path::PathBuf;
use std::time::Duration;

use codelet_fspec_tui::transport::websocket::WebSocketFspecBackend;
use codelet_fspec_tui::{Action, FspecBackend};
use codelet_rpc_server::bind_and_serve_tls;
use codelet_rpc_server::tls::load_tls_acceptor;
use codelet_test_helpers::{OrFail, TestResult};
use tokio::sync::mpsc::unbounded_channel;

mod common;

/// The RPC-456 test TLS material (a localhost certificate and its CA).
fn tls_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../fspec/tests/fixtures/tls")
        .join(name)
}

/// A TLS daemon on 127.0.0.1 serving the localhost certificate, and its
/// `wss://localhost:<port>` URL. Keep the returned dir alive while used.
async fn tls_daemon() -> Result<(tempfile::TempDir, url::Url), codelet_test_helpers::TestError> {
    let (dir, service) = common::temp_service()?;
    let acceptor = load_tls_acceptor(&tls_fixture("server.pem"), &tls_fixture("server.key"))
        .or_fail("load the test certificate")?;
    let (addr, _stats, _join) = bind_and_serve_tls("127.0.0.1:0", service, acceptor)
        .await
        .or_fail("serve TLS on 127.0.0.1:0")?;
    let url = url::Url::parse(&format!("wss://localhost:{}", addr.port())).or_fail("wss url")?;
    Ok((dir, url))
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_private_ca_daemon_is_reachable_through_connect_with_supervisor_with_fspec_daemon_ca(
) -> TestResult {
    // @step Given a TLS daemon serving a certificate for localhost signed by the test CA
    let (_dir, url) = tls_daemon().await?;
    // @step And FSPEC_DAEMON_CA names the test CA
    std::env::set_var("FSPEC_DAEMON_CA", tls_fixture("ca.pem"));

    // @step When WebSocketFspecBackend::connect_with_supervisor connects to wss://localhost:<port>
    let (action_tx, mut action_rx) = unbounded_channel::<Action>();
    let connected = WebSocketFspecBackend::connect_with_supervisor(url, action_tx).await;
    std::env::remove_var("FSPEC_DAEMON_CA");
    let backend = connected.or_fail("connect_with_supervisor")?;

    // @step Then list_work_units answers at once and no Action::Disconnected was sent
    let units = backend
        .list_work_units()
        .await
        .or_fail("list_work_units over the first connect")?;
    assert!(!units.is_empty(), "the seeded work units come back");
    while let Ok(action) = action_rx.try_recv() {
        assert!(
            !matches!(action, Action::Disconnected),
            "the first connect was refused"
        );
    }
    Ok(())
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn without_fspec_daemon_ca_the_first_connect_is_refused() -> TestResult {
    // @step Given the same TLS daemon and no FSPEC_DAEMON_CA
    let (_dir, url) = tls_daemon().await?;
    std::env::remove_var("FSPEC_DAEMON_CA");

    // @step When WebSocketFspecBackend::connect_with_supervisor connects to wss://localhost:<port>
    let (action_tx, mut action_rx) = unbounded_channel::<Action>();
    let backend = WebSocketFspecBackend::connect_with_supervisor(url, action_tx)
        .await
        .or_fail("connect_with_supervisor")?;

    // @step Then Action::Disconnected arrives and list_work_units reports the backend disconnected
    let first = tokio::time::timeout(Duration::from_secs(5), action_rx.recv())
        .await
        .or_fail("an action within 5 s")?
        .or_fail("the action bus is open")?;
    assert!(matches!(first, Action::Disconnected), "{first:?}");
    let refused = backend.list_work_units().await;
    assert!(refused.is_err(), "the refused backend has no client");
    Ok(())
}
