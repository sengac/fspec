//! RPC-447: client WebSocket connect with an optional bearer token.
//!
//! Feature: spec/features/daemon-bearer-auth.feature
//! Feature: spec/features/clients-trust-a-daemon-s-private-ca.feature

use std::sync::Arc;

use anyhow::Context;
use tokio_tungstenite::tungstenite;
use tokio_tungstenite::WebSocketStream;

use super::trust::{client_tls_config, DaemonCa};

/// The stream and handshake response of a client connect.
pub type WsConnection = (
    WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    tungstenite::handshake::client::Response,
);

/// RPC-447: open a client WebSocket to `url`, sending
/// `Authorization: Bearer <bearer>` when a token is given (wss:// via
/// rustls, trusting the public roots and `FSPEC_DAEMON_CA`, RPC-458).
pub async fn ws_connect(url: &str, bearer: Option<&str>) -> anyhow::Result<WsConnection> {
    ws_connect_trusting(url, bearer, None).await
}

/// RPC-458: [`ws_connect`] that also trusts a saved daemon's own CA. The
/// trust is built (and a bad CA file refused) before any connection is
/// tried; a certificate that still fails verification is an error naming
/// the dialed host.
pub async fn ws_connect_trusting(
    url: &str,
    bearer: Option<&str>,
    saved_ca: Option<&DaemonCa>,
) -> anyhow::Result<WsConnection> {
    use tungstenite::client::IntoClientRequest;
    install_crypto_provider();
    let mut request = url.into_client_request()?;
    if let Some(token) = bearer {
        request
            .headers_mut()
            .insert("authorization", format!("Bearer {token}").parse()?);
    }
    let host = request.uri().host().unwrap_or_default().to_owned();
    let connector = match request.uri().scheme_str() {
        Some("wss") => Some(tokio_tungstenite::Connector::Rustls(Arc::new(
            client_tls_config(saved_ca)?,
        ))),
        _ => None,
    };
    match tokio_tungstenite::connect_async_tls_with_config(request, None, false, connector).await {
        Ok(connection) => Ok(connection),
        Err(error) if is_certificate_error(&error) => Err(anyhow::Error::new(error))
            .with_context(|| format!("TLS certificate verification failed for {host}")),
        Err(error) => Err(error.into()),
    }
}

/// Whether the TLS handshake refused the daemon's certificate.
fn is_certificate_error(error: &tungstenite::Error) -> bool {
    let tungstenite::Error::Io(io) = error else {
        return false;
    };
    io.get_ref()
        .and_then(|inner| inner.downcast_ref::<rustls::Error>())
        .is_some_and(|tls| {
            matches!(
                tls,
                rustls::Error::InvalidCertificate(_) | rustls::Error::InvalidCertRevocationList(_)
            )
        })
}

/// BUG-619: rustls cannot pick a process crypto provider when more than one
/// is compiled in (nanograph's AWS SDK brings aws-lc-rs beside the ring
/// provider RPC-456 uses), and tokio-tungstenite's wss:// connector then
/// panics. Installs ring, the provider the daemon's TLS listener uses, as
/// the process default unless the process already chose one. fspec's main
/// calls it first; [`ws_connect`] calls it for library callers.
pub fn install_crypto_provider() {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        // Another thread installing a provider first is just as good.
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
}
