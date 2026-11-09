//! Shared NAPI-free footer-state poller (WT-002, originally TUI-091/RPC-043).
//!
//! A per-session background task that reads the session's effective CWD
//! from the footer-cwd registry, polls `get_current_branch`, and emits
//! [`StreamChunk::FooterStateUpdate`] chunks on a registered manager-owned
//! `chunks_tx` broadcast every 5 seconds.
//!
//! Before WT-002 this logic lived ONLY in `rust/napi/src/footer_poller.rs`
//! and `FspecAgentHooks::spawn_footer_poller` (the fspec binary's hook impl)
//! was a no-op — so the `fspec` binary never emitted footer updates and
//! isolated (detached-HEAD worktree) sessions showed a blank branch for the
//! session's whole life.
//!
//! Both front doors now delegate here (two-front-doors rule):
//! - `codelet-agent-loop`'s `FspecAgentHooks` (fspec binary) → this module
//! - `codelet-napi`'s `NapiSessionManagerHooks` → its `crate::footer_poller`
//!   shim → this module
//!
//! The NAPI shape tests pin the shim's delegation
//! (`crate::footer_poller::spawn_footer_poller`), which remains true.
//!
//! The poison handling mirrors the napi-side original (poisoned lock →
//! descriptive `expect`), which `background_session.rs` in this crate
//! also does.

#![allow(clippy::expect_used)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex as StdMutex;

use once_cell::sync::Lazy;

use codelet_rpc_types::{SessionId, StreamChunk};

/// Per-session cancellation flags for footer pollers.
static FOOTER_POLLER_TOKENS: Lazy<StdMutex<HashMap<String, Arc<AtomicBool>>>> =
    Lazy::new(|| StdMutex::new(HashMap::new()));

/// Process-global chunk-emission target (WT-002).
///
/// The `fspec` binary owns a NON-singleton `SessionManager` whose
/// `chunks_tx` is the only broadcast the embedded TUI subscribes to. Each
/// hook install site registers the correct sender before any session is
/// created. Registration REPLACES the previous value (tests register a
/// fresh manager per test; production registers once and re-registers the
/// same value). BUG-405: when nothing is registered there is no target —
/// emission is refused and reported, never sent to a global.
static CHUNK_SENDER: StdMutex<Option<tokio::sync::broadcast::Sender<(SessionId, StreamChunk)>>> =
    StdMutex::new(None);

/// Register the manager-owned `chunks_tx` broadcast that footer pollers
/// emit on. Replaces any previous registration.
pub fn register_chunk_sender(sender: tokio::sync::broadcast::Sender<(SessionId, StreamChunk)>) {
    *CHUNK_SENDER
        .lock()
        .expect("footer poller chunk sender lock poisoned") = Some(sender);
}

/// Clear the registered chunk sender (tests; production registers once).
pub fn unregister_chunk_sender() {
    *CHUNK_SENDER
        .lock()
        .expect("footer poller chunk sender lock poisoned") = None;
}

/// Resolve the emission target for chunk emission: the registered
/// manager-owned `chunks_tx` broadcast. BUG-405: with nothing registered
/// this is an error — there is no singleton manager to fall back to.
///
/// WT-004: also used by the shared block-notification emitter
/// (`session_tool_callbacks::emit_block_notification`) so blocked actions
/// land on the same broadcast the footer poller uses.
pub fn emission_target() -> Result<tokio::sync::broadcast::Sender<(SessionId, StreamChunk)>, String>
{
    CHUNK_SENDER
        .lock()
        .expect("footer poller chunk sender lock poisoned")
        .clone()
        .ok_or_else(|| {
            "no chunk sender is registered (register_chunk_sender was not called)".to_string()
        })
}

/// Spawn a background task that polls git status for a session every 5 seconds.
///
/// Emits `FooterStateUpdate` chunks through the registered manager-owned
/// `chunks_tx` broadcast. BUG-405: with no sender registered, no runtime or
/// a malformed session id the poller is not started and the reason is
/// logged.
///
/// * `session_id` - The session UUID string
/// * `cwd` - The effective working directory (project root or worktree path)
/// * `worktree_path` - If Some, the session is isolated (uses worktree CWD)
pub fn spawn_footer_poller(session_id: String, cwd: String, worktree_path: Option<String>) {
    let cancelled = Arc::new(AtomicBool::new(false));
    let cancelled_clone = cancelled.clone();

    {
        let mut map = FOOTER_POLLER_TOKENS
            .lock()
            .expect("footer poller tokens lock poisoned");
        // Cancel any existing poller for this session
        if let Some(old_flag) = map.remove(&session_id) {
            old_flag.store(true, Ordering::Relaxed);
        }
        map.insert(session_id.clone(), cancelled);
    }

    let sid = session_id.clone();
    let initial_cwd = worktree_path.unwrap_or(cwd);
    let session_uuid = match uuid::Uuid::parse_str(&session_id) {
        Ok(uuid) => uuid,
        Err(error) => {
            tracing::error!(session = %session_id, %error, "footer poller not started: malformed session id");
            return;
        }
    };

    // TUI-091: Seed the footer CWD registry with the initial value so the
    // footer shows something immediately before any Bash commands run.
    codelet_tools::footer_cwd::update_footer_cwd(session_uuid, initial_cwd.clone());

    let handle = match tokio::runtime::Handle::try_current() {
        Ok(handle) => handle,
        Err(error) => {
            tracing::error!(session = %session_id, %error, "footer poller not started: no tokio runtime");
            return;
        }
    };
    let sender = match emission_target() {
        Ok(sender) => sender,
        Err(error) => {
            tracing::error!(session = %session_id, %error, "footer poller not started");
            return;
        }
    };

    handle.spawn(async move {
        // The home directory, when known, shortens the displayed path to
        // "~/..."; without one the full path is shown.
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .ok()
            .filter(|home| !home.is_empty());

        // TUI-091: Track previous state so we only emit on change.
        // CWD is now dynamic — we re-read it from the footer-cwd registry
        // each tick.
        let mut prev_cwd = initial_cwd.clone();
        let mut prev_display_path = if let Some(home) = home.as_deref().filter(|home| initial_cwd.starts_with(*home)) {
            format!("~{}", &initial_cwd[home.len()..])
        } else {
            initial_cwd.clone()
        };
        let mut prev_branch: Option<String> = None;
        let mut prev_is_git = false;
        let mut first_run = true;

        loop {
            if cancelled_clone.load(Ordering::Relaxed) {
                break;
            }

            // TUI-091: Read the current CWD from the per-session registry.
            // BashTool writes here after every invocation.
            // The registry is seeded at spawn; an empty entry means the
            // session's cwd was unregistered (destroyed or re-pointed), so
            // this poller is finished.
            let Some(current_cwd) = codelet_tools::footer_cwd::get_footer_cwd(session_uuid) else {
                tracing::debug!(session = %sid, "footer poller stopping: the session's cwd was unregistered");
                break;
            };

            // Recompute display_path if CWD changed
            let current_display_path = if current_cwd != prev_cwd {
                if let Some(home) = home.as_deref().filter(|home| current_cwd.starts_with(*home)) {
                    format!("~{}", &current_cwd[home.len()..])
                } else {
                    current_cwd.clone()
                }
            } else {
                prev_display_path.clone()
            };

            // Poll ONLY branch name (reads .git/HEAD — near-zero CPU cost).
            // Do NOT call get_staged_files / get_unstaged_files /
            // get_untracked_files — those walk the entire worktree and burn
            // massive CPU every poll cycle. TUI-091: keep this optimization.
            //
            // WT-002: `Ok(None)` (detached HEAD inside a repo) is STILL a
            // git repo — emit with `is_git_repo=true, branch=None` so the
            // TUI footer can show the detached indicator instead of
            // collapsing to a blank branch. Only `Err` (not a repo) maps
            // to `is_git_repo=false`.
            let cwd_for_git = current_cwd.clone();
            let git_result = tokio::task::spawn_blocking(move || {
                match codelet_git::get_current_branch(&cwd_for_git) {
                    Ok(branch) => (true, branch),
                    Err(_) => (false, None),
                }
            })
            .await;

            if let Ok((is_git, branch)) = git_result {
                // Only emit if something changed (or on first run)
                let cwd_changed = current_cwd != prev_cwd;
                if first_run || cwd_changed || is_git != prev_is_git || branch != prev_branch {
                    first_run = false;
                    prev_cwd = current_cwd.clone();
                    prev_display_path = current_display_path.clone();
                    prev_is_git = is_git;
                    prev_branch = branch.clone();

                    let chunk = StreamChunk::footer_state_update(
                        current_cwd.clone(),
                        current_display_path.clone(),
                        is_git,
                        branch,
                    );
                    // A broadcast send fails only when no client is
                    // subscribed — routine between attaches, so it is
                    // reported at debug level rather than as an error.
                    if let Err(error) = sender.send((SessionId::from(sid.clone()), chunk)) {
                        tracing::debug!(session = %sid, %error, "footer update has no subscriber");
                    }
                }
            }

            // Sleep 5 seconds, checking cancellation every 500ms
            for _ in 0..10 {
                if cancelled_clone.load(Ordering::Relaxed) {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }
    });
}

/// Stop the footer poller for a session.
pub fn stop_footer_poller(session_id: &str) {
    let mut map = FOOTER_POLLER_TOKENS
        .lock()
        .expect("footer poller tokens lock poisoned");
    if let Some(flag) = map.remove(session_id) {
        flag.store(true, Ordering::Relaxed);
    }
}
