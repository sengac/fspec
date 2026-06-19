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
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let s1 = session.clone();
        let h1 = tokio::spawn(async move {
            if let Ok(info) = backend.get_model_info(s1.clone()).await {
                let _ = action_tx.send(Action::ModelInfoLoaded(s1, info));
            }
        });
        self.pending_tasks.push(h1);
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
}
