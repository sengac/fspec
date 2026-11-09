//! TUI-123: connecting to a daemon within a timeout, with the failure
//! named by kind.
//!
//! Feature: spec/features/daemon-management-on-par-with-workspaces.feature
//!
//! The switcher shows why a daemon could not be reached: nothing listening
//! (connection refused), no answer within [`DAEMON_CONNECT_TIMEOUT`], a TLS
//! handshake failure, or the daemon rejecting the bearer token (HTTP 401).
//! Messages name the URL (which never holds credentials) and never the
//! token.

use std::time::Duration;

use tokio_tungstenite::tungstenite;

use crate::transport::WebSocketFspecBackend;

/// How long the switcher waits for a daemon to accept the connection.
pub const DAEMON_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Why a daemon could not be reached.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DaemonConnectError {
    #[error("connection refused: nothing is listening at {url} (is the daemon running?)")]
    Refused { url: String },
    #[error("timed out after {}s: the daemon at {url} did not answer", after.as_secs_f32())]
    TimedOut { url: String, after: Duration },
    #[error("TLS handshake failed with {url}: {detail}")]
    Tls { url: String, detail: String },
    #[error("the daemon at {url} rejected the token (401 Unauthorized); edit the daemon to save the right token")]
    TokenRejected { url: String },
    #[error("could not connect to {url}: {detail}")]
    Other { url: String, detail: String },
}

/// Connect a backend to the daemon at `url`, presenting `bearer`, giving
/// up after `timeout`.
pub async fn connect_daemon(
    url: &url::Url,
    bearer: Option<String>,
    timeout: Duration,
) -> Result<WebSocketFspecBackend, DaemonConnectError> {
    connect_daemon_trusting(url, bearer, None, timeout).await
}

/// RPC-458: [`connect_daemon`] that also trusts a saved daemon's own CA.
pub async fn connect_daemon_trusting(
    url: &url::Url,
    bearer: Option<String>,
    ca: Option<codelet_rpc_server::DaemonCa>,
    timeout: Duration,
) -> Result<WebSocketFspecBackend, DaemonConnectError> {
    let connecting = WebSocketFspecBackend::connect_trusting(url.clone(), bearer, ca);
    match tokio::time::timeout(timeout, connecting).await {
        Ok(Ok(backend)) => Ok(backend),
        Ok(Err(error)) => Err(classify(url, &error)),
        Err(_) => Err(DaemonConnectError::TimedOut {
            url: url.to_string(),
            after: timeout,
        }),
    }
}

/// Name the failure in `error`'s chain.
fn classify(url: &url::Url, error: &anyhow::Error) -> DaemonConnectError {
    let text = url.to_string();
    let tls = url.scheme() == "wss";
    for cause in error.chain() {
        if let Some(ws) = cause.downcast_ref::<tungstenite::Error>() {
            match ws {
                tungstenite::Error::Http(response) if response.status().as_u16() == 401 => {
                    return DaemonConnectError::TokenRejected { url: text };
                }
                tungstenite::Error::Tls(detail) => {
                    return DaemonConnectError::Tls {
                        url: text,
                        detail: detail.to_string(),
                    };
                }
                tungstenite::Error::Io(io) => {
                    if let Some(kind) = io_kind(&text, tls, io) {
                        return kind;
                    }
                }
                _ => {}
            }
        }
        if let Some(io) = cause.downcast_ref::<std::io::Error>() {
            if let Some(kind) = io_kind(&text, tls, io) {
                return kind;
            }
        }
    }
    DaemonConnectError::Other {
        url: text,
        detail: format!("{error:#}"),
    }
}

/// Refused connections, and (for `wss://`) the TLS layer's errors, which
/// tokio-rustls reports as `InvalidData` / unexpected-EOF I/O errors.
fn io_kind(url: &str, tls: bool, io: &std::io::Error) -> Option<DaemonConnectError> {
    use std::io::ErrorKind;
    match io.kind() {
        ErrorKind::ConnectionRefused => Some(DaemonConnectError::Refused {
            url: url.to_owned(),
        }),
        ErrorKind::InvalidData | ErrorKind::UnexpectedEof if tls => Some(DaemonConnectError::Tls {
            url: url.to_owned(),
            detail: io.to_string(),
        }),
        _ => None,
    }
}
