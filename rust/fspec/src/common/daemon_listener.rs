//! BUG-595: what a local daemon's discovery entry advertises: the scheme
//! it serves and the host a local client dials (product owner, 2026-11-07).
//! Re-exported from `common`; split out of `daemon_json.rs` to keep it
//! under its size cap (REFAC-024).
//!
//! Feature: spec/features/local-daemon-discovery-names-the-scheme-and-host.feature

use anyhow::{anyhow, Result};
use serde::Deserialize;

/// BUG-595: the WebSocket scheme a daemon serves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum DaemonScheme {
    /// Plain WebSocket.
    #[serde(rename = "ws")]
    Ws,
    /// WebSocket over TLS (--tls-cert/--tls-key).
    #[serde(rename = "wss")]
    Wss,
}

impl DaemonScheme {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ws => "ws",
            Self::Wss => "wss",
        }
    }

    /// The scheme a discovery file records. An entry without the field was
    /// written before it existed, when every daemon served plain ws;
    /// anything else is an error.
    pub fn from_entry(value: Option<&serde_json::Value>) -> Result<Self> {
        match value {
            None => Ok(Self::Ws),
            Some(serde_json::Value::String(text)) if text == "ws" => Ok(Self::Ws),
            Some(serde_json::Value::String(text)) if text == "wss" => Ok(Self::Wss),
            Some(other) => Err(anyhow!(
                "invalid `scheme` {other}: expected \"ws\" or \"wss\""
            )),
        }
    }
}

/// BUG-595: the host a discovery file records. An entry without the field
/// was written before it existed, when local clients dialed 127.0.0.1; an
/// empty or non-string host is an error.
pub fn host_from_entry(value: Option<&serde_json::Value>) -> Result<String> {
    match value {
        None => Ok(LOOPBACK_HOST.to_string()),
        Some(serde_json::Value::String(host)) if !host.is_empty() => Ok(host.clone()),
        Some(other) => Err(anyhow!(
            "invalid `host` {other}: expected a host name or address"
        )),
    }
}

/// The host a plain daemon on an unspecified bind is dialed at.
const LOOPBACK_HOST: &str = "127.0.0.1";

/// BUG-595: what a daemon's discovery entry advertises: the scheme it serves
/// and the host a local client dials.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonListener {
    pub scheme: DaemonScheme,
    pub host: String,
}

impl DaemonListener {
    /// A plain listener on 127.0.0.1 (combined mode).
    pub fn loopback() -> Self {
        Self {
            scheme: DaemonScheme::Ws,
            host: LOOPBACK_HOST.to_string(),
        }
    }

    /// The URL a client dials for this listener on `port`.
    pub fn url(&self, port: u16) -> String {
        daemon_url(self.scheme, &self.host, port)
    }
}

/// `<scheme>://<host>:<port>`, an IPv6 address in brackets.
pub(super) fn daemon_url(scheme: DaemonScheme, host: &str, port: u16) -> String {
    if host.contains(':') {
        format!("{}://[{host}]:{port}", scheme.as_str())
    } else {
        format!("{}://{host}:{port}", scheme.as_str())
    }
}

/// BUG-595 (product owner, 2026-11-07): the scheme and host a daemon bound
/// to `bind` advertises. The host is the --bind host as given; an
/// unspecified bind (0.0.0.0, ::) is 127.0.0.1 for a plain daemon, and a TLS
/// daemon advertises `tls_name` (the host its certificate is for) when given
/// and refuses an unspecified bind without it.
pub fn discovery_listener(bind: &str, tls: bool, tls_name: Option<&str>) -> Result<DaemonListener> {
    let host = match bind.rsplit_once(':') {
        Some((host, _port)) => host,
        None => bind,
    };
    let host = host.trim_start_matches('[').trim_end_matches(']');
    let unspecified = host
        .parse::<std::net::IpAddr>()
        .is_ok_and(|address| address.is_unspecified());
    if !tls {
        let host = if unspecified { LOOPBACK_HOST } else { host };
        return Ok(DaemonListener {
            scheme: DaemonScheme::Ws,
            host: host.to_string(),
        });
    }
    let host = match tls_name {
        Some(name) if !name.is_empty() => name,
        Some(_) => return Err(anyhow!("--tls-name must not be empty")),
        None if unspecified => {
            return Err(anyhow!(
                "--bind {bind} listens on every address; a TLS daemon needs --tls-name <the host its certificate is for> so local clients can dial it"
            ))
        }
        None => host,
    };
    Ok(DaemonListener {
        scheme: DaemonScheme::Wss,
        host: host.to_string(),
    })
}
