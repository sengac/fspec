//! RPC-094 — Extracted mode-view key handlers from `dispatch.rs` so
//! the orchestrator file stays under the 300-LoC source-shape budget.
//!
//! Houses `handle_resume_view_key` + `handle_search_view_key` which
//! together routed ~50 lines of match-arm boilerplate.

use crossterm::event::KeyEvent;

use crate::components::{Action, EventResult};

use super::resume_session_view::ResumeSessionViewOutcome;
use super::search_history_view::SearchHistoryViewOutcome;
use super::AgentView;

impl AgentView {
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
            ResumeSessionViewOutcome::RequestDelete(session_id) => {
                self.emit(Action::RequestDeleteSession(session_id));
                Some(EventResult::consumed())
            }
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
