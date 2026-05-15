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
            Action::ChunkReceived(id, chunk) => {
                // The chunks subscriber has already filtered by the
                // AgentViewStore's current_session; record the chunk
                // into the AgentView's scrollback.
                self.navigator.agent.record_chunk(chunk);
                // RPC-018: fold the chunk into per-session token state
                // so the SessionHeader paints `tokens: in↓ out↑ [N%]`
                // in real time.
                self.agent_view_store.apply_chunk_to_token_state(id, chunk);
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
            Action::CheckpointCountsLoaded(counts) => {
                // RPC-015: bootstrap delivered fresh checkpoint counts;
                // store them so the BoardView header repaints with the
                // live `Checkpoints: N Manual, M Auto` text.
                self.board_store.set_checkpoint_counts(*counts);
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
                // RPC-018: refresh per-session chrome state. Two
                // spawned tasks fire `backend.get_model_info(session)`
                // + `backend.get_thinking_level(session)` and dispatch
                // their respective *Loaded actions on success. Guard
                // with `Handle::try_current()` so synchronous unit tests
                // (which call `dispatch` without a Tokio runtime) get a
                // graceful no-op instead of a panic from `tokio::spawn`.
                if tokio::runtime::Handle::try_current().is_ok() {
                    let backend = self.backend.clone();
                    let action_tx = self.action_tx.clone();
                    let session_for_model = session.clone();
                    let h1 = tokio::spawn(async move {
                        if let Ok(info) = backend.get_model_info(session_for_model.clone()).await {
                            let _ = action_tx.send(Action::ModelInfoLoaded(session_for_model, info));
                        }
                    });
                    self.pending_tasks.push(h1);
                    let backend = self.backend.clone();
                    let action_tx = self.action_tx.clone();
                    let session_for_thinking = session.clone();
                    let h2 = tokio::spawn(async move {
                        if let Ok(level) = backend.get_thinking_level(session_for_thinking.clone()).await {
                            let _ = action_tx.send(Action::ThinkingLevelLoaded(session_for_thinking, level));
                        }
                    });
                    self.pending_tasks.push(h2);
                }
            }
            Action::FocusPrevColumn => {
                self.board_store.focus_prev_column();
            }
            Action::FocusNextColumn => {
                self.board_store.focus_next_column();
            }
            Action::SelectNext => {
                // RPC-016: route arrow keys through move_selection so
                // they auto-scroll when crossing the viewport boundary.
                let vh = self.navigator.board.last_viewport_height();
                self.board_store.move_selection(1, vh);
            }
            Action::SelectPrev => {
                let vh = self.navigator.board.last_viewport_height();
                self.board_store.move_selection(-1, vh);
            }
            Action::ScrollFocusedColumnUp(vh) => {
                self.board_store.scroll_focused_column(-1, *vh);
            }
            Action::ScrollFocusedColumnDown(vh) => {
                self.board_store.scroll_focused_column(1, *vh);
            }
            Action::SelectFirstInFocused => {
                self.board_store.select_first_in_focused();
            }
            Action::SelectLastInFocused => {
                self.board_store.select_last_in_focused();
            }
            Action::SetFocusedColumn(idx) => {
                // RPC-023: map the column index to a name via
                // COLUMN_ORDER and forward to BoardStore::set_focused_column.
                use crate::store::COLUMN_ORDER;
                if let Some(column) = COLUMN_ORDER.get(*idx) {
                    self.board_store.set_focused_column(column);
                }
            }
            Action::SelectIndexInFocused(idx) => {
                // RPC-023: route through the viewport-aware setter so
                // the click both targets the row AND scrolls it into
                // view when it falls outside the current viewport.
                let vh = self.navigator.board.last_viewport_height();
                self.board_store.select_index_in_focused(*idx, vh);
            }
            Action::ReEnableMouseTracking(_owner) => {
                // RPC-023 scaffolding: the BoardView slice does not opt
                // into TUI-078 button-press, so no toggle is registered
                // for App::dispatch to look up. RPC-019 will introduce
                // an owner-keyed registry of MouseTrackingToggle
                // instances and route this variant through it.
            }
            Action::ReorderUp => {
                // RPC-017: persist a one-step UP move for the selected
                // work unit. The workspace WorkUnitsWatcher fires
                // Action::WorkUnitsLoaded after the write, which the
                // existing arm above re-seeds the BoardStore from.
                if let Some(unit) = self.board_store.selected_work_unit() {
                    let id = unit.id.clone();
                    let backend = self.backend.clone();
                    tokio::spawn(async move {
                        if let Err(err) = backend.move_work_unit_up(id).await {
                            tracing::debug!("move_work_unit_up failed: {err}");
                        }
                    });
                }
            }
            Action::ReorderDown => {
                // RPC-017: mirror of Action::ReorderUp for the DOWN direction.
                if let Some(unit) = self.board_store.selected_work_unit() {
                    let id = unit.id.clone();
                    let backend = self.backend.clone();
                    tokio::spawn(async move {
                        if let Err(err) = backend.move_work_unit_down(id).await {
                            tracing::debug!("move_work_unit_down failed: {err}");
                        }
                    });
                }
            }
            Action::ModelInfoLoaded(session_id, info) => {
                // RPC-018: AgentViewStore caches the latest ModelInfo so
                // the SessionHeader paints model badges.
                self.agent_view_store
                    .set_model_info(session_id.clone(), info.clone());
            }
            Action::ThinkingLevelLoaded(session_id, level) => {
                // RPC-018: AgentViewStore caches the latest ThinkingLevel.
                self.agent_view_store
                    .set_thinking_level(session_id.clone(), *level);
            }
            Action::WorkspaceInfoLoaded(info) => {
                // RPC-018: AgentViewStore caches the workspace snapshot
                // so the SessionFooter paints cwd + git branch.
                self.agent_view_store.set_workspace(Some(info.clone()));
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
