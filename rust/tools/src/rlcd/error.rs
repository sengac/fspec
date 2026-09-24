//! RLCD-001 — error types for the RLCD service supervisor.
//!
//! Feature: spec/features/rlcd-service-supervisor.feature
//!
//! Public API errors use `thiserror` (workspace rule: no `anyhow::Error` in
//! public APIs). Consumers (RLCD-002/003/004) treat every variant as a
//! fail-open signal: structured error / warn-log / skip-stage — never a
//! hard block.

use thiserror::Error;

/// Errors surfaced by the RLCD supervisor (health checks, port scanning,
/// local auto-spawn, pidfile handling).
#[derive(Debug, Error)]
pub enum RlcdError {
    /// The RLCD endpoint could not be reached (connection refused, timeout,
    /// unroutable host).
    #[error("RLCD unreachable at {url}: {detail}")]
    Unreachable {
        /// The endpoint that was probed.
        url: String,
        /// Transport-level detail (e.g. "connection refused", "timed out").
        detail: String,
    },

    /// The endpoint answered but the /health payload is not the ready shape
    /// (wrong `status`, missing/empty `model`).
    #[error("RLCD health response invalid: {detail}")]
    Health {
        /// What was wrong with the /health payload.
        detail: String,
    },

    /// Every port in the scan window is occupied and none answers with the
    /// RLCD ready shape — the local spawn cannot proceed (fail-open).
    #[error("all {limit} ports from {base} are occupied (or not RLCD); no free port for the local RLCD server")]
    NoFreePort {
        /// First port of the scan window.
        base: u16,
        /// Number of ports scanned.
        limit: usize,
    },

    /// The configured spawn binary does not resolve on PATH.
    #[error("rlcd binary '{binary}' not found on PATH; cannot spawn a local RLCD server")]
    BinaryNotFound {
        /// The binary name that failed to resolve.
        binary: String,
    },

    /// The server answered 422 — the request is invalid (schema, model
    /// mismatch, token/branch limits). The detail is the first server-side
    /// error detail, verbatim (the agent's own fault — make it actionable).
    #[error("RLCD rejected the request (422): {detail}")]
    Validation {
        /// The server's first detail message, verbatim.
        detail: String,
    },

    /// The server answered 429 — its scoring queue is full (retry later).
    #[error("RLCD queue busy, retry in {retry_after}s")]
    Busy {
        /// Seconds to wait, from the `Retry-After` header (default 1).
        retry_after: u64,
    },

    /// A 5xx response or a transport failure that was NOT a connect error
    /// (timeout, body read failure, malformed success payload).
    #[error("RLCD server failure: {detail}")]
    Server {
        /// What went wrong (status, timeout, read detail).
        detail: String,
    },

    /// Filesystem failure (pidfile, serve log, config write).
    #[error("IO error in RLCD supervisor: {0}")]
    Io(#[from] std::io::Error),
}
