//! `App::dispatch` — single mutation surface for [`BoardStore`] +
//! [`AgentViewStore`] per the RPC-009 single-task tenere pattern.
use crate::components::disconnect_dialog::{DisconnectDialog, DISCONNECT_DIALOG_ID};
use crate::components::Action;
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
                if let Some(ctx) = self.agent_view_store.session_context_mut_for(id) {
                    ctx.record_chunk(chunk);
                }
                self.agent_view_store.apply_chunk_to_token_state(id, chunk);
                self.handle_stream_chunk_state_updates(id, chunk); // RPC-045
                self.maybe_push_error_dialog_for_chunk(chunk); // RPC-079
            }
            Action::SessionStatusChanged(id, status) => {
                // RPC-045: push-driven SessionStatus from status_changes_rx.
                self.handle_session_status_changed(id.clone(), *status);
            }
            Action::Disconnected if !self.compositor.contains(DISCONNECT_DIALOG_ID) => {
                // RPC-011 CR-1: DisconnectDialog @ Priority::Critical.
                self.compositor.push(Box::new(DisconnectDialog::new()));
            }
            Action::ManualReconnect => {
                // RPC-011: route `r` through FspecBackend (no-op default).
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
                        // PROV-101 FIX 1: an empty id is a decline — surface it
                        // explicitly and never seed it as the active session.
                        crate::app::session_creation::route_bootstrap_create_session(
                            session,
                            &active_session_tx,
                            &action_tx,
                        );
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
                // RPC-050: bind work unit to current session via the
                // attach action; lazy SessionCreated re-dispatches below.
                let _ = self
                    .action_tx
                    .send(Action::AttachWorkUnitToSession(id.clone()));
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
            Action::OpenAgentView(target) => {
                self.handle_open_agent_view(target.clone());
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
                self.handle_session_created(session.clone());
            }
            Action::SessionCreationDeclined => {
                // PROV-101 FIX 1: create_session declined (no default model).
                // Surface it explicitly instead of swallowing an empty id.
                self.handle_session_creation_declined();
            }
            Action::FocusPrevColumn => self.board_store.focus_prev_column(),
            Action::FocusNextColumn => self.board_store.focus_next_column(),
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
            Action::SelectFirstInFocused => self.board_store.select_first_in_focused(),
            Action::SelectLastInFocused => self.board_store.select_last_in_focused(),
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
                self.scroll_focused(-(self.navigator.agent.scrollback_viewport_hint() as i64))
            }
            Action::ScrollbackPageDown => {
                self.scroll_focused(self.navigator.agent.scrollback_viewport_hint() as i64)
            }
            Action::ScrollbackLineUp => self.scroll_focused(-1),
            Action::ScrollbackLineDown => self.scroll_focused(1),
            Action::ScrollbackHome => {
                if let Some(ctx) = self.agent_view_store.current_session_context_mut() {
                    ctx.scrollback.jump_to_top();
                }
            }
            Action::ScrollbackMouseWheelUp(velocity) => self.scroll_focused(-(*velocity as i64)),
            Action::ScrollbackMouseWheelDown(velocity) => self.scroll_focused(*velocity as i64),
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
            // RPC-026 /resume + /search wiring → helpers in app/dispatch_resume_search_views.rs.
            Action::OpenResumeView => self.handle_open_resume_view(),
            Action::CloseResumeView => self.handle_close_resume_view(),
            Action::OpenSearchView => self.handle_open_search_view(),
            Action::CloseSearchView => self.handle_close_search_view(),
            Action::SessionListLoaded(s) => self.handle_session_list_loaded(s.clone()),
            Action::AttachToSession(s) => self.handle_attach_to_session(s.clone()),
            Action::SearchHistory(q) => self.handle_search_history(q.clone()),
            Action::HistorySearchResults { query, matches } => {
                self.handle_history_search_results(query.clone(), matches.clone())
            }
            Action::InsertIntoInput(t) => self.handle_insert_into_input(t.clone()),
            Action::RequestDeleteSession(id) => self.handle_request_delete_session(id.clone()),
            Action::ConfirmDeleteSession(id) => self.handle_confirm_delete_session(id.clone()),
            Action::EmitSessionNotice(sid, text) => {
                self.handle_emit_session_notice(sid, text.clone())
            }
            Action::SessionResumeComplete(id) => self.handle_session_resume_complete(id.clone()),
            // Capability dispatchers: try_dispatch_* fallbacks (keep <300 LoC).
            _ => {
                let _ = self.try_dispatch_model_selector(&action)
                    || self.try_dispatch_model_thinking_dialogs(&action)
                    || self.try_dispatch_pause_hitl(&action)
                    || self.try_dispatch_provider_settings(&action)
                    || self.try_dispatch_blocklist(&action)
                    || self.try_dispatch_changed_files(&action)
                    || self.try_dispatch_viewer(&action)
                    || self.try_dispatch_checkpoints(&action)
                    || self.try_dispatch_merge_worktree(&action)
                    || self.try_dispatch_slash_schedule(&action)
                    || self.try_dispatch_slash_loop(&action)
                    || self.try_dispatch_create_session_dialog(&action)
                    || self.try_dispatch_supervisor_links(&action)
                    || self.try_dispatch_dialog_dismiss(&action);
            }
        }
        self.navigator.apply_action(&action);
        let _ = self.compositor.update(action);
        self.should_render = true;
    }
}
