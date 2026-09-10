//! App::dispatch routing for `Action::EnterWorkUnit` (board → agent).
//!
//! Factored out of `app/dispatch.rs` so the orchestrator stays under the
//! 300-LoC ceiling. Handles the work-unit bind + view flip (or
//! MUX-001 R8 in-grid binding) + the lazy first-session `create_session`
//! spawn.

use crate::components::Action;
use crate::views::ViewMode;

use super::state::App;

impl App {
    /// Enter a work unit from the board: bind it to the current session,
    /// flip to the agent view (or bind in the mux grid), and lazily
    /// create the first session if none is open.
    pub(crate) fn handle_enter_work_unit(&mut self, id: &String) {
        let status = self
            .board_store
            .column_units(self.board_store.focused_column())
            .iter()
            .find(|u| u.id == *id)
            .map(|u| u.status.clone());
        self.agent_view_store
            .set_current_work_unit(Some(id.clone()), status);
        // MUX-001 R8: in mux mode, Enter on a board work unit
        // binds the unit + focuses the agent pane WITHOUT
        // flipping the whole view.
        // BUG-175: gate on the LIVE view, not the persisted
        // `mux.config().enabled` flag — with the flag leaked
        // across a restart (saved while in the grid) the mux
        // path would have swallowed the flip and stranded the
        // user on the Board with a bound-but-unopened unit.
        if self.navigator.active_view == ViewMode::Mux {
            let _ = self.action_tx.send(Action::MuxEnterWorkUnit(id.clone()));
        } else {
            self.navigator.active_view = ViewMode::Agent;
        }
        // RPC-050: bind work unit to current session via the
        // attach action; lazy SessionCreated re-dispatches below.
        let _ = self.action_tx.send(Action::AttachWorkUnitToSession(id.clone()));
        if self.agent_view_store.current_session().is_none() {
            let backend = self.backend.clone();
            let action_tx = self.action_tx.clone();
            let active_session_tx = self.active_session_tx.clone();
            let handle = tokio::spawn(async move {
                if let Ok(session) = backend.create_session(None).await {
                    // PROV-101 FIX 1: empty id == decline; surface it
                    // explicitly, never seed an empty active session.
                    crate::app::session_creation::route_bootstrap_create_session(
                        session,
                        &active_session_tx,
                        &action_tx,
                    );
                }
            });
            self.pending_tasks.push(handle);
        }
    }
}
