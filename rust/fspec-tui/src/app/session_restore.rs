//! BUG-206 — auto-restore of the last active session on client startup and
//! reconnect. MEMORY-ONLY.
//!
//! Feature: spec/features/auto-restore-last-active-session-on-client-startup-and-reconnect.feature
//!
//! Resolution (R1): the candidates are (a) on the reconnect path, the
//! client's focused session (tabs survive transient WS drops; another
//! client may have moved the daemon pointer meanwhile), and (b) the
//! daemon's in-memory active-session pointer (RPC-037
//! `get_active_session`). The first candidate that is verified LIVE via
//! `list_live_sessions()` (BUG-206 R3 — the daemon's in-memory session
//! map ONLY, never persisted manifests) wins and is attached through the
//! EXISTING `Action::AttachToSession` path (RPC-049 durable restore:
//! `resume_session` + `get_buffered_output` replay).
//!
//! NO FALLBACKS (BUG-206 R1): a candidate that is not live (closed, or
//! lost to a daemon restart) is NOT restorable — nothing happens, the
//! client lands on the board. No `list_sessions` disk scan, no
//! resume-from-disk, no `create_session` fallback: the RPC-012 lazy
//! create-on-enter-work-unit path is the only session-creation path on
//! startup/reconnect.
//!
//! (Supersedes the TUI-113 fallback chain: the `list_sessions(daemon_cwd)`
//! most-recent entry and the empty-daemon `create_session(None)` were
//! disk-aware fallbacks that resurrected sessions the user had closed.)

use std::sync::Arc;

use codelet_rpc_types::SessionId;

use crate::components::Action;
use crate::transport::FspecBackend;

use super::state::App;

/// BUG-206 R1/R3: resolve the restore target from the backend.
///
/// `focused_override` is `Some` on the reconnect path (the client's
/// focused session is the first candidate); bootstrap passes `None`.
/// A candidate is restorable ONLY if it is in the daemon's in-memory
/// session map (`list_live_sessions` — never persisted manifests).
/// Returns `None` when nothing is restorable: the client lands on the
/// board and the RPC-012 lazy create-on-enter-work-unit path remains
/// the only way a session gets created.
pub(crate) async fn resolve_restore_target(
    backend: &Arc<dyn FspecBackend>,
    focused_override: Option<SessionId>,
) -> Option<SessionId> {
    let mut candidates: Vec<SessionId> = Vec::new();
    if let Some(sid) = focused_override {
        candidates.push(sid);
    }
    // (b) the daemon's in-memory active-session pointer (RPC-037).
    match backend.get_active_session().await {
        Ok(Some(sid)) => candidates.push(sid),
        Ok(None) => {}
        Err(err) => {
            tracing::debug!("restore: get_active_session failed: {err}");
        }
    }
    if candidates.is_empty() {
        return None;
    }
    // Liveness gate (BUG-206 R3): the daemon's in-memory session map
    // ONLY. A candidate that is not live (closed / manifest-only /
    // lost to a daemon restart) is not a candidate at all — no disk
    // fallback, no create fallback (BUG-206 R1).
    let live = match backend.list_live_sessions().await {
        Ok(live) => live,
        Err(err) => {
            tracing::debug!("restore: list_live_sessions failed, nothing restorable: {err}");
            return None;
        }
    };
    candidates
        .into_iter()
        .find(|sid| live.iter().any(|live_id| live_id == sid))
}

impl App {
    /// BUG-206 R1: spawn the restore round-trip. Fire-and-forget —
    /// bootstrap never awaits it, so the Board first frame renders
    /// immediately (mirrors the lazy create_session spawn pattern).
    ///
    /// When resolution yields a live target the existing
    /// `Action::AttachToSession` path is dispatched (RPC-049 durable
    /// restore). When nothing is restorable NOTHING is dispatched:
    /// the client lands on the board — no create fallback, no disk
    /// resume (the RPC-012 lazy create-on-enter-work-unit path is the
    /// only session-creation path on a fresh start / reconnect).
    pub fn spawn_session_restore(&mut self, focused_override: Option<SessionId>) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle = tokio::spawn(async move {
            match resolve_restore_target(&backend, focused_override).await {
                Some(target) => {
                    tracing::info!(
                        session_id = %target.value,
                        "BUG-206: restore target resolved (live in the daemon), dispatching ReattachSession"
                    );
                    let _ = action_tx.send(Action::ReattachSession(target));
                }
                None => {
                    tracing::debug!(
                        "BUG-206: nothing restorable in the daemon's memory; landing on the board"
                    );
                }
            }
        });
        self.pending_tasks.push(handle);
    }

    /// TUI-113 R3: publish the active session to the daemon
    /// (fire-and-forget — errors are logged at debug and never block the
    /// UI). Guarded by the runtime probe so synchronous unit tests (which
    /// dispatch without a tokio runtime) no-op instead of panicking.
    pub(crate) fn spawn_set_active_session(&mut self, session: SessionId) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let handle = tokio::spawn(async move {
            if let Err(err) = backend.set_active_session(session.clone()).await {
                tracing::debug!(
                    session_id = %session.value,
                    "TUI-113 R3: set_active_session failed: {err}"
                );
            }
        });
        self.pending_tasks.push(handle);
    }
}
