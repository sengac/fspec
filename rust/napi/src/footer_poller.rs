//! Background footer-state poller (RPC-043, originally TUI-091).
//!
//! WT-002: the poller logic moved to the NAPI-free shared implementation
//! in `codelet_sessions::footer_poller` so the fspec binary
//! (`FspecAgentHooks`) and the NAPI path (`NapiSessionManagerHooks`) run
//! the SAME code (two-front-doors rule). This module is now a thin shim
//! that keeps the `crate::footer_poller::spawn_footer_poller` delegation
//! pinned by the RPC-043 shape tests, and registers the singleton
//! manager's `chunks_tx` as the emission target for this process
//! (preserving today's NAPI `SessionManager::instance()` behavior — the
//! shared poller falls back to the singleton when nothing is registered).

use codelet_sessions::session_manager::SessionManager;

/// TUI-091: Spawn a background task that polls git status for a session
/// every 5 seconds. Delegates to the shared NAPI-free poller (WT-002).
///
/// * `session_id` - The session UUID string
/// * `cwd` - The effective working directory (project root or worktree path)
/// * `worktree_path` - If Some, the session is isolated (uses worktree CWD)
pub(crate) fn spawn_footer_poller(session_id: String, cwd: String, worktree_path: Option<String>) {
    // The napi process emits via the singleton manager — the TS-side
    // fan-out subscribes to `SessionManager::instance().chunks_tx()`.
    // (Re-)register it on every spawn so the shared poller's emission
    // target is this process's singleton.
    codelet_sessions::footer_poller::register_chunk_sender(
        SessionManager::instance().chunks_tx().clone(),
    );
    codelet_sessions::footer_poller::spawn_footer_poller(session_id, cwd, worktree_path);
}

/// TUI-091: Stop the footer poller for a session.
pub(crate) fn stop_footer_poller(session_id: &str) {
    codelet_sessions::footer_poller::stop_footer_poller(session_id);
}
