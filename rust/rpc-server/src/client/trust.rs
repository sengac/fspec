//! RPC-458: the certificates a client trusts on a `wss://` connect.
//!
//! Feature: spec/features/clients-trust-a-daemon-s-private-ca.feature
//!
//! A client trusts the public web PKI roots plus the PEM certificates of
//! the file named by `FSPEC_DAEMON_CA` (every connect) and of a saved
//! daemon's own CA file (when connecting to that saved daemon). The
//! certificate is still verified against the dialed host: there is no
//! skip-verification mode. A CA file that is missing, unreadable or holds
//! no PEM certificate is an error naming the file and where it came from,
//! before any connection is tried; nothing falls back to the public roots
//! alone.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::CertificateDer;
use thiserror::Error;

/// The variable naming a PEM CA file every client connect trusts.
pub const DAEMON_CA_ENV: &str = "FSPEC_DAEMON_CA";

/// Where a CA file came from; errors name it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaOrigin {
    /// The `FSPEC_DAEMON_CA` variable.
    Env,
    /// The `ca` of the saved daemon with this name (`fspec daemons add --ca`).
    SavedDaemon(String),
    /// A `--ca` command-line option (checked before it is saved).
    Option,
}

impl fmt::Display for CaOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Env => f.write_str(DAEMON_CA_ENV),
            Self::SavedDaemon(name) => write!(f, "saved daemon {name}'s CA"),
            Self::Option => f.write_str("--ca"),
        }
    }
}

/// A saved daemon's own CA file, trusted when connecting to that daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonCa {
    /// The saved daemon's name.
    pub daemon: String,
    /// The PEM file.
    pub path: PathBuf,
}

/// Why a CA file cannot be trusted.
#[derive(Debug, Error)]
pub enum DaemonCaError {
    #[error("{origin} {} cannot be read: {reason}", path.display())]
    Unreadable {
        origin: CaOrigin,
        path: PathBuf,
        reason: String,
    },
    #[error("{origin} {} holds no PEM certificate", path.display())]
    NoCertificate { origin: CaOrigin, path: PathBuf },
    #[error("{origin} {} holds an unusable certificate: {reason}", path.display())]
    Invalid {
        origin: CaOrigin,
        path: PathBuf,
        reason: String,
    },
    #[error("{DAEMON_CA_ENV} is set but empty; name a PEM CA file or unset it")]
    EmptyEnv,
}

/// The PEM certificates in `path`, which `origin` named. Missing,
/// unreadable, malformed and certificate-less files are errors.
pub fn load_ca_file(
    path: &Path,
    origin: &CaOrigin,
) -> Result<Vec<CertificateDer<'static>>, DaemonCaError> {
    let bytes = std::fs::read(path).map_err(|error| DaemonCaError::Unreadable {
        origin: origin.clone(),
        path: path.to_owned(),
        reason: error.to_string(),
    })?;
    let mut certificates = Vec::new();
    for certificate in CertificateDer::pem_slice_iter(&bytes) {
        certificates.push(certificate.map_err(|error| DaemonCaError::Invalid {
            origin: origin.clone(),
            path: path.to_owned(),
            reason: error.to_string(),
        })?);
    }
    if certificates.is_empty() {
        return Err(DaemonCaError::NoCertificate {
            origin: origin.clone(),
            path: path.to_owned(),
        });
    }
    let mut check = rustls::RootCertStore::empty();
    for certificate in &certificates {
        check
            .add(certificate.clone())
            .map_err(|error| DaemonCaError::Invalid {
                origin: origin.clone(),
                path: path.to_owned(),
                reason: error.to_string(),
            })?;
    }
    Ok(certificates)
}

/// The `FSPEC_DAEMON_CA` file, when the variable is set.
fn env_ca() -> Result<Option<PathBuf>, DaemonCaError> {
    match std::env::var_os(DAEMON_CA_ENV) {
        None => Ok(None),
        Some(value) if value.is_empty() => Err(DaemonCaError::EmptyEnv),
        Some(value) => Ok(Some(PathBuf::from(value))),
    }
}

/// The rustls client configuration for a `wss://` connect: the public
/// roots, the `FSPEC_DAEMON_CA` certificates and `saved`'s, over the ring
/// provider (BUG-619).
pub fn client_tls_config(saved: Option<&DaemonCa>) -> Result<rustls::ClientConfig, anyhow::Error> {
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let mut extra = Vec::new();
    if let Some(path) = env_ca()? {
        extra.extend(load_ca_file(&path, &CaOrigin::Env)?);
    }
    if let Some(saved) = saved {
        extra.extend(load_ca_file(
            &saved.path,
            &CaOrigin::SavedDaemon(saved.daemon.clone()),
        )?);
    }
    for certificate in extra {
        roots.add(certificate)?;
    }
    Ok(rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()?
    .with_root_certificates(roots)
    .with_no_client_auth())
}
