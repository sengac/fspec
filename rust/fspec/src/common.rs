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
//!   - daemon.json autodiscovery — atomic write + idempotent remove
//!     (`common/daemon_json.rs`, re-exported here).
//!   - workspace + non-loopback bind validation.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Once;

use anyhow::{anyhow, Context, Result};
use codelet_agent_loop::FspecAgentHooks;
use codelet_common::logging::{
    privacy::{PrivacyFields, PrivacySpanLayer},
    private_files,
};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_core::SessionManagerHandle;
use codelet_fspec_core::FspecCoreError;
use codelet_rpc::{register_log_layer, BroadcastLogLayer, SharedFspecService};
use codelet_sessions::SessionManager;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[path = "common/daemon_json.rs"]
mod daemon_json;
#[path = "common/daemon_listener.rs"]
mod daemon_listener;
#[allow(unused_imports)]
pub use daemon_listener::*;
// Test binaries that `#[path]`-include this module may not use every helper.
#[allow(unused_imports)]
pub use daemon_json::*;
include!("common/daemon_registry.rs");
include!("common/shutdown.rs");
include!("common/tracing_setup.rs");
use tracing_subscriber::EnvFilter;

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
///
/// RPC-044 fix: construct a real `codelet_sessions::SessionManager` and
/// pass it into `SharedFspecService::with_session_manager` as
/// `Arc<dyn SessionManagerHandle>`. After this, the `fspec` binary
/// drives real agent sessions through the NAPI-free `codelet-sessions`
/// crate; AgentView observes the live session manager's `chunks_rx`
/// instead of the empty fallback broadcast. The default
/// `NoopSessionManagerHooks` is left in place — the full agent-loop /
/// scheduler / footer-poller / IsolationStateChange hooks are wired in
/// RPC-045+ when the AgentView is connected to the new RPC surface.
pub fn build_service(workspace: &Path) -> Result<Arc<SharedFspecService>> {
    // RPC-025: initialise the global data directory BEFORE any
    // persistence-touching code path can observe it (including any
    // persistence lazily reached through SessionManager below).
    let data_dir = home_fspec_dir()?;
    codelet_common::set_data_directory(data_dir)
        .map_err(|e| anyhow!("codelet_common::set_data_directory: {e}"))?;

    // RPC-407: initialise the process-global blocklist project root so
    // that `<workspace>/.fspec/blocklist.json` rules are enforced. The
    // `check_bash_command` / `check_file_path` middleware hot-reloads
    // config on every check via the stored root, so setting the root
    // ONCE at startup is sufficient for rule-edit hot-reload. Before
    // RPC-407 only the legacy napi path (rust/napi/src/blocklist.rs)
    // called init_blocklist — the Rust binary silently ignored project
    // blocklist rules and applied only ~/.fspec/blocklist.json.
    // build_service is the shared chokepoint for `daemon` and
    // `combined` modes, so neither entry point can skip it; `client`
    // mode runs no tools locally (the daemon owns the sessions).
    codelet_tools::blocklist::init_blocklist(Some(workspace));

    let watcher = Arc::new(
        WorkUnitsWatcher::new(workspace)
            .with_context(|| format!("WorkUnitsWatcher::new({})", workspace.display()))?,
    );

    // RPC-044: construct a real SessionManager and pass it as a
    // SessionManagerHandle. SharedFspecService::with_session_manager
    // already delegates chunks_rx / logs_rx / status_changes_rx to the
    // attached handle (rust/rpc/src/lib.rs lines 526-580), so the
    // broadcast wiring is complete without any additional fan-out task.
    //
    // RPC-072: install `FspecAgentHooks` (from the NAPI-free
    // `codelet-agent-loop` crate) so the session's `spawn_agent_loop`
    // actually drains `input_rx`, dispatches to the session's
    // `LlmProvider`, and emits `StreamChunk::Text` + `Done` chunks back
    // through `BackgroundSession::handle_output`. Before RPC-072, the
    // hooks impl installed here was the no-op `FspecSessionManagerHooks`
    // — typed input vanished into a dropped channel and the AgentView
    // saw `Running → Idle` with no assistant chunks. The loop-cleanup
    // behaviour that `FspecSessionManagerHooks::cleanup_session_loops`
    // used to provide is preserved 1:1 by
    // `FspecAgentHooks::cleanup_session_loops`.
    let manager = SessionManager::new();
    // PROV-119 / BUG-463: restore the persisted default model; a damaged
    // default-model.json is an error, not a silently dropped default.
    manager
        .restore_persisted_default_model()
        .map_err(|error| anyhow!(error))?;
    // RPC-444: sessions default to the start workspace, not the process cwd.
    manager
        .set_default_project(workspace.to_path_buf())
        .map_err(|error| anyhow!(error))?;
    // RPC-386: populate the self-weak so sessions this daemon manager creates
    // carry an owning-manager back-reference. This makes the AgentManager
    // handler bind to THIS manager (not the global singleton), so spawned
    // subordinates land in the daemon-owned manager.
    manager.set_hooks(Arc::new(FspecAgentHooks::new()));
    // WT-002: register THIS (non-singleton) manager's chunks_tx as the
    // footer-poller emission target. FspecAgentHooks now spawns the shared
    // NAPI-free poller (codelet-sessions::footer_poller), which emits on
    // the registered sender — so isolated (detached-HEAD worktree)
    // sessions show their git state in the TUI footer instead of staying
    // blank for the session's whole life.
    codelet_sessions::footer_poller::register_chunk_sender(manager.chunks_tx().clone());
    // WT-004: register THIS (non-singleton) manager as the tool-callbacks
    // manager and wire the three shared NAPI-free callbacks (GIT-020
    // isolation context, BLOCK-006 work-unit stage, block-notification
    // emitter) into codelet-tools' process-global OnceLocks. Before WT-004
    // these were registered ONLY by the NAPI adapter, so in this binary
    // every tool wrapper's `validate_and_resolve_path` got
    // `isolation_ctx == None` and isolated sessions could read/write ANY
    // path — the worktree was a cosmetic badge. build_service returns
    // before any session exists, so registering here (before the service
    // is handed out) is safe: the OnceLocks accept the first set only and
    // this process never registers a second manager.
    codelet_sessions::session_tool_callbacks::register_manager(Arc::clone(&manager));
    codelet_tools::facade::set_get_effective_cwd_callback(
        codelet_sessions::session_tool_callbacks::isolation_context,
    );
    codelet_tools::facade::set_get_work_unit_stage_callback(
        codelet_sessions::session_tool_callbacks::work_unit_stage,
    );
    codelet_tools::facade::set_block_notification_callback(
        codelet_sessions::session_tool_callbacks::emit_block_notification,
    );

    // RPC-066: under the `test-stub-provider` feature, install the
    // deterministic stub LlmProvider into the process-global in-memory
    // registry and pin the session manager's default model to
    // "stub/canned" so `create_session(role)` over the WS backend
    // creates sessions backed by the stub. The feature is OFF in
    // production builds — this branch compiles out entirely.
    //
    // Architecture notes [E], [J], [K] from RPC-066.
    #[cfg(feature = "test-stub-provider")]
    {
        codelet_providers::stub_provider::register_stub_provider();
        manager
            .set_default_model("stub/canned")
            .map_err(|error| anyhow::anyhow!(error))?;
    }

    let session_manager: Arc<dyn SessionManagerHandle> = manager;

    Ok(Arc::new(
        SharedFspecService::with_session_manager(watcher, session_manager)
            .with_cwd(workspace.to_path_buf()),
    ))
}

/// Resolve the workspace root: explicit `--workspace` or current dir.
pub fn resolve_workspace(opt: Option<PathBuf>) -> Result<PathBuf> {
    match opt {
        Some(p) => Ok(p),
        None => std::env::current_dir().context("std::env::current_dir"),
    }
}

/// RPC-447 / RPC-456: the daemon's bearer token from FSPEC_DAEMON_TOKEN.
/// Unset is `None`; a non-Unicode or empty value is an error, never
/// silently "no token".
pub fn daemon_token() -> Result<Option<String>> {
    match std::env::var("FSPEC_DAEMON_TOKEN") {
        Ok(token) if token.is_empty() => Err(anyhow!("FSPEC_DAEMON_TOKEN is set but empty")),
        Ok(token) => Ok(Some(token)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => {
            Err(anyhow!("FSPEC_DAEMON_TOKEN is not valid Unicode"))
        }
    }
}

/// Resolve the per-user fspec state dir (`~/.fspec`).
///
/// BUG-167: also used by `client::run` to initialise the process-global
/// data directory (the combined/daemon path does the same in
/// `build_service`) so shared-config persistence resolves the same
/// user-scope path in every mode.
pub fn home_fspec_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("cannot resolve home directory"))?;
    Ok(home.join(".fspec"))
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

/// Render an [`FspecCoreError`] for the CLI surface.
///
/// The dispatcher-facing `FspecCoreError::Display` impl is part of the
/// LLM-tool contract and wraps validation failures in
/// `"Invalid args for fspec command <name>: <reason>"`. Shell users
/// (parity target: `node dist/index.js <cmd>`) MUST see the unwrapped
/// `<reason>` only — TS commands `output.error('Error:', error.message)`
/// or `output.error('✗ Failed to <verb>:', error.message)` never include
/// the dispatcher envelope. This helper strips it so every ported
/// bridge can emit byte-parity stderr without duplicating the match
/// boilerplate.
///
/// For non-`InvalidArgs` variants the Display impl is forwarded
/// verbatim — those variants (`Io`, `ParseJson`, `DirectoryNotFound`,
/// `UnknownCommand`, `NotYetPorted`) carry no dispatcher framing.
pub fn render_core_error(err: &FspecCoreError) -> String {
    match err {
        FspecCoreError::InvalidArgs { reason, .. } => reason.clone(),
        _ => err.to_string(),
    }
}

/// Print the DISC-003 `nextSteps` trailer (two lines) with two-space
/// indentation, when the mutation success envelope carries it. Shared by the
/// foundation-domain mutation bridges so the rendering lives in one place
/// (work unit DISC-003, rule 4/14).
pub fn print_next_steps(parsed: &serde_json::Value) {
    if let Some(ns) = parsed.get("nextSteps").and_then(serde_json::Value::as_str) {
        if !ns.is_empty() {
            for line in ns.lines() {
                println!("  {line}");
            }
        }
    }
}

/// Strip the dispatcher's `"Invalid args for fspec command <name>: "`
/// envelope so a CLI bridge that routes through `dispatch_command` can emit
/// only the bare `<reason>` on stderr — exactly what the TS
/// `output.error('✗ Failed to <verb>:', error.message)` path prints.
///
/// `dispatch_command` returns errors as already-rendered Display strings, so
/// validation failures arrive wrapped in the LLM-tool envelope
/// `"Invalid args for fspec command <name>: <reason>"`. The shell user never
/// sees that framing. Non-validation errors carry no envelope and are
/// returned verbatim.
///
/// Shared by every kebab-routed bridge (e.g. `add-bounded-context`,
/// `add-external-system`) so the unwrap logic lives in exactly one place.
pub fn strip_dispatch_envelope(msg: &str) -> &str {
    const PREFIX: &str = "Invalid args for fspec command ";
    match msg.strip_prefix(PREFIX) {
        Some(rest) => rest.split_once(": ").map_or(msg, |(_, after)| after),
        None => msg,
    }
}
include!("common/spec_project_root.rs");
#[cfg(test)]
#[path = "common/tests/mod.rs"]
mod tests;
