//! `App::dispatch` — single mutation surface for [`BoardStore`] +
//! [`AgentViewStore`] per the RPC-009 single-task tenere pattern.
//!
//! Every Action that mutates state lands here. Subscriber tasks (work
//! units / chunks / logs) emit Actions only — they never touch the
//! stores directly. The Navigator's `apply_action` flips `active_view`
//! after this method has updated the underlying stores.

use crate::components::Action;
use crate::components::disconnect_dialog::{DisconnectDialog, DISCONNECT_DIALOG_ID};
use crate::views::ViewMode;

use super::state::App;

impl App {
    /// Dispatch an [`Action`] into the App. Updates the BoardStore /
    /// AgentViewStore / Navigator + Compositor in lockstep.
    pub fn dispatch(&mut self, action: Action) {
        match &action {
            Action::Quit => {
                self.should_quit = true;
            }
            Action::InputSubmitted(text) => {
                self.handle_input_submitted(text.clone());
            }
            Action::Interrupt => {
                if let Some(session) = self.agent_view_store.current_session().cloned() {
                    let backend = self.backend.clone();
                    tokio::spawn(async move {
                        let _ = backend.interrupt(session).await;
                    });
                }
            }
            Action::ChunkReceived(_id, chunk) => {
                // The chunks subscriber has already filtered by the
                // AgentViewStore's current_session; record the chunk
                // into the AgentView's scrollback.
                self.navigator.agent.record_chunk(chunk);
            }
            Action::Disconnected => {
                // RPC-011 CR-1 rule [1]: push the DisconnectDialog @
                // Priority::Critical when the WebSocket drops.
                if !self.compositor.contains(DISCONNECT_DIALOG_ID) {
                    self.compositor.push(Box::new(DisconnectDialog::new()));
                }
            }
            Action::ManualReconnect => {
                // RPC-011 rule [4]: route the `r` press through the
                // FspecBackend trait. Default impl is a no-op for
                // backends without a supervisor.
                self.backend.request_manual_reconnect();
            }
            Action::Reconnected => {
                let _ = self.compositor.remove(DISCONNECT_DIALOG_ID);
                // RPC-011 rule [5] / [23]: re-issue list_work_units +
                // create_session(None) so the App's left pane re-seeds
                // and the AgentView gets a fresh active session id.
                let backend = self.backend.clone();
                let action_tx = self.action_tx.clone();
                let active_session_tx = self.active_session_tx.clone();
                tokio::spawn(async move {
                    if let Ok(units) = backend.list_work_units().await {
                        let _ = action_tx.send(Action::WorkUnitsLoaded(units));
                    }
                    if let Ok(session) = backend.create_session(None).await {
                        let _ = active_session_tx.send(Some(session.clone()));
                        let _ = action_tx.send(Action::SessionCreated(session));
                    }
                });
            }
            Action::WorkUnitsLoaded(units) => {
                self.board_store.replace_work_units(units.clone());
            }
            Action::EnterWorkUnit(id) => {
                let status = self
                    .board_store
                    .column_units(self.board_store.focused_column())
                    .iter()
                    .find(|u| u.id == *id)
                    .map(|u| u.status.clone());
                self.agent_view_store
                    .set_current_work_unit(Some(id.clone()), status);
                self.navigator.active_view = ViewMode::Agent;
                // Lazy session creation: only if no current_session yet.
                if self.agent_view_store.current_session().is_none() {
                    let backend = self.backend.clone();
                    let action_tx = self.action_tx.clone();
                    let active_session_tx = self.active_session_tx.clone();
                    let handle = tokio::spawn(async move {
                        if let Ok(session) = backend.create_session(None).await {
                            let _ = active_session_tx.send(Some(session.clone()));
                            let _ = action_tx.send(Action::SessionCreated(session));
                        }
                    });
                    self.pending_tasks.push(handle);
                }
            }
            Action::OpenAgentView(target) => {
                match target {
                    Some(sid) => {
                        self.agent_view_store
                            .set_navigation_target(Some(sid.clone()));
                    }
                    None => {
                        self.agent_view_store.request_create_session_dialog();
                    }
                }
                self.navigator.active_view = ViewMode::Agent;
            }
            Action::BackToBoard => {
                self.navigator.active_view = ViewMode::Board;
            }
            Action::NavigationTargetSet(target) => {
                self.agent_view_store.set_navigation_target(target.clone());
            }
            Action::AttachSession(work_unit_id, session) => {
                self.board_store
                    .attach_session(work_unit_id, session.clone());
            }
            Action::SessionCreated(session) => {
                self.agent_view_store
                    .set_current_session(Some(session.clone()));
                let _ = self.active_session_tx.send(Some(session.clone()));
                if let Some(id) = self
                    .agent_view_store
                    .current_work_unit_id()
                    .map(|s| s.to_string())
                {
                    let _ = self
                        .action_tx
                        .send(Action::AttachSession(id, session.clone()));
                }
            }
            Action::FocusPrevColumn => {
                self.board_store.focus_prev_column();
            }
            Action::FocusNextColumn => {
                self.board_store.focus_next_column();
            }
            Action::SelectNext => {
                let col = self.board_store.focused_column().to_string();
                let cur = self.board_store.selected_index_for(&col);
                self.board_store
                    .set_selected_index_for(&col, cur.saturating_add(1));
            }
            Action::SelectPrev => {
                let col = self.board_store.focused_column().to_string();
                let cur = self.board_store.selected_index_for(&col);
                self.board_store
                    .set_selected_index_for(&col, cur.saturating_sub(1));
            }
            Action::ReorderUp | Action::ReorderDown => {
                // RPC-012 architecture note [1]: persistence is out of scope
                // for this slice — placeholder no-op.
            }
            _ => {}
        }
        // Navigator may need to flip active_view; Compositor may need
        // to react to lifecycle actions. Both always see every Action.
        self.navigator.apply_action(&action);
        let _ = self.compositor.update(action);
        self.should_render = true;
    }

    /// Spawn `backend.send_input` for the AgentViewStore's current
    /// session. Handles the no-session-manager stub case by surfacing a
    /// notice line in the scrollback rather than dispatching the call.
    fn handle_input_submitted(&mut self, text: String) {
        self.navigator
            .agent
            .push_line(format!("user> {text}"));
        let Some(session) = self.agent_view_store.current_session().cloned() else {
            return;
        };
        if session.value == "rpc-no-session-manager" {
            self.navigator.agent.push_line(
                "[notice] no LLM session manager attached — input recorded but \
                 not sent to a model.",
            );
            return;
        }
        let backend = self.backend.clone();
        tokio::spawn(async move {
            let _ = backend.send_input(session, text).await;
        });
    }
}
