//! App::dispatch routing for RPC-025 history recall + submission
//! persistence.
//!
//! Feature: spec/features/rpc025-app-dispatch.feature
//!
//! Factored out of `app/dispatch.rs` so the orchestrator file stays
//! under the 300-LoC ceiling. Hosts:
//!   - `handle_history_prev`: load snapshot (async) or walk back.
//!   - `handle_history_next`: walk forward / exit recall.
//!   - `handle_history_snapshot_loaded`: apply a freshly loaded
//!     snapshot to the per-session HistoryNavState.
//!   - `handle_input_submitted_persistence`: fire-and-forget
//!     `persistence_add_history` + reset per-session history state.
//!     Called from `handle_input_submitted` (RPC-020) AFTER `send_input`.

use codelet_rpc_types::SessionId;

use crate::components::Action;

use super::state::App;

impl App {
    /// Route `Action::HistoryPrev` for the currently focused session.
    ///
    /// First press (recall_index == None) snapshots the live
    /// MultiLineInput into `cached_draft`, then spawns a tokio task
    /// that fetches the per-session history via
    /// `backend.persistence_get_history(session, 100)`. The spawned
    /// task dispatches `Action::HistorySnapshotLoaded(session, snapshot)`
    /// on success; the follow-up arm caches the snapshot and snaps
    /// `recall_index` to `Some(0)`.
    ///
    /// Subsequent presses (recall_index == Some(k)) walk synchronously
    /// through the cached snapshot and clamp at `len - 1` so a fifth
    /// Shift+↑ on a three-entry history is a no-op.
    pub(crate) fn handle_history_prev(&mut self) {
        let Some(session) = self.agent_view_store.current_session().cloned() else {
            return;
        };
        let current_idx = self
            .agent_view_store
            .history_state_for(&session)
            .and_then(|s| s.recall_index);
        match current_idx {
            None => self.start_history_recall(session),
            Some(k) => self.step_history_prev(&session, k),
        }
    }

    /// Eagerly cache the live MultiLineInput into `cached_draft` and
    /// spawn the snapshot-load task. The eager cache ensures the user's
    /// in-progress draft survives even if the load races with further
    /// input (a fresh load is always paired with the live draft
    /// captured at the moment of the FIRST Shift+↑).
    fn start_history_recall(&mut self, session: SessionId) {
        let live = self.navigator.agent.input.value();
        let state = self.agent_view_store.history_state_for_mut(&session);
        state.cached_draft = Some(live);
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let session_for_task = session.clone();
        let handle = tokio::spawn(async move {
            if let Ok(snapshot) =
                backend.persistence_get_history(session_for_task.clone(), 100).await
            {
                let _ = action_tx
                    .send(Action::HistorySnapshotLoaded(session_for_task, snapshot));
            }
        });
        self.pending_tasks.push(handle);
    }

    /// Walk one step back into the cached snapshot, clamped at the tail.
    fn step_history_prev(&mut self, session: &SessionId, k: usize) {
        let next_idx = match self.agent_view_store.cached_history_snapshot(session) {
            Some(snap) if !snap.is_empty() => (k + 1).min(snap.len() - 1),
            _ => return,
        };
        let value = match self.agent_view_store.cached_history_snapshot(session) {
            Some(snap) => snap[next_idx].clone(),
            None => return,
        };
        self.agent_view_store
            .history_state_for_mut(session)
            .recall_index = Some(next_idx);
        self.navigator.agent.input.set_value(&value);
    }

    /// Route `Action::HistoryNext`. No-op in live-draft mode
    /// (`recall_index == None`). On `Some(0)` exits recall and restores
    /// the cached_draft into the MultiLineInput. Otherwise decrements
    /// `recall_index` and replaces the MultiLineInput with the entry
    /// at the new index.
    pub(crate) fn handle_history_next(&mut self) {
        let Some(session) = self.agent_view_store.current_session().cloned() else {
            return;
        };
        let current_idx = self
            .agent_view_store
            .history_state_for(&session)
            .and_then(|s| s.recall_index);
        match current_idx {
            None => {}
            Some(0) => self.exit_history_recall(&session),
            Some(k) => self.step_history_next(&session, k),
        }
    }

    fn exit_history_recall(&mut self, session: &SessionId) {
        let cached = self
            .agent_view_store
            .history_state_for(session)
            .and_then(|s| s.cached_draft.clone())
            .unwrap_or_default();
        let state = self.agent_view_store.history_state_for_mut(session);
        state.recall_index = None;
        self.navigator.agent.input.set_value(&cached);
    }

    fn step_history_next(&mut self, session: &SessionId, k: usize) {
        let new_idx = k - 1;
        let value = match self.agent_view_store.cached_history_snapshot(session) {
            Some(snap) if new_idx < snap.len() => snap[new_idx].clone(),
            _ => return,
        };
        self.agent_view_store
            .history_state_for_mut(session)
            .recall_index = Some(new_idx);
        self.navigator.agent.input.set_value(&value);
    }

    /// Route `Action::HistorySnapshotLoaded`. Drops empty snapshots so a
    /// Shift+↑ with no history leaves state untouched. For non-empty
    /// snapshots: cache the snapshot, snap `recall_index` to `Some(0)`,
    /// replace MultiLineInput with `snapshot[0]`.
    pub(crate) fn handle_history_snapshot_loaded(
        &mut self,
        session: SessionId,
        snapshot: Vec<String>,
    ) {
        if snapshot.is_empty() {
            return;
        }
        let first = snapshot[0].clone();
        self.agent_view_store
            .set_history_snapshot(session.clone(), snapshot);
        self.agent_view_store
            .history_state_for_mut(&session)
            .recall_index = Some(0);
        // Only paint into MultiLineInput when the focused session
        // matches the loaded snapshot's session — a load that races
        // with a session cycle must not overwrite the other session's
        // live input.
        if self.agent_view_store.current_session() == Some(&session) {
            self.navigator.agent.input.set_value(&first);
        }
    }

    /// RPC-025 extension to `handle_input_submitted`: fire-and-forget
    /// `backend.persistence_add_history(session, text)` and reset the
    /// per-session HistoryNavState so the next Shift+↑ pulls a fresh
    /// snapshot. The spawn pattern guarantees a slow disk write never
    /// blocks input submission.
    pub(crate) fn handle_input_submitted_persistence(&mut self, session: SessionId, text: String) {
        self.agent_view_store.reset_history_state(&session);
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let handle = tokio::spawn(async move {
            let _ = backend.persistence_add_history(session, text).await;
        });
        self.pending_tasks.push(handle);
    }
}
