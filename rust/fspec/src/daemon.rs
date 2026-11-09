//! Daemon mode (RPC-010 + RPC-011 hardening): headless WS server only.
//!
//! RPC-011 sequence (per architecture notes [1] + [5] + [6]):
//!   1. RPC-456: a non-loopback `--bind` needs TLS and a token (rule [21])
//!   2. resolve workspace
//!   3. `common::build_service(workspace)` → one Arc<SharedFspecService>
//!   4. install init_tracing_daemon
//!   5. `bind_and_serve(bind, service)` → start WS server (sets stats handle)
//!   6. print port on STDOUT (RPC-005 contract)
//!   7. optionally write `--pidfile`
//!   8. write daemon.json (now with pid + started_at + version)
//!   9. enter SIGNAL LOOP — drives ShutdownReason variants:
//!         * Sigint / Sigterm  → request_shutdown(stats) + drain + exit 0
//!         * Sighup            → log + rebuild watcher + continue loop
//!  10. on exit: remove pidfile + daemon.json + exit 0

use std::path::PathBuf;
use std::process;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use anyhow::Result;
use codelet_rpc_server::{bind_and_serve, bind_and_serve_tls, request_shutdown};

use crate::common::{self, ShutdownReason};

/// RPC-456: `tls` is the `(certificate, key)` PEM pair from --tls-cert/--tls-key.
/// BUG-595: `tls_name` is --tls-name, the host the certificate is for.
pub async fn run(
    workspace: Option<PathBuf>,
    bind: String,
    pidfile: Option<PathBuf>,
    tls: Option<(PathBuf, PathBuf)>,
    tls_name: Option<String>,
) -> Result<()> {
    // RPC-011 race-fix: install signal listeners FIRST, before any
    // async I/O. A SIGTERM/SIGINT that arrives during `bind_and_serve`
    // (after the test harness reads the port and immediately signals)
    // must reach a tokio listener — without this, the default libc
    // handler kills the process with the signal's exit code.
    let mut shutdown_signals = common::ShutdownSignals::install()?;

    // RPC-456: a network bind needs TLS AND a token; bad TLS files or a
    // non-Unicode / empty token stop the daemon with the reason.
    let token = common::daemon_token()?;
    codelet_rpc_server::tls::check_bind_security(&bind, tls.is_some(), token.is_some())?;
    // BUG-595: what the discovery entries advertise, decided before binding.
    let listener = common::discovery_listener(&bind, tls.is_some(), tls_name.as_deref())?;
    let acceptor = match &tls {
        Some((cert, key)) => Some(codelet_rpc_server::tls::load_tls_acceptor(cert, key)?),
        None => None,
    };

    // CLI-015: this process hosts native agent sessions with the AstGrep rig
    // tool, so core prompts render their harness (capture) variants.
    std::env::set_var("FSPEC_CAPTURE_MODE", "1");

    let workspace = common::resolve_workspace(workspace)?;
    let service = common::build_service(&workspace)?;
    // RPC-447: FSPEC_DAEMON_TOKEN requires `Authorization: Bearer <token>`.
    if let Some(token) = token {
        service.set_auth_token(token);
    }
    common::init_tracing_daemon(&service)?;
    // RPC-442: every client's workspace list includes the start workspace.
    if let Err(error) = codelet_common::workspace_registry::open(&workspace) {
        tracing::warn!(%error, "failed to record the start workspace (continuing)");
    }

    let (addr, stats, join) = match acceptor {
        Some(acceptor) => bind_and_serve_tls(&bind, Arc::clone(&service), acceptor).await?,
        None => bind_and_serve(&bind, Arc::clone(&service)).await?,
    };

    println!("{}", addr.port());
    use std::io::Write;
    std::io::stdout().flush().ok();

    if let Some(pf) = pidfile.as_deref() {
        common::write_pidfile(pf, process::id(), addr.port())?;
    }

    let djson = common::daemon_json_path()?;
    let started_at = SystemTime::now();
    // BUG-289: never take daemon.json from another live daemon.
    if let Err(e) = common::claim_default_pointer(
        &djson,
        addr.port(),
        process::id(),
        &workspace,
        started_at,
        &listener,
    ) {
        tracing::warn!(error = %e, "failed to write daemon.json (continuing)");
    }
    // RPC-446: advertise this daemon in its own discovery file so clients
    // can list every local daemon, not only the daemon.json default.
    let daemons_dir = common::daemons_dir()?;
    if let Err(e) = common::write_daemon_entry(
        &daemons_dir,
        addr.port(),
        process::id(),
        &workspace,
        started_at,
        &listener,
    ) {
        tracing::warn!(error = %e, "failed to write the daemon discovery file (continuing)");
    }

    tracing::info!(workspace = %workspace.display(), addr = %addr, "fspec daemon listening");

    // RPC-011 signal loop: SIGHUP rebuilds watcher and continues;
    // SIGINT/SIGTERM drains and exits.
    loop {
        let reason = shutdown_signals.next().await;
        match reason {
            ShutdownReason::Sighup => {
                tracing::info!("SIGHUP: re-reading workspace");
                // RPC-011 rule [25]/[26]: build a fresh `WorkUnitsWatcher`
                // against the SAME workspace path and atomically swap it
                // into `SharedFspecService` via `rebuild_watcher`. The
                // ArcSwap-backed watcher slot makes the swap lock-free
                // and immediately visible to concurrent
                // `list_work_units` readers.
                match codelet_core::work_units::WorkUnitsWatcher::new(&workspace) {
                    Ok(new_watcher) => {
                        service.rebuild_watcher(new_watcher);
                        tracing::info!("SIGHUP: watcher rebuilt");
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "SIGHUP: failed to rebuild watcher; keeping existing");
                    }
                }
                continue;
            }
            ShutdownReason::Sigint | ShutdownReason::Sigterm => {
                tracing::info!(?reason, "shutdown signal received; draining");
                break;
            }
        }
    }

    // RPC-011 rule [28] Option B drain: notify shutdown_signal so each
    // per-connection task sends Close{going_away} + breaks. Then give
    // a short grace window (500ms) for the Close frames to flush, and
    // abort the accept loop's JoinHandle — `bind_and_serve` is a
    // never-ending `loop { listener.accept().await; … }`, so join.await
    // would block forever in the happy path. Abort is the documented
    // shutdown path (RPC-005) and architecture note [28] selects it as
    // Option B's minimum-viable drain.
    request_shutdown(&stats);
    tokio::time::sleep(Duration::from_millis(500)).await;
    join.abort();
    let _ = tokio::time::timeout(Duration::from_secs(1), join).await;
    tracing::info!("drain complete");

    if let Some(pf) = pidfile.as_deref() {
        common::remove_pidfile(pf);
    }
    common::remove_daemon_entry(&daemons_dir, process::id());
    // BUG-289: release daemon.json only if it is ours, handing it on.
    if let Err(e) = common::release_default_pointer(&djson, &daemons_dir, process::id()) {
        tracing::warn!(error = %e, "failed to release daemon.json (continuing)");
    }
    Ok(())
}
