//! RPC-094 — Extracted mode-view key handlers from `dispatch.rs` so
//! the orchestrator file stays under the 300-LoC source-shape budget.
//!
//! Houses `handle_resume_view_key` + `handle_search_view_key` which
//! together routed ~50 lines of match-arm boilerplate, and (TUI-129)
//! `scrollback_jump_key`.

use crossterm::event::{KeyCode, KeyEvent};

use crate::components::{Action, EventResult};

use super::resume_session_view::ResumeSessionViewOutcome;
use super::search_history_view::SearchHistoryViewOutcome;
use super::AgentView;

impl AgentView {
    /// RPC-094 / TUI-129: Home and End on an empty input jump the
    /// scrollback to the top and the bottom; with text in the input they
    /// are left to the composer, which moves the cursor.
    pub(super) fn scrollback_jump_key(&mut self, key: &KeyEvent) -> Option<EventResult> {
        if !key.modifiers.is_empty() || !self.input.is_empty() {
            return None;
        }
        let action = match key.code {
            KeyCode::Home => Action::ScrollbackHome,
            KeyCode::End => Action::ScrollbackEnd,
            _ => return None,
        };
        self.emit(action);
        Some(EventResult::consumed())
    }

    pub(super) fn handle_resume_view_key(&mut self, key: &KeyEvent) -> Option<EventResult> {
        let visible_rows = self.mode_view_visible_rows();
        let view = self.resume_view.as_mut()?;
        match view.handle_key(key.code, key.modifiers, visible_rows) {
            ResumeSessionViewOutcome::Selected(session_id) => {
                self.resume_view = None;
                self.emit(Action::AttachToSession(session_id));
                Some(EventResult::consumed())
            }
            ResumeSessionViewOutcome::Dismiss => {
                self.resume_view = None;
                self.emit(Action::CloseResumeView);
                Some(EventResult::consumed())
            }
            // The view opened its own delete confirm.
            ResumeSessionViewOutcome::RequestDelete(_) => Some(EventResult::consumed()),
            ResumeSessionViewOutcome::ConfirmedDelete(session_id) => {
                self.emit(Action::ConfirmDeleteSession(session_id));
                Some(EventResult::consumed())
            }
            ResumeSessionViewOutcome::CancelledDelete => Some(EventResult::consumed()),
            ResumeSessionViewOutcome::Continued => Some(EventResult::consumed()),
            ResumeSessionViewOutcome::Ignored => Some(EventResult::consumed()),
        }
    }

    pub(super) fn handle_search_view_key(&mut self, key: &KeyEvent) -> Option<EventResult> {
        let visible_rows = self.mode_view_visible_rows();
        let view = self.search_view.as_mut()?;
        match view.handle_key(key.code, key.modifiers, visible_rows) {
            SearchHistoryViewOutcome::FilterChanged(query) => {
                self.emit(Action::SearchHistory(query));
                Some(EventResult::consumed())
            }
            SearchHistoryViewOutcome::Selected(text) => {
                self.search_view = None;
                self.emit(Action::InsertIntoInput(text));
                Some(EventResult::consumed())
            }
            SearchHistoryViewOutcome::Dismiss => {
                self.search_view = None;
                self.emit(Action::CloseSearchView);
                Some(EventResult::consumed())
            }
            SearchHistoryViewOutcome::Continued => Some(EventResult::consumed()),
            SearchHistoryViewOutcome::Ignored => Some(EventResult::consumed()),
        }
    }
}
