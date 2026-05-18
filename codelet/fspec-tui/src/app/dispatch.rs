//! `App::dispatch` — single mutation surface for [`BoardStore`] +
//! [`AgentViewStore`] per the RPC-009 single-task tenere pattern.

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
                // RPC-024: route the chunk into the SessionContext whose
                // id matches `id`, not the currently focused one — this
                // allows background sessions to accumulate scrollback
                // while the user is on another session. Unknown ids are
                // dropped silently (race with session destruction).
                if let Some(ctx) = self.agent_view_store.session_context_mut_for(id) {
                    ctx.record_chunk(chunk);
                }
                // RPC-018: per-session token state fold (unchanged).
                self.agent_view_store.apply_chunk_to_token_state(id, chunk);
            }
            Action::Disconnected if !self.compositor.contains(DISCONNECT_DIALOG_ID) => {
                // RPC-011 CR-1 rule [1]: push the DisconnectDialog @
                // Priority::Critical when the WebSocket drops.
                self.compositor.push(Box::new(DisconnectDialog::new()));
            }
            Action::ManualReconnect => {
                // RPC-011 rule [4]: route the `r` press through the
                // FspecBackend trait. Default impl is a no-op for
                // backends without a supervisor.
                self.backend.request_manual_reconnect();
            }
            Action::Reconnected => {
                let _ = self.compositor.remove(DISCONNECT_DIALOG_ID);
                // RPC-011 rule [5]: re-bootstrap on reconnect.
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
                // RPC-024: append the new session to open_sessions and
                // focus it. Replaces the pre-RPC-024 set_current_session.
                self.agent_view_store
                    .append_session(crate::store::SessionContext::new(session.clone()));
                let _ = self.active_session_tx.send(Some(session.clone()));
                if let Some(id) = self
                    .agent_view_store
                    .current_work_unit_id()
                    .map(std::string::ToString::to_string)
                {
                    let _ = self
                        .action_tx
                        .send(Action::AttachSession(id, session.clone()));
                }
                // RPC-018: refresh per-session chrome state.
                self.refresh_session_chrome(session.clone());
            }
            Action::FocusPrevColumn => {
                self.board_store.focus_prev_column();
            }
            Action::FocusNextColumn => {
                self.board_store.focus_next_column();
            }
            Action::SelectNext => {
                // RPC-016: auto-scrolling arrow nav.
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
                // RPC-023: map idx via COLUMN_ORDER.
                use crate::store::COLUMN_ORDER;
                if let Some(column) = COLUMN_ORDER.get(*idx) {
                    self.board_store.set_focused_column(column);
                }
            }
            Action::SelectIndexInFocused(idx) => {
                // RPC-023: viewport-aware row select on click.
                let vh = self.navigator.board.last_viewport_height();
                self.board_store.select_index_in_focused(*idx, vh);
            }
            Action::ReEnableMouseTracking(_owner) => {
                // RPC-023 scaffolding: no toggle registry yet.
            }
            Action::ReorderUp => {
                // RPC-017: persist a one-step UP move; watcher re-seeds.
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
                // RPC-017: mirror of ReorderUp.
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
                // RPC-018: cache ModelInfo for the SessionHeader.
                self.agent_view_store
                    .set_model_info(session_id.clone(), info.clone());
            }
            Action::ThinkingLevelLoaded(session_id, level) => {
                self.agent_view_store
                    .set_thinking_level(session_id.clone(), *level);
            }
            Action::WorkspaceInfoLoaded(info) => {
                // RPC-018: cache workspace for the SessionFooter.
                self.agent_view_store.set_workspace(Some(info.clone()));
            }
            Action::SlashCommandSelected(slash_action) => {
                // RPC-020: route the user's slash command pick — Help /
                // Clear / Quit are wired live; everything else surfaces
                // a `[notice]` scrollback line.
                self.handle_slash_command(*slash_action);
            }
            Action::SearchFiles(prefix) => {
                // RPC-020: kick off a backend file search; the result
                // is dispatched back via Action::FileSearchResults.
                self.handle_search_files(prefix.clone());
            }
            Action::FileSearchResults(matches) => {
                // RPC-020: fold the backend's match list into the
                // currently-open file search popup.
                self.handle_file_search_results(matches.clone());
            }
            Action::SessionPrev => {
                // RPC-024: save outgoing draft, rotate index backward
                // with wrap-around, restore incoming draft.
                self.handle_session_cycle(-1);
            }
            Action::SessionNext => {
                // RPC-024: save outgoing draft, rotate index forward
                // with wrap-around, restore incoming draft.
                self.handle_session_cycle(1);
            }
            Action::ScrollbackPageUp => {
                // RPC-024: route scrollback PageUp into the focused
                // SessionContext's ScrollbackList so scroll state stays
                // per-session.
                let vh = self.navigator.agent.scrollback_viewport_hint();
                if let Some(ctx) = self.agent_view_store.current_session_context_mut() {
                    ctx.scrollback.scroll_up(vh);
                }
            }
            Action::ScrollbackPageDown => {
                // RPC-024: sibling of `ScrollbackPageUp` — advances
                // offset and re-snaps to stick-mode at the tail.
                let vh = self.navigator.agent.scrollback_viewport_hint();
                if let Some(ctx) = self.agent_view_store.current_session_context_mut() {
                    ctx.scrollback.scroll_down(vh);
                }
            }
            Action::HistoryPrev => {
                // RPC-025: load or step into per-session history recall.
                self.handle_history_prev();
            }
            Action::HistoryNext => {
                // RPC-025: walk forward / exit recall and restore draft.
                self.handle_history_next();
            }
            Action::HistorySnapshotLoaded(session, snapshot) => {
                // RPC-025: apply a freshly loaded history snapshot.
                self.handle_history_snapshot_loaded(session.clone(), snapshot.clone());
            }
            // RPC-026: /resume + /search mode-view wiring. Each arm
            // routes through a small helper in app/dispatch_rpc026.rs.
            Action::OpenResumeView => self.handle_open_resume_view(),
            Action::CloseResumeView => self.handle_close_resume_view(),
            Action::OpenSearchView => self.handle_open_search_view(),
            Action::CloseSearchView => self.handle_close_search_view(),
            Action::SessionListLoaded(s) => self.handle_session_list_loaded(s.clone()),
            Action::AttachToSession(s) => self.handle_attach_to_session(s.clone()),
            Action::SearchHistory(q) => self.handle_search_history(q.clone()),
            Action::HistorySearchResults(m) => self.handle_history_search_results(m.clone()),
            Action::InsertIntoInput(t) => self.handle_insert_into_input(t.clone()),
            Action::RequestDeleteSession(id) => self.handle_request_delete_session(id.clone()),
            Action::ConfirmDeleteSession(id) => self.handle_confirm_delete_session(id.clone()),
            _ => {}
        }
        self.navigator.apply_action(&action);
        let _ = self.compositor.update(action);
        self.should_render = true;
    }
}
