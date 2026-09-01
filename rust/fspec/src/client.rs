//! Client mode (RPC-010): frontend-only WS attach to a running daemon.
//!
//! Sequence (per architecture note [1]):
//!   1. resolve connect URL: explicit `--connect` or read daemon.json
//!      autodiscovery (rule [10]). Fail-fast if neither.
//!   2. init_tracing_client → log file under ~/.fspec/client.log only.
//!   3. construct the WebSocket backend via `WebSocketFspecBackend::connect`.
//!   4. construct + bootstrap + run the App (rule [11]; same App as combined mode).
//!   5. Reconnect on `r`: out of scope for THIS file — App owns the
//!      disconnect dialog + r-press reconnect. The single-attempt
//!      semantic is the App's responsibility (RPC-011 will harden it).

use std::pin::Pin;
use std::sync::Arc;

use anyhow::{Context, Result};
use codelet_fspec_tui::{App, WebSocketFspecBackend};
use url::Url;

use crate::common::{self, ShutdownReason};

pub async fn run(connect: Option<String>) -> Result<()> {
    common::install_panic_hook();
    common::init_tracing_client()?;

    // BUG-167: initialise the process-global data directory BEFORE the App is
    // constructed — the same single source of truth the combined/daemon
    // entry point sets via build_service. Shared-config persistence
    // (fspec-config.json: tui.mux, tui.lastUsedModel, …) resolves its
    // user-scope path from this global; without it a client-mode
    // `/mux save` / mux-exit auto-save silently no-ops.
    let data_dir = common::home_fspec_dir()?;
    codelet_common::set_data_directory(data_dir)
        .map_err(|e| anyhow::anyhow!("codelet_common::set_data_directory: {e}"))?;

    // CLI-015: this process runs native agent sessions with the AstGrep rig
    // tool (over the WebSocket transport), so core prompts render their
    // harness (capture) variants.
    std::env::set_var("FSPEC_CAPTURE_MODE", "1");

    let shutdown: Pin<Box<dyn std::future::Future<Output = Result<ShutdownReason>> + Send>> =
        Box::pin(common::build_shutdown_future());

    let url = resolve_connect_url(connect)?;
    tracing::info!(url = %url, "fspec client connecting");

    // RPC-011 rule [21] / [22]: construct the backend via plain
    // `connect()` FIRST so `App` construction honours the RPC-010
    // source-shape contract (one call site, App owns its action bus
    // internally). THEN call
    // `supervisor_handle.start(url, app.action_tx_clone())` so the
    // transport-layer reconnect supervisor publishes
    // `Action::Disconnected / Reconnecting(n) / Reconnected` onto the
    // SAME action bus the App is draining — without this wiring the
    // disconnect dialog would never appear in the released binary.
    let backend = WebSocketFspecBackend::connect(url.clone())
        .await
        .context("WebSocketFspecBackend::connect")?;
    let supervisor_handle = backend.supervisor_handle();
    let mut app = App::new(Arc::new(backend));
    supervisor_handle.start(url, app.action_tx_clone());
    app.bootstrap().await?;
    drive_run(app, shutdown).await
}

/// Race `App::run` against the shutdown future.
///
/// Mirror of `combined::drive_run`: if `App::run` returns an error
/// before any shutdown signal is delivered — which is the case in
/// headless test environments where `/dev/tty` is not available
/// (`ENXIO` from `enable_raw_mode`) — fall back to blocking on the
/// shutdown future so the WS connection (which IS what the
/// client-mode integration tests observe) stays alive until the
/// test sends `SIGINT`/`SIGTERM`. The `App::run` path on a real
/// terminal is the normal interactive case; this fallback just keeps
/// the client useful as a verifiable subprocess from pipe-based
/// harnesses.
async fn drive_run(
    app: App,
    mut shutdown: Pin<Box<dyn std::future::Future<Output = Result<ShutdownReason>> + Send>>,
) -> Result<()> {
    tokio::select! {
        r = app.run() => match r {
            Ok(()) => Ok(()),
            Err(e) => {
                tracing::warn!(error = %e, "App::run failed; entering headless fallback (stdin-driven quit)");
                tokio::select! {
                    _ = stdin_quit_signal() => Ok(()),
                    sig = &mut shutdown => sig.map(|_| ()),
                }
            }
        },
        sig = &mut shutdown => sig.map(|_| ()),
    }
}

/// Future that completes when stdin yields a `'q'` byte OR when stdin
/// is closed at startup (immediate EOF — the `.output()` test-harness
/// pattern sets stdin to `Stdio::null()`).
///
/// Used by the headless fallback in [`drive_run`] so a pipe-based
/// test harness (no `/dev/tty`) can still drive `q to quit` semantics
/// from the disconnect dialog. Any non-'q' byte is consumed silently —
/// `'r'` for reconnect is App-loop responsibility and not implemented
/// in the fallback (RPC-011 will harden full reconnect with a TTY).
///
/// Behavior on EOF:
/// * Immediate EOF (no bytes read at all) → return Ok so callers using
///   `Command::output()` (which sets stdin = Stdio::null()) don't hang
///   forever. Hitting this path also implies App::run failed AND no
///   interactive input is available — the only sensible reaction is to
///   let the process exit.
/// * Late EOF (after at least one byte was read) → block forever so
///   the reconnect-bootstrap test pattern (writer drops stdin after
///   pressing `r`) still keeps the client subprocess alive until the
///   real shutdown signal fires.
async fn stdin_quit_signal() {
    use tokio::io::{AsyncReadExt, BufReader};
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut buf = [0u8; 64];
    let mut bytes_seen = false;
    loop {
        match reader.read(&mut buf).await {
            Ok(0) => {
                if !bytes_seen {
                    // Stdin closed at startup (Stdio::null() pattern).
                    // Exit promptly so test harnesses don't hang.
                    return;
                }
                // Late EOF after at least one byte — block forever so
                // the outer `select!` keeps observing the shutdown
                // future and the test's eventual `client_child.kill()`
                // can still terminate the process cleanly.
                std::future::pending::<()>().await;
            }
            Ok(n) => {
                bytes_seen = true;
                if buf[..n].contains(&b'q') {
                    return;
                }
            }
            Err(_) => {
                std::future::pending::<()>().await;
            }
        }
    }
}

fn resolve_connect_url(explicit: Option<String>) -> Result<Url> {
    if let Some(s) = explicit {
        return Url::parse(&s).with_context(|| format!("--connect URL parse: {s}"));
    }
    let djson = common::daemon_json_path()?;
    // RPC-011 rule [20]: verify the pid is alive (and delete stale files)
    // BEFORE trusting the URL.
    let handshake = common::read_and_verify_daemon_json(&djson).with_context(|| {
        // Surface a hint about the `--connect` escape hatch so users
        // who don't have a daemon running know they can still point
        // the client at an explicit WS URL.
        "(use `--connect <ws-url>` to bypass daemon.json autodiscovery)"
    })?;
    Url::parse(&format!("ws://127.0.0.1:{}", handshake.port)).context("synthesize ws url")
}
