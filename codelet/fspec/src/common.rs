//! Shared startup helpers for the `fspec` binary (RPC-010).
//!
//! Single source of truth for:
//!   - `build_service(workspace)` — constructs WorkUnitsWatcher +
//!     SharedFspecService exactly once per process (RPC-010 rule [3]).
//!   - `init_tracing_*()` — three subscriber variants (combined / daemon /
//!     client) per architecture note [7]. The registry is built FIRST,
//!     then `register_log_layer(service)` is called to push the sender
//!     onto codelet-rpc's process-global Vec.
//!   - `install_panic_hook()` — idempotent ratatui restoration so a
//!     panic in TUI mode (combined / client) doesn't leave the alt-screen
//!     borked.
//!   - daemon.json autodiscovery — atomic write + idempotent remove.
//!   - workspace + non-loopback bind validation.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Once;

use anyhow::{anyhow, Context, Result};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::{register_log_layer, BroadcastLogLayer, SharedFspecService};
use serde::Deserialize;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

/// RPC-011 rule [21] / scenario "daemon.json schema upgrade": the
/// authoritative client-side view of daemon.json. `read_and_verify_daemon_json`
/// returns this on success.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct DaemonHandshake {
    pub port: u16,
    pub pid: u32,
    pub started_at: Option<String>,
    pub version: Option<String>,
}

/// RPC-011 enum return for `build_shutdown_future` — distinguishes
/// SIGINT/SIGTERM (drain + exit) from SIGHUP (rebuild watcher, keep
/// running).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownReason {
    Sigint,
    Sigterm,
    Sighup,
}

/// Build the single SharedFspecService for this process.
///
/// RPC-015 / RPC-017 fix: attach the workspace cwd via
/// [`SharedFspecService::with_cwd`] so that cwd-dependent RPC methods
/// (`checkpoint_counts`, `move_work_unit_up`, `move_work_unit_down`)
/// can locate the workspace.
///
/// RPC-025 fix: initialise the global data directory via
/// [`codelet_common::set_data_directory`] BEFORE constructing the
/// watcher or service. Without this, `HistoryStore::new()` returns
/// `Err("Data directory not initialized")` and `dispatch_rpc025`
/// silently swallows the error — making Shift+↑/↓ appear inert in the
/// live binary. `build_service` is the chokepoint for both
/// `combined::run` and `daemon::run`; `client::run` does NOT call it
/// (client mode inherits the daemon's data dir over tarpc).
pub fn build_service(workspace: &Path) -> Result<Arc<SharedFspecService>> {
    // RPC-025: initialise the global data directory BEFORE any
    // persistence-touching code path can observe it.
    let data_dir = home_fspec_dir()?;
    codelet_common::set_data_directory(data_dir)
        .map_err(|e| anyhow!("codelet_common::set_data_directory: {e}"))?;

    let watcher = Arc::new(
        WorkUnitsWatcher::new(workspace)
            .with_context(|| format!("WorkUnitsWatcher::new({})", workspace.display()))?,
    );
    Ok(Arc::new(
        SharedFspecService::new(watcher).with_cwd(workspace.to_path_buf()),
    ))
}

fn env_filter() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
}

/// Combined mode: tracing → rolling file under ~/.fspec/logs/ AND the
/// LogEvent broadcast inside SharedFspecService. NO stderr fmt subscriber
/// (stderr is reserved for the PORT banner + panic backtraces).
pub fn init_tracing_combined(service: &Arc<SharedFspecService>) -> Result<()> {
    let logs_dir = home_fspec_dir()?.join("logs");
    fs::create_dir_all(&logs_dir).with_context(|| format!("mkdir {}", logs_dir.display()))?;
    let appender = tracing_appender::rolling::daily(&logs_dir, "fspec-combined.log");
    let (nb, guard) = tracing_appender::non_blocking(appender);
    keep_guard(guard);
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(nb)
        .with_ansi(false);
    tracing_subscriber::registry()
        .with(BroadcastLogLayer)
        .with(file_layer)
        .with(env_filter())
        .init();
    let _ = register_log_layer(Arc::clone(service));
    Ok(())
}

/// Daemon mode: tracing → stderr fmt (RPC-005 pattern) AND the LogEvent
/// broadcast. Registry built FIRST, register_log_layer called SECOND so
/// only the sender-push side-effect of register_log_layer lands.
pub fn init_tracing_daemon(service: &Arc<SharedFspecService>) -> Result<()> {
    let fmt_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stderr);
    tracing_subscriber::registry()
        .with(BroadcastLogLayer)
        .with(fmt_layer)
        .with(env_filter())
        .init();
    let _ = register_log_layer(Arc::clone(service));
    Ok(())
}

/// Client mode: tracing → rolling file under ~/.fspec/client.log; no
/// SharedFspecService exists here (the daemon owns it).
pub fn init_tracing_client() -> Result<()> {
    let dir = home_fspec_dir()?;
    fs::create_dir_all(&dir).with_context(|| format!("mkdir {}", dir.display()))?;
    let appender = tracing_appender::rolling::daily(&dir, "client.log");
    let (nb, guard) = tracing_appender::non_blocking(appender);
    keep_guard(guard);
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(nb)
        .with_ansi(false);
    tracing_subscriber::registry()
        .with(file_layer)
        .with(env_filter())
        .init();
    Ok(())
}

/// Idempotent panic hook that restores the terminal BEFORE the default
/// hook prints the backtrace, so a panic during TUI mode doesn't leave
/// the alt-screen wedged.
pub fn install_panic_hook() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            ratatui::restore();
            let _ = crossterm::execute!(
                std::io::stdout(),
                crossterm::event::DisableMouseCapture,
                crossterm::event::DisableBracketedPaste,
            );
            prev(info);
        }));
    });
}

/// Resolve the workspace root: explicit `--workspace` or current dir.
pub fn resolve_workspace(opt: Option<PathBuf>) -> Result<PathBuf> {
    match opt {
        Some(p) => Ok(p),
        None => std::env::current_dir().context("std::env::current_dir"),
    }
}

/// REJECT non-loopback bind addresses at clap-arg validation (rule [21]).
pub fn validate_loopback_bind(bind: &str) -> Result<()> {
    let host = bind
        .rsplit_once(':')
        .map(|(h, _)| h)
        .unwrap_or(bind)
        .trim_matches(|c| c == '[' || c == ']');
    let ok = matches!(host, "127.0.0.1" | "::1" | "localhost");
    if !ok {
        return Err(anyhow!(
            "error: --bind must be a loopback address (127.0.0.1, ::1, or localhost); auth/TLS for external binds is out of scope (future card)"
        ));
    }
    Ok(())
}

/// Resolve `daemon.json` path: `$XDG_RUNTIME_DIR/fspec/daemon.json` if
/// set, else `~/.fspec/daemon.json`.
pub fn daemon_json_path() -> Result<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_RUNTIME_DIR") {
        let p = PathBuf::from(xdg);
        if !p.as_os_str().is_empty() {
            return Ok(p.join("fspec").join("daemon.json"));
        }
    }
    Ok(home_fspec_dir()?.join("daemon.json"))
}

fn home_fspec_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("cannot resolve home directory"))?;
    Ok(home.join(".fspec"))
}

/// Atomic write daemon.json (temp+rename). RPC-011 schema: includes
/// pid + started_at (ISO 8601) + version (CARGO_PKG_VERSION).
pub fn write_daemon_json(
    path: &Path,
    port: u16,
    pid: u32,
    workspace: &Path,
    started_at: std::time::SystemTime,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let started_at_iso = system_time_iso8601(started_at);
    let body = serde_json::json!({
        "port": port,
        "pid": pid,
        "workspace": workspace.canonicalize().unwrap_or_else(|_| workspace.to_path_buf()),
        "started_at": started_at_iso,
        "version": env!("CARGO_PKG_VERSION"),
    });
    let tmp = path.with_extension("json.tmp");
    let mut f = fs::File::create(&tmp).with_context(|| format!("create {}", tmp.display()))?;
    f.write_all(serde_json::to_string_pretty(&body)?.as_bytes())?;
    f.sync_all()?;
    drop(f);
    fs::rename(&tmp, path).with_context(|| format!("rename {} -> {}", tmp.display(), path.display()))?;
    Ok(())
}

/// Render a `SystemTime` as an ISO 8601 / RFC 3339 UTC string with
/// second precision (e.g. `2026-05-11T12:34:56Z`). Avoids a
/// chrono dep by going through humantime's RFC3339 formatter.
fn system_time_iso8601(t: std::time::SystemTime) -> String {
    let duration = t
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs() as i64;
    // Minimal RFC 3339 formatter — pure stdlib so no chrono dep.
    let days_from_epoch = secs / 86_400;
    let mut secs_in_day = (secs % 86_400 + 86_400) % 86_400;
    let hour = secs_in_day / 3_600;
    secs_in_day %= 3_600;
    let minute = secs_in_day / 60;
    let second = secs_in_day % 60;
    let (year, month, day) = civil_from_days(days_from_epoch);
    format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z"
    )
}

/// Howard Hinnant's days_from_civil inverse, in pure stdlib.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u32, d as u32)
}

/// Idempotent remove daemon.json.
pub fn remove_daemon_json(path: &Path) {
    let _ = fs::remove_file(path);
}

/// Write `pid=<u32>\nport=<u16>` to a pidfile.
pub fn write_pidfile(path: &Path, pid: u32, port: u16) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    let body = format!("pid={pid}\nport={port}\n");
    fs::write(path, body).with_context(|| format!("write pidfile {}", path.display()))
}

/// Idempotent remove pidfile.
pub fn remove_pidfile(path: &Path) {
    let _ = fs::remove_file(path);
}

/// Future that completes on SIGINT (ctrl_c), SIGTERM, or SIGHUP. Unix uses the
/// tokio::signal::unix handlers. RPC-011 rule [25] / scenario "SIGHUP
/// rebuilds...": SIGHUP yields `ShutdownReason::Sighup` so the daemon's
/// top-level signal loop can rebuild the watcher and keep running.
///
/// NOTE — race-with-signal limitation: this function installs the
/// tokio signal listeners SYNCHRONOUSLY when called, then returns an
/// `impl Future` that awaits one of them. A signal that arrives BEFORE
/// the first call is dropped via the default handler (process exits
/// with signal code instead of clean exit 0). For long-running daemons,
/// use [`ShutdownSignals::install`] FIRST and call `.next().await` in
/// a loop — that arms the listeners up-front so the bootstrap window
/// is safe.
#[cfg(unix)]
pub fn build_shutdown_future() -> impl std::future::Future<Output = Result<ShutdownReason>> {
    use tokio::signal::unix::{signal, SignalKind};
    let sigterm_result = signal(SignalKind::terminate());
    let sigint_result = signal(SignalKind::interrupt());
    let sighup_result = signal(SignalKind::hangup());
    async move {
        let mut sigterm = sigterm_result?;
        let mut sigint = sigint_result?;
        let mut sighup = sighup_result?;
        tokio::select! {
            _ = sigterm.recv() => Ok(ShutdownReason::Sigterm),
            _ = sigint.recv() => Ok(ShutdownReason::Sigint),
            _ = sighup.recv() => Ok(ShutdownReason::Sighup),
        }
    }
}

#[cfg(not(unix))]
pub fn build_shutdown_future() -> impl std::future::Future<Output = Result<ShutdownReason>> {
    async {
        tokio::signal::ctrl_c().await?;
        Ok(ShutdownReason::Sigint)
    }
}

/// RPC-011: long-lived shutdown-signal handles. Construct via
/// [`ShutdownSignals::install`] BEFORE any work the test harness can
/// race a signal against (in particular before `bind_and_serve`'s
/// async port-bind). Drop only when the daemon is exiting — the
/// underlying `tokio::signal::unix::Signal` registrations are removed
/// on drop, so dropping mid-loop creates a fresh race window.
///
/// Use [`next`](Self::next) in the daemon's top-level loop:
/// SIGINT/SIGTERM should `break`, SIGHUP should `continue` (rebuild
/// watcher and re-await).
#[cfg(unix)]
pub struct ShutdownSignals {
    sigterm: tokio::signal::unix::Signal,
    sigint: tokio::signal::unix::Signal,
    sighup: tokio::signal::unix::Signal,
}

#[cfg(unix)]
impl ShutdownSignals {
    /// Install tokio listeners for SIGTERM, SIGINT, SIGHUP. Returns
    /// `Err` if any of them fails to register (extremely unlikely on a
    /// healthy unix process).
    pub fn install() -> Result<Self> {
        use tokio::signal::unix::{signal, SignalKind};
        Ok(Self {
            sigterm: signal(SignalKind::terminate())?,
            sigint: signal(SignalKind::interrupt())?,
            sighup: signal(SignalKind::hangup())?,
        })
    }

    /// Await one of the three signals and yield the corresponding
    /// [`ShutdownReason`]. Re-callable in a loop — each call awaits
    /// the NEXT signal.
    pub async fn next(&mut self) -> ShutdownReason {
        tokio::select! {
            _ = self.sigterm.recv() => ShutdownReason::Sigterm,
            _ = self.sigint.recv() => ShutdownReason::Sigint,
            _ = self.sighup.recv() => ShutdownReason::Sighup,
        }
    }
}

/// Windows fallback: only ctrl_c is supported (mirrors
/// [`build_shutdown_future`]).
#[cfg(not(unix))]
pub struct ShutdownSignals;

#[cfg(not(unix))]
impl ShutdownSignals {
    pub fn install() -> Result<Self> {
        Ok(Self)
    }
    pub async fn next(&mut self) -> ShutdownReason {
        let _ = tokio::signal::ctrl_c().await;
        ShutdownReason::Sigint
    }
}

// Keep the non-blocking tracing-appender WorkerGuard alive for the whole
// process (otherwise the worker thread is dropped immediately and log
// writes get lost). Stored in a Once-initialised static.
fn keep_guard(guard: tracing_appender::non_blocking::WorkerGuard) {
    use std::sync::Mutex;
    static GUARDS: Mutex<Vec<tracing_appender::non_blocking::WorkerGuard>> =
        Mutex::new(Vec::new());
    if let Ok(mut g) = GUARDS.lock() {
        g.push(guard);
    }
}

/// Read daemon.json and return the `port` field.
///
/// RPC-011 rule [20] keeps this function as a legacy accessor that
/// delegates to `read_and_verify_daemon_json`. Both production
/// callers (`client.rs` and `status.rs`) use `read_and_verify_daemon_json`
/// directly so they can capture the full `DaemonHandshake` struct — but
/// the legacy port-only accessor is preserved so out-of-tree callers
/// (and future scripts) can still extract just the port without pulling
/// in serde_json.
#[allow(dead_code)]
pub fn read_daemon_json_port(path: &Path) -> Result<u16> {
    let handshake = read_and_verify_daemon_json(path)?;
    Ok(handshake.port)
}

/// RPC-011 rule [21]: read daemon.json, parse, then probe `kill(pid, None)`
/// (unix) / `GetExitCodeProcess` (windows) BEFORE trusting any URL.
/// On stale (dead pid) delete the file and return Err with the stable
/// text "no daemon.json found" so callers (client.rs, status.rs) can
/// match it.
pub fn read_and_verify_daemon_json(path: &Path) -> Result<DaemonHandshake> {
    let body = match fs::read_to_string(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(anyhow!(
                "no daemon.json found at {} — start `fspec daemon` first",
                path.display()
            ));
        }
        Err(e) => {
            return Err(anyhow::Error::from(e).context(format!(
                "read daemon.json at {}",
                path.display()
            )));
        }
    };
    let parsed: serde_json::Value = serde_json::from_str(&body).context("parse daemon.json")?;
    let port = parsed
        .get("port")
        .and_then(|p| p.as_u64())
        .ok_or_else(|| anyhow!("daemon.json missing or invalid `port` field"))?
        as u16;
    let pid = parsed
        .get("pid")
        .and_then(|p| p.as_u64())
        .ok_or_else(|| anyhow!("daemon.json missing or invalid `pid` field"))?
        as u32;
    let started_at = parsed
        .get("started_at")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());
    let version = parsed
        .get("version")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());

    if !pid_is_alive(pid) {
        // Stale — delete the file and tell callers no daemon is running.
        let _ = fs::remove_file(path);
        return Err(anyhow!(
            "no daemon.json found (stale daemon.json removed at {}) — start `fspec daemon` first",
            path.display()
        ));
    }

    Ok(DaemonHandshake {
        port,
        pid,
        started_at,
        version,
    })
}

/// Probe whether `pid` is alive without sending a real signal.
#[cfg(unix)]
fn pid_is_alive(pid: u32) -> bool {
    use nix::sys::signal::kill;
    use nix::unistd::Pid;
    match kill(Pid::from_raw(pid as i32), None) {
        Ok(()) => true,
        Err(nix::errno::Errno::EPERM) => true, // owned by another user but exists
        Err(_) => false,
    }
}

#[cfg(not(unix))]
fn pid_is_alive(_pid: u32) -> bool {
    // RPC-011 windows support is best-effort — we conservatively
    // accept the daemon.json. A future card adds GetExitCodeProcess.
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RPC-017 regression: production `build_service` MUST attach the
    /// workspace cwd to the SharedFspecService via `.with_cwd(...)`.
    #[test]
    fn build_service_attaches_workspace_cwd() {
        // @step Given the codelet-fspec binary crate after RPC-017 lands
        let tmp = tempfile::tempdir().expect("tempdir");
        let workspace = tmp.path();

        // @step When common::build_service(workspace) is invoked against a temp workspace path
        let service = build_service(workspace).expect("build_service");

        // @step Then the returned Arc<SharedFspecService>::cwd() returns Some equal to that workspace path
        assert_eq!(
            service.cwd(),
            Some(&workspace.to_path_buf()),
            "build_service must call SharedFspecService::with_cwd(workspace)",
        );

        // @step And codelet/fspec/src/common.rs contains the substring ".with_cwd(workspace.to_path_buf())"
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/common.rs"))
            .expect("read common.rs");
        assert!(
            src.contains(".with_cwd(workspace.to_path_buf())"),
            "common.rs must contain the literal .with_cwd(workspace.to_path_buf()) chain in build_service",
        );
    }

    /// RPC-025 regression: the production `fspec` binary MUST call
    /// `codelet_common::set_data_directory(~/.fspec)` BEFORE exposing
    /// any persistence_*_history RPC method. Without this,
    /// `HistoryStore::new()` returns `Err("Data directory not
    /// initialized")` and `dispatch_rpc025` silently swallows the
    /// error — making Shift+↑/↓ appear inert in the live binary even
    /// though unit tests pass (because they call
    /// `set_data_directory(temp.path())` themselves).
    #[test]
    fn build_service_initializes_global_data_directory_for_persistence() {
        // @step Given the codelet/fspec binary crate after RPC-025 lands
        let tmp = tempfile::tempdir().expect("tempdir");
        let workspace = tmp.path();

        // @step When common::build_service is invoked against a tempdir workspace
        let _service = build_service(workspace).expect("build_service");

        // @step Then codelet_common::get_data_dir() returns Ok with a path ending in ".fspec"
        let data_dir = codelet_common::get_data_dir()
            .expect("codelet_common::get_data_dir() must be Ok after build_service");
        assert!(
            data_dir.ends_with(".fspec"),
            "build_service must initialise the data directory to a `.fspec` path; got {}",
            data_dir.display()
        );

        // @step And codelet/fspec/src/common.rs contains the substring "codelet_common::set_data_directory"
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/common.rs"))
            .expect("read common.rs");
        assert!(
            src.contains("codelet_common::set_data_directory"),
            "common.rs must contain a literal `codelet_common::set_data_directory(...)` call (RPC-025 regression)",
        );

        // @step And the set_data_directory call appears BEFORE the WorkUnitsWatcher::new(workspace) call in build_service
        let set_idx = src
            .find("codelet_common::set_data_directory")
            .expect("set_data_directory call must exist");
        let watcher_idx = src
            .find("WorkUnitsWatcher::new(workspace)")
            .expect("WorkUnitsWatcher::new(workspace) call must exist in build_service");
        assert!(
            set_idx < watcher_idx,
            "set_data_directory must be called BEFORE WorkUnitsWatcher::new in build_service",
        );
    }
}
