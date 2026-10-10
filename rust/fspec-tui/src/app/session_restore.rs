//! TUI-113 — auto-restore of the last active session on client startup and
//! reconnect.
//!
//! Feature: spec/features/auto-restore-last-active-session-on-client-startup-and-reconnect.feature
//!
//! Resolution order (R2):
//! 1. the client's focused session (reconnect path only — tabs survive
//!    transient WS drops; another client may have moved the daemon pointer
//!    meanwhile),
//! 2. the daemon's in-memory active-session pointer (RPC-037
//!    `get_active_session`),
//! 3. the most-recent entry of `list_sessions(daemon_cwd)` where
//!    `daemon_cwd` comes from `get_workspace_info().cwd` (RPC-018) with
//!    fallback to the empty string (no project filter). NEVER the
//!    client-side cwd — on a remote client machine that path matches no
//!    daemon session (the RPC-427 filter would empty the list).
//!
//! The winner is attached through the EXISTING `Action::AttachToSession`
//! path (RPC-049 durable restore: `resume_session` +
//! `get_buffered_output` replay). `create_session(None)` is the
//! reconnect-only fallback (R4/R5 — supersedes the RPC-011
//! create-on-reconnect contract) and is NOT run on bootstrap, where the
//! RPC-012 lazy create-on-enter-work-unit path remains the safety net.

use std::sync::Arc;

use codelet_rpc_types::SessionId;

use crate::components::Action;
use crate::transport::FspecBackend;

use super::state::App;

/// TUI-113 R2: resolve the restore target from the backend.
///
/// `focused_override` is `Some` on the reconnect path (the client's
/// focused session wins); bootstrap passes `None`. Pure function of the
/// backend — no App state — so it can run inside the spawned task.
pub(crate) async fn resolve_restore_target(
    backend: &Arc<dyn FspecBackend>,
    focused_override: Option<SessionId>,
) -> Option<SessionId> {
    if let Some(sid) = focused_override {
        return Some(sid);
    }
    // (b) the daemon's in-memory active-session pointer (RPC-037).
    match backend.get_active_session().await {
        Ok(Some(sid)) => return Some(sid),
        Ok(None) => {}
        Err(err) => {
            tracing::debug!("restore: get_active_session failed: {err}");
        }
    }
    // (c) most-recent session of the daemon's WORKSPACE (TUI-099 sorts
    // list_sessions by updated_at DESC, most-recent first). The project
    // filter uses the daemon cwd — never the client cwd (R2).
    let daemon_cwd = match backend.get_workspace_info().await {
        Ok(info) => info.cwd,
        Err(err) => {
            tracing::debug!("restore: get_workspace_info failed, retrying unfiltered: {err}");
            String::new()
        }
    };
    match backend.list_sessions(daemon_cwd).await {
        Ok(sessions) => sessions
            .into_iter()
            .next()
            .map(|info| SessionId::new(info.id)),
        Err(err) => {
            tracing::debug!("restore: list_sessions failed: {err}");
            None
        }
    }
}

impl App {
    /// TUI-113 R1/R4: spawn the restore round-trip. Fire-and-forget —
    /// bootstrap never awaits it, so the Board first frame renders
    /// immediately (mirrors the lazy create_session spawn pattern).
    ///
    /// `create_fallback` (reconnect only, R5): when resolution yields
    /// nothing, run the RPC-011 `create_session(None)` safety net.
    /// Bootstrap passes `false` — the RPC-012 lazy create-on-enter-work-unit
    /// path is the safety net there.
    pub fn spawn_session_restore(
        &mut self,
        focused_override: Option<SessionId>,
        create_fallback: bool,
    ) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let active_session_tx = self.active_session_tx.clone();
        // Reconnect path (focused_override set) re-attaches WITHOUT
        // re-opening a closed tab; bootstrap/fresh-start restores
        // re-open the last-viewed session (RPC-049 durable-restore).
        let reattach = focused_override.is_some();
        let handle = tokio::spawn(async move {
            match resolve_restore_target(&backend, focused_override).await {
                Some(target) => {
                    tracing::info!(
                        session_id = %target.value,
                        "TUI-113: restore target resolved, dispatching restore action"
                    );
                    let action = if reattach {
                        Action::ReattachSession(target)
                    } else {
                        Action::RestoreSession(target)
                    };
                    let _ = action_tx.send(action);
                }
                None => {
                    if create_fallback {
                        // R4/R5: RPC-011 safety net — a genuinely empty
                        // daemon still gets a session on reconnect.
                        // The routed SessionCreated action goes through
                        // the idempotent fold (dedup guard), so an
                        // already-open focused session is never
                        // re-created or focus-stolen.
                        if let Ok(session) = backend.create_session(None).await {
                            super::session_creation::route_bootstrap_create_session(
                                session,
                                &active_session_tx,
                                &action_tx,
                            );
                        }
                    } else {
                        tracing::debug!("TUI-113: nothing restorable; staying on the board");
                    }
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
