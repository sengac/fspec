//! App::dispatch routing for RPC-018 per-session chrome state.
//!
//! Factored out of `app/dispatch.rs` so the orchestrator file stays
//! under the 300-LoC ceiling. Hosts the SessionCreated arm's chrome
//! refresh (spawning `get_model_info` / `get_thinking_level`) plus the
//! three `*Loaded` arms that fold the spawned tasks' results into the
//! AgentViewStore.

use codelet_rpc_types::SessionId;

use crate::components::Action;

use super::state::App;

impl App {
    /// Spawn `get_model_info` + `get_thinking_level` for `session`.
    /// Guarded by `Handle::try_current` so synchronous unit tests
    /// (which call `dispatch` without a Tokio runtime) get a graceful
    /// no-op instead of a panic from `tokio::spawn`.
    ///
    /// RPC-022: additionally spawns a `get_session_role` task so the
    /// RoleBanner paints from the first frame when the session
    /// already has a role overlay set.
    pub(crate) fn refresh_session_chrome(&mut self, session: SessionId) {
        self.spawn_hydrate_work_unit_focus(session.clone());
        self.spawn_hydrate_continue_state(session.clone());
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        self.spawn_model_sync(session.clone(), None);
        // MODEL-012: the daemon applies each model's saved level when a session
        // is created, resumed or switches model; the TUI only reads the level
        // so the badge reflects it, and never sets one itself.
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let s2 = session.clone();
        let h2 = tokio::spawn(async move {
            if let Ok(level) = backend.get_thinking_level(s2.clone()).await {
                let _ = action_tx.send(Action::ThinkingLevelLoaded(s2, level));
            }
        });
        self.pending_tasks.push(h2);
        // RPC-022: session role fetch — co-emitted from the same chrome
        // refresh path so bootstrap + SessionCreated both populate
        // AgentViewStore.role_by_session.
        self.spawn_get_session_role(session);
    }

    /// RPC-430: spawn `backend.get_debug_enabled(session_id)` to hydrate
    /// the debug state when attaching to a session. On success dispatches
    /// `Action::DebugEnabledLoaded(session_id, enabled)` so the `[DEBUG]`
    /// badge reflects the ground-truth. Mirrors TypeScript's
    /// `applyPendingDebugState()` fallback.
    pub(crate) fn spawn_hydrate_debug_state(&mut self, session: SessionId) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let sid = session;
        let handle = tokio::spawn(async move {
            if let Ok(enabled) = backend.get_debug_enabled(sid.clone()).await {
                let _ = action_tx.send(Action::DebugEnabledLoaded(sid, enabled));
            }
        });
        self.pending_tasks.push(handle);
    }
}
