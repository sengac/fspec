//! RPC-447: daemon bearer-token authentication.
//!
//! Feature: spec/features/daemon-bearer-auth.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::{bind_and_serve, ws_client_connect, ws_connect};
use std::sync::Arc;
use tarpc::context;
use tokio_tungstenite::tungstenite::Error as WsError;

type TestResult = Result<(), Box<dyn std::error::Error>>;

async fn serve(
    token: Option<&str>,
) -> Result<(tempfile::TempDir, String), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    std::fs::create_dir_all(dir.path().join("spec"))?;
    std::fs::write(
        dir.path().join("spec/work-units.json"),
        r#"{"workUnits":{"AUTH-001":{"id":"AUTH-001","title":"Login","type":"story","status":"backlog"}}}"#,
    )?;
    let service = SharedFspecService::new(Arc::new(WorkUnitsWatcher::new(dir.path())?));
    if let Some(token) = token {
        service.set_auth_token(token.to_string());
    }
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", Arc::new(service)).await?;
    Ok((dir, format!("ws://{addr}/")))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_token_protected_daemon_rejects_a_handshake_without_the_bearer() -> TestResult {
    // @step Given a daemon service armed with token "s3cret"
    let (_dir, url) = serve(Some("s3cret")).await?;

    // @step When a client connects without an Authorization header
    let result = tokio_tungstenite::connect_async(url.as_str()).await;

    // @step Then the handshake is rejected with HTTP 401
    match result {
        Err(WsError::Http(response)) => assert_eq!(response.status().as_u16(), 401),
        other => panic!(
            "expected an HTTP 401 rejection, got {:?}",
            other.map(|_| ())
        ),
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_client_with_the_matching_bearer_is_served() -> TestResult {
    // @step Given a daemon service armed with token "s3cret"
    let (_dir, url) = serve(Some("s3cret")).await?;

    // @step When a client connects with bearer "s3cret" and lists work units
    let (ws, _response) = ws_connect(&url, Some("s3cret")).await?;
    let client = ws_client_connect(ws).await?;
    let units = client.client().list_work_units(context::current()).await?;

    // @step Then the RPC succeeds
    assert_eq!(units.len(), 1);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_daemon_without_a_token_accepts_plain_clients() -> TestResult {
    // @step Given a daemon service without a token
    let (_dir, url) = serve(None).await?;

    // @step When a client connects without an Authorization header and lists work units
    let (ws, _response) = ws_connect(&url, None).await?;
    let client = ws_client_connect(ws).await?;
    let units = client.client().list_work_units(context::current()).await?;

    // @step Then the RPC succeeds
    assert_eq!(units.len(), 1);
    Ok(())
}
