use super::reconnect::NetworkFailure;
use rig::http_client::Error;
use std::error::Error as StdError;

pub(super) struct NetworkResolver;

pub(super) async fn resolve(host: &str, port: u16) -> std::io::Result<Vec<std::net::SocketAddr>> {
    tokio::net::lookup_host((host, port))
        .await
        .map(Iterator::collect)
        .map_err(|_| std::io::Error::other(NetworkFailure::Connection))
}

impl reqwest::dns::Resolve for NetworkResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        Box::pin(async move {
            let addresses = resolve(name.as_str(), 0).await?;
            Ok(Box::new(addresses.into_iter()) as reqwest::dns::Addrs)
        })
    }
}

fn connection(error: &std::io::Error) -> bool {
    error
        .get_ref()
        .is_some_and(|source| source.downcast_ref::<NetworkFailure>().is_some())
        || matches!(
            error.kind(),
            std::io::ErrorKind::ConnectionRefused
                | std::io::ErrorKind::ConnectionReset
                | std::io::ErrorKind::ConnectionAborted
                | std::io::ErrorKind::NotConnected
                | std::io::ErrorKind::AddrNotAvailable
                | std::io::ErrorKind::NetworkDown
                | std::io::ErrorKind::NetworkUnreachable
                | std::io::ErrorKind::HostUnreachable
                | std::io::ErrorKind::TimedOut
        )
}

pub(super) fn http_setup(error: reqwest::Error) -> Error {
    let mut source = error.source();
    while let Some(error) = source {
        if error.downcast_ref::<NetworkFailure>().is_some() {
            return NetworkFailure::Connection.into_http_error();
        }
        if let Some(error) = error.downcast_ref::<std::io::Error>() {
            if connection(error) {
                return NetworkFailure::Connection.into_http_error();
            }
        }
        source = error.source();
    }
    Error::Instance("Codex network setup failed".into())
}

pub(super) fn socket_setup(error: tokio_tungstenite::tungstenite::Error) -> Error {
    match error {
        tokio_tungstenite::tungstenite::Error::Io(error) if connection(&error) => {
            NetworkFailure::Connection.into_http_error()
        }
        _ => Error::Instance("Codex socket setup failed".into()),
    }
}

pub(super) fn socket_stream(error: tokio_tungstenite::tungstenite::Error) -> Error {
    use tokio_tungstenite::tungstenite::{error::ProtocolError, Error as SocketError};
    match error {
        SocketError::ConnectionClosed
        | SocketError::Protocol(ProtocolError::ResetWithoutClosingHandshake) => {
            NetworkFailure::Interrupted.into_http_error()
        }
        SocketError::Io(error)
            if connection(&error)
                || matches!(
                    error.kind(),
                    std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::UnexpectedEof
                ) =>
        {
            NetworkFailure::Interrupted.into_http_error()
        }
        error => Error::Instance(Box::new(error)),
    }
}
