//! RPC-028 — AgentView mouse-event routing.
//!
//! Extracted from `dispatch.rs` so the parent file stays under the
//! 300-LoC source-shape budget enforced by `tests/source_shape_rpc019.rs`.
//!
//! Routes a single `Event::Mouse` through (in order): the open
//! mode-view (resume / search), then the open popup (slash / file).
//! Returns `None` when nothing absorbs the event.

use crossterm::event::MouseEvent;
use ratatui::layout::Rect;

use crate::components::{Action, EventResult};

use super::file_search_popup::FilePopupOutcome;
use super::resume_session_view::ResumeSessionViewOutcome;
use super::search_history_view::SearchHistoryViewOutcome;
use super::slash_command_popup::PopupOutcome;
use super::AgentView;

impl AgentView {
    pub(super) fn handle_mode_view_mouse(&mut self, ev: MouseEvent) -> Option<EventResult> {
        let visible_rows = self.mode_view_visible_rows();
        let area = self.last_render_area?;
        let body_rect = Rect {
            x: area.x,
            y: area.y.saturating_add(2),
            width: area.width,
            height: area.height.saturating_sub(3),
        };
        if let Some(view) = self.resume_view.as_mut() {
            match view.handle_mouse(ev, body_rect, visible_rows) {
                ResumeSessionViewOutcome::Selected(session_id) => {
                    self.resume_view = None;
                    self.emit(Action::AttachToSession(session_id));
                    return Some(EventResult::consumed());
                }
                _ => return Some(EventResult::consumed()),
            }
        }
        if let Some(view) = self.search_view.as_mut() {
            match view.handle_mouse(ev, body_rect, visible_rows) {
                SearchHistoryViewOutcome::Continued | SearchHistoryViewOutcome::Ignored => {
                    return Some(EventResult::consumed())
                }
                _ => return Some(EventResult::consumed()),
            }
        }
        None
    }

    pub(super) fn handle_popup_mouse(&mut self, ev: MouseEvent) -> Option<EventResult> {
        let area = self.last_render_area?;
        if let Some(popup) = self.slash_popup.as_mut() {
            match popup.handle_mouse(ev, area) {
                PopupOutcome::Continued => return Some(EventResult::consumed()),
                PopupOutcome::Ignored => return None,
                _ => return Some(EventResult::consumed()),
            }
        }
        if let Some(popup) = self.file_popup.as_mut() {
            match popup.handle_mouse(ev, area) {
                FilePopupOutcome::Continued => return Some(EventResult::consumed()),
                FilePopupOutcome::Ignored => return None,
                _ => return Some(EventResult::consumed()),
            }
        }
        None
    }
}
