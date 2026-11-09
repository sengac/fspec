//! BUG-428 — the daemon switcher authenticates to token-protected local
//! daemons.
//!
//! Feature: spec/features/daemon-switcher-local-token.feature

use std::sync::Arc;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::daemons::{DaemonChoice, DaemonDirectory};
use codelet_rpc::SharedFspecService;

use super::ClientDaemonDirectory;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A local daemon that requires bearer "s3cret".
async fn protected_daemon() -> Result<(tempfile::TempDir, DaemonChoice), Box<dyn std::error::Error>>
{
    let dir = tempfile::tempdir()?;
    std::fs::create_dir_all(dir.path().join("spec"))?;
    std::fs::write(
        dir.path().join("spec/work-units.json"),
        r#"{"workUnits":{}}"#,
    )?;
    let service = SharedFspecService::new(Arc::new(WorkUnitsWatcher::new(dir.path())?));
    service.set_auth_token("s3cret".to_string());
    let (addr, _stats, _join) =
        codelet_rpc_server::bind_and_serve("127.0.0.1:0", Arc::new(service)).await?;
    let choice = DaemonChoice {
        name: format!("local:{}", addr.port()),
        url: format!("ws://127.0.0.1:{}", addr.port()),
        saved: false,
    };
    Ok((dir, choice))
}

fn switcher() -> ClientDaemonDirectory {
    ClientDaemonDirectory {
        action_tx: tokio::sync::mpsc::unbounded_channel().0,
    }
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_local_daemon_is_reached_with_the_fspec_daemon_token_bearer() -> TestResult {
    // @step Given a local daemon that requires bearer "s3cret"
    let (_dir, choice) = protected_daemon().await?;
    // @step And FSPEC_DAEMON_TOKEN is "s3cret" in the client
    std::env::set_var("FSPEC_DAEMON_TOKEN", "s3cret");
    // @step When the switcher connects to the local daemon entry
    let connected = switcher().connect(&choice).await;
    std::env::remove_var("FSPEC_DAEMON_TOKEN");
    // @step Then the connection succeeds
    connected?;
    Ok(())
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_protected_local_daemon_refuses_a_switcher_without_a_token() -> TestResult {
    // @step Given a local daemon that requires bearer "s3cret"
    let (_dir, choice) = protected_daemon().await?;
    // @step And FSPEC_DAEMON_TOKEN is not set in the client
    std::env::remove_var("FSPEC_DAEMON_TOKEN");
    // @step When the switcher connects to the local daemon entry
    let connected = switcher().connect(&choice).await;
    // @step Then the connection fails
    assert!(
        connected.is_err(),
        "a protected daemon must refuse a client without its bearer"
    );
    Ok(())
}
