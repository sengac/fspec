//! Background footer-state poller (RPC-043, originally TUI-091).
//!
//! Extracted from `codelet/napi/src/session_manager.rs` lines 4034-4180
//! by RPC-043. Spawns a per-session task that polls the on-disk git
//! state for the session's effective working directory and emits
//! [`StreamChunk::footer_state_update`] chunks through the manager-owned
//! `chunks_tx` broadcast (which the napi fan-out task forwards to JS).

use crate::types::StreamChunk;
use codelet_sessions::session_manager::SessionManager;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;

/// Per-session cancellation flags for footer pollers.
static FOOTER_POLLER_TOKENS: once_cell::sync::Lazy<
    StdMutex<std::collections::HashMap<String, Arc<AtomicBool>>>,
> = once_cell::sync::Lazy::new(|| StdMutex::new(std::collections::HashMap::new()));

/// TUI-091: Spawn a background task that polls git status for a session every 5 seconds.
///
/// Emits `FooterStateUpdate` chunks via the global callback so TypeScript
/// can update a Zustand store without any JS-side polling/NAPI calls.
///
/// * `session_id` - The session UUID string
/// * `cwd` - The effective working directory (project root or worktree path)
/// * `worktree_path` - If Some, the session is isolated (uses worktree CWD)
pub(crate) fn spawn_footer_poller(
    session_id: String,
    cwd: String,
    worktree_path: Option<String>,
) {
    let cancelled = Arc::new(AtomicBool::new(false));
    let cancelled_clone = cancelled.clone();

    {
        let mut map = FOOTER_POLLER_TOKENS
            .lock()
            .expect("footer poller tokens lock poisoned");
        // Cancel any existing poller for this session
        if let Some(old_flag) = map.remove(&session_id) {
            old_flag.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        map.insert(session_id.clone(), cancelled);
    }

    let sid = session_id.clone();
    let initial_cwd = worktree_path.unwrap_or(cwd);

    // TUI-091: Seed the footer CWD registry with the initial value so the
    // footer shows something immediately before any Bash commands run.
    if let Ok(uuid) = uuid::Uuid::parse_str(&session_id) {
        codelet_tools::footer_cwd::update_footer_cwd(uuid, initial_cwd.clone());
    }

    // Try to spawn on an existing runtime, falling back gracefully
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        return;
    };

    // Parse the session UUID once for registry lookups inside the loop.
    let session_uuid = uuid::Uuid::parse_str(&session_id).ok();

    handle.spawn(async move {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_default();

        // TUI-091: Track previous state so we only emit on change.
        // CWD is now dynamic — we re-read it from the SessionRegistry each tick.
        let mut prev_cwd = initial_cwd.clone();
        let mut prev_display_path = if !home.is_empty() && initial_cwd.starts_with(&home) {
            format!("~{}", &initial_cwd[home.len()..])
        } else {
            initial_cwd.clone()
        };
        let mut prev_branch: Option<String> = None;
        let mut prev_is_git = false;
        let mut first_run = true;

        loop {
            if cancelled_clone.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }

            // TUI-091: Read the current CWD from the per-session registry.
            // BashTool writes here after every invocation.
            let current_cwd = session_uuid
                .and_then(codelet_tools::footer_cwd::get_footer_cwd)
                .unwrap_or_else(|| initial_cwd.clone());

            // Recompute display_path if CWD changed
            let current_display_path = if current_cwd != prev_cwd {
                if !home.is_empty() && current_cwd.starts_with(&home) {
                    format!("~{}", &current_cwd[home.len()..])
                } else {
                    current_cwd.clone()
                }
            } else {
                prev_display_path.clone()
            };

            // Poll ONLY branch name (reads .git/HEAD — near-zero CPU cost).
            // Do NOT call get_staged_files / get_unstaged_files / get_untracked_files
            // — those walk the entire worktree and burn massive CPU every poll cycle.
            // TUI-091: Use the DYNAMIC CWD, not a frozen value.
            let cwd_for_git = current_cwd.clone();
            let git_result = tokio::task::spawn_blocking(move || {
                let branch = codelet_git::get_current_branch(&cwd_for_git).ok().flatten();
                let is_git = branch.is_some();
                (is_git, branch)
            })
            .await;

            if let Ok((is_git, branch)) = git_result {
                // Only emit if something changed (or on first run)
                let cwd_changed = current_cwd != prev_cwd;
                if first_run
                    || cwd_changed
                    || is_git != prev_is_git
                    || branch != prev_branch
                {
                    first_run = false;
                    prev_cwd = current_cwd.clone();
                    prev_display_path = current_display_path.clone();
                    prev_is_git = is_git;
                    prev_branch = branch.clone();

                    // RPC-041: route through SessionManager::instance().chunks_tx()
                    // — the napi-side fan-out task subscribes once at
                    // startup and delivers each chunk to the TSFN.
                    let chunk = StreamChunk::footer_state_update(
                        current_cwd.clone(),
                        current_display_path.clone(),
                        is_git,
                        branch,
                    );
                    let _ = SessionManager::instance().chunks_tx().send((
                        codelet_rpc_types::SessionId::from(sid.clone()),
                        chunk,
                    ));
                }
            }

            // Sleep 5 seconds, checking cancellation every 500ms
            for _ in 0..10 {
                if cancelled_clone.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }
    });
}

/// TUI-091: Stop the footer poller for a session.
pub(crate) fn stop_footer_poller(session_id: &str) {
    let mut map = FOOTER_POLLER_TOKENS
        .lock()
        .expect("footer poller tokens lock poisoned");
    if let Some(flag) = map.remove(session_id) {
        flag.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}
