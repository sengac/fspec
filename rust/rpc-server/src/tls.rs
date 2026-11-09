//! RPC-456: the daemon's remote listener — when a bind may leave the
//! machine, and the TLS acceptor that serves it.
//!
//! Feature: spec/features/remote-daemon-over-tls-with-required-token.feature

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use thiserror::Error;
use tokio_rustls::TlsAcceptor;

/// Why a bind address was refused.
#[derive(Debug, Error, PartialEq, Eq)]
#[error(
    "--bind {bind} is not a loopback address; a network bind needs {}",
    missing.join(" and ")
)]
pub struct BindRefused {
    /// The refused bind address.
    pub bind: String,
    /// What the network bind is missing.
    pub missing: Vec<&'static str>,
}

/// Why the TLS certificate or key could not be loaded.
#[derive(Debug, Error)]
pub enum TlsSetupError {
    #[error("TLS certificate {path}: {reason}")]
    Certificate { path: PathBuf, reason: String },
    #[error("TLS key {path}: {reason}")]
    Key { path: PathBuf, reason: String },
    #[error("TLS configuration rejected: {0}")]
    Config(#[from] rustls::Error),
}

/// Whether `bind` is a loopback host (`127.0.0.1`, `::1`, `localhost`).
pub fn is_loopback_bind(bind: &str) -> bool {
    let host = match bind.rsplit_once(':') {
        Some((host, _port)) => host,
        None => bind,
    };
    matches!(
        host.trim_matches(|c| c == '[' || c == ']'),
        "127.0.0.1" | "::1" | "localhost"
    )
}

/// A loopback bind is always allowed; any other bind only with TLS AND a
/// bearer token, so nobody off the machine reaches the daemon in clear
/// text or without the token.
pub fn check_bind_security(bind: &str, tls: bool, token: bool) -> Result<(), BindRefused> {
    if is_loopback_bind(bind) {
        return Ok(());
    }
    let mut missing = Vec::new();
    if !tls {
        missing.push("TLS (--tls-cert and --tls-key)");
    }
    if !token {
        missing.push("a bearer token (FSPEC_DAEMON_TOKEN)");
    }
    if missing.is_empty() {
        return Ok(());
    }
    Err(BindRefused {
        bind: bind.to_owned(),
        missing,
    })
}

/// Build a TLS acceptor from a PEM certificate chain and a PEM private key.
pub fn load_tls_acceptor(cert: &Path, key: &Path) -> Result<TlsAcceptor, TlsSetupError> {
    let certificate_error = |reason: String| TlsSetupError::Certificate {
        path: cert.to_path_buf(),
        reason,
    };
    let chain = CertificateDer::pem_file_iter(cert)
        .map_err(|error| certificate_error(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| certificate_error(error.to_string()))?;
    if chain.is_empty() {
        return Err(certificate_error(
            "no certificate found in the file".to_owned(),
        ));
    }
    let private_key = PrivateKeyDer::from_pem_file(key).map_err(|error| TlsSetupError::Key {
        path: key.to_path_buf(),
        reason: error.to_string(),
    })?;
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()?
    .with_no_client_auth()
    .with_single_cert(chain, private_key)?;
    Ok(TlsAcceptor::from(Arc::new(config)))
}
