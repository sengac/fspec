//! RLCD-001 — local auto-spawn: port scanning + detached spawn.
//!
//! Feature: spec/features/rlcd-service-supervisor.feature
//!
//! Auto-spawn is permitted ONLY for local URLs (127.0.0.1/localhost) whose
//! configured binary resolves on PATH. Remote hosts are connect-only (never
//! spawned). The spawn is detached — own process group, null stdin,
//! stdout/err appended to `<user_dir>/logs/rlcd-serve.log` — so the server
//! survives the fspec process. On non-Unix platforms the detach is
//! Unix-shaped and the spawn path degrades to connect-only (documented in
//! rule 4).
//!
//! Pidfile / liveness helpers live in [`crate::rlcd::pidfile`].

use std::net::SocketAddrV4;

use tracing::{debug, info, warn};

use crate::rlcd::config::{RlcdConfig, user_dir};
use crate::rlcd::error::RlcdError;
use crate::rlcd::pidfile::{
    is_local_url, is_pid_alive, pidfile_path, read_pidfile, remove_stale_pidfile, resolve_binary,
    url_port,
};
use crate::rlcd::health::health_check;

/// Health-probe timeout used while scanning occupied ports.
const PORT_PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(300);

/// Budget for waiting on a live-but-loading pid before giving up on it.
const LIVE_PID_WAIT: std::time::Duration = std::time::Duration::from_secs(10);

/// Budget for the first model load / checkpoint download after spawn.
const SPAWN_LOAD_BUDGET: std::time::Duration = std::time::Duration::from_secs(60);

/// Where a local-server ensure attempt ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnsureOutcome {
    /// A new server process was started on this port and its pid recorded in
    /// the pidfile (`pid == 0` marks "no process started by us" — also used
    /// when a healthy server already answers the configured URL).
    Spawned { port: u16, pid: u32 },
    /// A pidfile pointed at a live process whose /health was still down
    /// (loading) — a bounded wait happened and no second server was
    /// started.
    WaitingExisting,
    /// The configured URL is remote (or spawn disabled) — connect-only,
    /// nothing local was touched.
    NoLocalSpawn,
    /// The spawn flow could not proceed (all ports occupied, binary
    /// missing, spawn failure) — structured reason for the fail-open
    /// policy.
    Failed(String),
}

/// Whether the configured endpoint is ALREADY serving a healthy RLCD server
/// (a short /health probe). A healthy configured endpoint must never
/// trigger a spawn — fspec simply connects to it.
async fn configured_url_is_ready(url: &str) -> bool {
    health_check(url, PORT_PROBE_TIMEOUT).await.is_ok()
}

// ============================================================================
// port scan
// ============================================================================

/// The result of scanning the port window for a spawn target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortSelection {
    /// A free port was found — spawn here.
    SpawnFree { port: u16 },
    /// A port is occupied by a healthy RLCD server — connect to it, do NOT
    /// spawn.
    ReuseExisting { port: u16, model: String },
}

/// A port is "occupied" when a probe bind on 127.0.0.1:port fails
/// (rule 5 — the TcpListener probe bind).
async fn is_port_occupied(port: u16) -> bool {
    match tokio::net::TcpListener::bind(SocketAddrV4::new(std::net::Ipv4Addr::LOCALHOST, port))
        .await
    {
        Ok(_listener) => false,
        Err(_) => true,
    }
}

/// Scan `spawn.port..(+spawn.port_scan_limit-1)` for either a free port or
/// an occupied port whose /health returns the RLCD ready shape (a healthy
/// existing server — reuse it, never spawn a second one).
pub async fn select_port(cfg: &RlcdConfig) -> Result<PortSelection, RlcdError> {
    let limit = cfg.spawn.port_scan_limit.max(1);
    for offset in 0..limit {
        let offset_u16 = u16::try_from(offset).unwrap_or(u16::MAX);
        let port = cfg
            .spawn
            .port
            .checked_add(offset_u16)
            .filter(|p| *p > 0)
            .unwrap_or(cfg.spawn.port);
        if !is_port_occupied(port).await {
            return Ok(PortSelection::SpawnFree { port });
        }
        // The port is taken — is the occupant a healthy RLCD server?
        let probe_url = format!("http://127.0.0.1:{port}");
        if let Ok(model) = health_check(&probe_url, PORT_PROBE_TIMEOUT).await {
            info!(port, %model, "rlcd: reusing existing healthy server on occupied port");
            return Ok(PortSelection::ReuseExisting { port, model });
        }
    }
    Err(RlcdError::NoFreePort {
        base: cfg.spawn.port,
        limit,
    })
}

// ============================================================================
// ensure-local-server
// ============================================================================

/// Ensure a local RLCD server exists for a local config:
///
/// 1. Remote URL (or `enabled == false`) → `NoLocalSpawn` (connect-only).
/// 2. A healthy server already answers the configured URL → connect to it
///    (`Spawned { pid: 0 }` — nothing started by us).
/// 3. Live pid in the pidfile → bounded wait for its model load, then
///    `WaitingExisting` (never double-spawn).
/// 4. Stale pidfile → removed. Port scan → reuse a healthy occupant, or
///    spawn on a free port (or `Failed` with a structured reason — the
///    fail-open policy).
pub async fn ensure_local_server(cfg: &RlcdConfig) -> EnsureOutcome {
    if !cfg.enabled || !is_local_url(&cfg.url) {
        return EnsureOutcome::NoLocalSpawn;
    }
    // (2) healthy configured endpoint: connect to it — never spawn, never
    // touch the pidfile.
    if configured_url_is_ready(&cfg.url).await {
        return EnsureOutcome::Spawned {
            port: url_port(&cfg.url),
            pid: 0,
        };
    }
    let pidfile = pidfile_path();

    // (3) live pid: wait within the budget, never double-spawn.
    if let Some(pid) = read_pidfile(&pidfile) {
        if is_pid_alive(pid) {
            info!(pid, "rlcd: live pid in pidfile — waiting for model load (bounded)");
            let deadline = std::time::Instant::now() + LIVE_PID_WAIT;
            while std::time::Instant::now() < deadline {
                if health_check(&cfg.url, std::time::Duration::from_secs(2)).await.is_ok() {
                    return EnsureOutcome::WaitingExisting;
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
            return EnsureOutcome::WaitingExisting;
        }
        // (4) stale pidfile: remove, then fall through to scan + spawn.
        remove_stale_pidfile(&pidfile);
    }

    // Port scan BEFORE touching the binary: the "all ports occupied"
    // failure must be reported as a port problem even when the binary is
    // missing (the structured reason drives the fail-open consumers).
    let selected = match select_port(cfg).await {
        Ok(sel) => sel,
        Err(RlcdError::NoFreePort { base, limit }) => {
            let reason = format!(
                "all {limit} ports from {base} are occupied by non-RLCD services — no free port for the local RLCD server (fail-open)"
            );
            warn!(%reason, "rlcd: spawn failed");
            return EnsureOutcome::Failed(reason);
        }
        Err(e) => return EnsureOutcome::Failed(format!("rlcd port scan error: {e}")),
    };

    match selected {
        PortSelection::ReuseExisting { port, model } => {
            // A healthy server is already there: the status poller connects
            // to it; no second process is ever started.
            info!(port, %model, "rlcd: existing healthy server found during port scan");
            EnsureOutcome::Spawned {
                port,
                pid: 0, // 0 = no process started by us; the pidfile is untouched
            }
        }
        PortSelection::SpawnFree { port } => {
            let binary = match resolve_binary(&cfg.spawn.binary) {
                Some(b) => b,
                None => {
                    let reason = format!(
                        "rlcd binary '{}' not found on PATH — cannot spawn a local server (fail-open)",
                        cfg.spawn.binary
                    );
                    warn!(%reason, "rlcd: spawn skipped");
                    return EnsureOutcome::Failed(reason);
                }
            };
            spawn_server(cfg, &binary, &pidfile, port).await
        }
    }
}

/// Spawn the detached server on `port`, write the pidfile, and poll /health
/// for the model load (up to 60s — first-time checkpoint download).
async fn spawn_server(
    cfg: &RlcdConfig,
    binary: &std::path::Path,
    pidfile: &std::path::Path,
    port: u16,
) -> EnsureOutcome {
    if let Some(parent) = pidfile.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            warn!(error = %e, "rlcd: failed to create rlcd dir: {e}");
        }
    }
    // serve log: <user_dir>/logs/rlcd-serve.log (append, for BOTH stdout
    // and stderr — rule 6: two independent open handles).
    let serve_log = user_dir()
        .unwrap_or_else(|| std::path::PathBuf::from(".fspec"))
        .join("logs")
        .join("rlcd-serve.log");
    if let Some(parent) = serve_log.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let mut cmd = std::process::Command::new(binary);
    cmd.args(["serve", "--host", &cfg.spawn.bind_address, "--port", &port.to_string()]);
    cmd.stdin(std::process::Stdio::null());
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&serve_log)
    {
        Ok(file) => {
            cmd.stdout(std::process::Stdio::from(file));
            match std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&serve_log)
            {
                Ok(stderr_file) => {
                    cmd.stderr(std::process::Stdio::from(stderr_file));
                }
                Err(e) => {
                    warn!(error = %e, "rlcd: could not open stderr log; stderr goes to null");
                    cmd.stderr(std::process::Stdio::null());
                }
            }
        }
        Err(e) => {
            warn!(error = %e, "rlcd: could not open serve log; child output goes to null");
            cmd.stdout(std::process::Stdio::null());
            cmd.stderr(std::process::Stdio::null());
        }
    }
    #[cfg(unix)]
    use std::os::unix::process::CommandExt as _;
    #[cfg(unix)]
    {
        cmd.process_group(0); // own process group → survives the fspec process
    }

    let child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            let reason = format!("rlcd spawn failed: {e} (fail-open)");
            warn!(%reason, "rlcd: spawn failed");
            return EnsureOutcome::Failed(reason);
        }
    };
    let pid = child.id();
    let _ = std::fs::write(pidfile, pid.to_string());
    info!(pid, port, "rlcd: spawned local server (detached, own process group)");

    // Poll /health for the model load / first-time checkpoint download.
    let deadline = std::time::Instant::now() + SPAWN_LOAD_BUDGET;
    let probe_url = format!("http://127.0.0.1:{port}");
    while std::time::Instant::now() < deadline {
        if health_check(&probe_url, std::time::Duration::from_secs(2)).await.is_ok() {
            return EnsureOutcome::Spawned { port, pid };
        }
        // The process died in the meantime → clean the pidfile.
        if !is_pid_alive(pid) {
            let _ = std::fs::remove_file(pidfile);
            let reason = format!(
                "rlcd server process (pid {pid}) exited during startup — see {serve_log:?} (fail-open)"
            );
            warn!(%reason, "rlcd: spawned server exited early");
            return EnsureOutcome::Failed(reason);
        }
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    }
    // Model not ready within 60s: keep the process (it may still be
    // loading); the status poller keeps probing in the background.
    debug!(port, "rlcd: model load exceeded 60s; continuing in background");
    EnsureOutcome::Spawned { port, pid }
}
