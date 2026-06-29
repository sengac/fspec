//! RPC-028 — AgentView mouse-event routing.
//!
//! Extracted from `dispatch.rs` so the parent file stays under the
//! 300-LoC source-shape budget enforced by `tests/source_shape_rpc019.rs`.
//!
//! Routes a single `Event::Mouse` through (in order): the open
//! mode-view (resume / search), then the open popup (slash / file),
//! then (RPC-094) the scrollback rect itself. Returns `None` when
//! nothing absorbs the event.

use crossterm::event::{MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use crate::components::scroll_viewport::WheelDirection;
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

    /// RPC-383: while the turn content modal is open, route mouse-wheel
    /// ScrollUp/ScrollDown into the modal's scroll offset (mirroring the
    /// scrollback wheel handling) via `Action::TurnModalScroll{Up,Down}`.
    /// Returns `None` (so the event bubbles to the scrollback) when the
    /// modal is closed or the event is not a vertical wheel. The modal is
    /// a full-screen overlay, so no rect hit-test is needed.
    pub(super) fn handle_turn_modal_mouse(&mut self, ev: MouseEvent) -> Option<EventResult> {
        self.turn_modal_seq?;
        match ev.kind {
            MouseEventKind::ScrollUp => {
                self.emit(Action::TurnModalScrollUp);
                Some(EventResult::consumed())
            }
            MouseEventKind::ScrollDown => {
                self.emit(Action::TurnModalScrollDown);
                Some(EventResult::consumed())
            }
            _ => None,
        }
    }

    /// RPC-094: route mouse wheel events that fall inside the
    /// scrollback rect into the focused SessionContext via the new
    /// `Action::ScrollbackMouseWheel{Up,Down}(velocity)` variants.
    /// Hit-tests the cached `last_scrollback_area` (set in
    /// `render_with_store`). Wheel events outside the rect bubble.
    pub(super) fn handle_scrollback_mouse(&mut self, ev: MouseEvent) -> Option<EventResult> {
        let rect = self.last_scrollback_area?;
        let inside = ev.column >= rect.x
            && ev.column < rect.x.saturating_add(rect.width)
            && ev.row >= rect.y
            && ev.row < rect.y.saturating_add(rect.height);
        if !inside {
            return None;
        }
        match ev.kind {
            MouseEventKind::ScrollUp => {
                let step = self.scrollback_wheel.step(WheelDirection::Up);
                let velocity = step.unsigned_abs();
                self.emit(Action::ScrollbackMouseWheelUp(velocity));
                Some(EventResult::consumed())
            }
            MouseEventKind::ScrollDown => {
                let step = self.scrollback_wheel.step(WheelDirection::Down);
                let velocity = step.unsigned_abs();
                self.emit(Action::ScrollbackMouseWheelDown(velocity));
                Some(EventResult::consumed())
            }
            _ => None,
        }
    }
}
