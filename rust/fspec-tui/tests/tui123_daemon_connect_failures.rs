//! TUI-123 — connecting to a daemon names the failure: connection refused,
//! timed out, TLS handshake failed or token rejected (401).
//!
//! Feature: spec/features/daemon-management-on-par-with-workspaces.feature
//!
//! Each "daemon" is a real local TCP socket behaving badly in one way.

use std::time::Duration;

use codelet_fspec_tui::daemon_connect::connect_daemon;
use codelet_test_helpers::TestResult;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const TOKEN: &str = "secret-token";

/// A port nothing listens on.
async fn closed_port() -> std::io::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    Ok(listener.local_addr()?.port())
}

/// Accept connections, answer each with `reply` once its request has
/// arrived (`None`: accept and never answer).
async fn serve(reply: Option<&'static str>) -> std::io::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    tokio::spawn(async move {
        let mut held = Vec::new();
        while let Ok((mut socket, _)) = listener.accept().await {
            match reply {
                Some(reply) => {
                    let mut request = vec![0u8; 4096];
                    let _ = socket.read(&mut request).await;
                    let _ = socket.write_all(reply.as_bytes()).await;
                    let _ = socket.shutdown().await;
                }
                None => held.push(socket),
            }
        }
    });
    Ok(port)
}

async fn failure(url: &str, timeout: Duration) -> Result<String, Box<dyn std::error::Error>> {
    let url = url::Url::parse(url)?;
    match connect_daemon(&url, Some(TOKEN.to_string()), timeout).await {
        Ok(_) => Err(format!("connecting to {url} must fail").into()),
        Err(error) => Ok(error.to_string()),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn connect_failures_are_named_by_kind() -> TestResult {
    // @step Given daemons that refuse the connection, never answer, do not speak TLS and reject the token with 401
    let refused = closed_port().await?;
    let silent = serve(None).await?;
    let plain = serve(Some(
        "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n",
    ))
    .await?;
    let rejecting = serve(Some(
        "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n",
    ))
    .await?;

    // @step When the client connects to each of them
    let wait = Duration::from_secs(5);
    let refused = failure(&format!("ws://127.0.0.1:{refused}"), wait).await?;
    let silent = failure(
        &format!("ws://127.0.0.1:{silent}"),
        Duration::from_millis(300),
    )
    .await?;
    let plain = failure(&format!("wss://127.0.0.1:{plain}"), wait).await?;
    let rejecting = failure(&format!("ws://127.0.0.1:{rejecting}"), wait).await?;

    // @step Then the failures read "connection refused", "timed out", "TLS handshake failed" and "rejected the token (401 Unauthorized)"
    assert!(refused.contains("connection refused"), "{refused}");
    assert!(silent.contains("timed out"), "{silent}");
    assert!(plain.contains("TLS handshake failed"), "{plain}");
    assert!(
        rejecting.contains("rejected the token (401 Unauthorized)"),
        "{rejecting}"
    );
    for message in [&refused, &silent, &plain, &rejecting] {
        assert!(!message.contains(TOKEN), "{message}");
    }
    Ok(())
}
