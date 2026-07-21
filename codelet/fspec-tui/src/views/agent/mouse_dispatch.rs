//! RPC-028 — AgentView mouse-event routing.
//!
//! Extracted from `dispatch.rs` so the parent file stays under the
//! 300-LoC source-shape budget enforced by `tests/source_shape_rpc019.rs`.
//!
//! Routes a single `Event::Mouse` through (in order): the open
//! mode-view (resume / search), then the open popup (slash / file),
//! then (RPC-094) the scrollback rect itself. Returns `None` when
//! nothing absorbs the event.

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use crate::components::scroll_viewport::WheelDirection;
use crate::components::{Action, EventResult};
use crate::mouse::gesture::SelectionGesture;
use crate::mouse::scrollbar_drag::ScrollbarGeometry;

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

    /// RPC-383 / COPY-008: while the turn content modal is open, route
    /// left press/drag/release into the modal's own selection recognizer
    /// (rule [1]) BEFORE the wheel-scroll branch, and route mouse-wheel
    /// ScrollUp/ScrollDown into the modal's scroll offset (clearing any
    /// active selection first, rule [6]). Returns `None` (so the event
    /// bubbles to the scrollback) when the modal is closed or the event
    /// is neither a left button nor a vertical wheel. The modal is a
    /// full-screen overlay, so no rect hit-test is needed.
    ///
    /// TUI-103: before text selection, hit-test the scrollbar gutter
    /// (rightmost column of the modal body) and route left-button events
    /// through `ScrollbarDrag`. On computed offset, emit
    /// `Action::TurnModalJumpToOffset`.
    pub(super) fn handle_turn_modal_mouse(&mut self, ev: MouseEvent) -> Option<EventResult> {
        self.turn_modal_seq?;
        match ev.kind {
            MouseEventKind::Down(MouseButton::Left)
            | MouseEventKind::Drag(MouseButton::Left)
            | MouseEventKind::Up(MouseButton::Left) => {
                // TUI-103: check if the click is on the scrollbar gutter
                if let Some(sb_rect) = self.turn_modal_scrollbar_rect {
                    if crate::mouse::rect_contains(sb_rect, ev.column, ev.row) {
                        let total = self.turn_modal_total_rows;
                        let viewport = self.turn_modal_viewport_rows;
                        if total > viewport {
                            let geom = ScrollbarGeometry {
                                area_height: viewport,
                                total_items: total,
                                visible_items: viewport,
                                current_offset: self.turn_modal_offset,
                            };
                            // Convert to body-local row
                            let body = self.turn_modal_body_origin?;
                            let local_row = ev.row.saturating_sub(body.y);
                            let local_ev = MouseEvent {
                                row: local_row,
                                ..ev
                            };
                            if let Some(offset) =
                                self.turn_modal_scrollbar_drag.on_mouse(local_ev, geom)
                            {
                                self.emit(Action::TurnModalJumpToOffset(offset));
                            }
                            return Some(EventResult::consumed());
                        }
                    }
                }
                // Click outside scrollbar: fall through to text selection
                if matches!(ev.kind, MouseEventKind::Up(MouseButton::Left)) {
                    self.turn_modal_scrollbar_drag.reset();
                }
                self.feed_turn_modal_selection(ev);
                Some(EventResult::consumed())
            }
            MouseEventKind::ScrollUp => {
                self.turn_modal_selection = None; // COPY-008 rule [6].
                self.emit(Action::TurnModalScrollUp);
                Some(EventResult::consumed())
            }
            MouseEventKind::ScrollDown => {
                self.turn_modal_selection = None; // COPY-008 rule [6].
                self.emit(Action::TurnModalScrollDown);
                Some(EventResult::consumed())
            }
            _ => None,
        }
    }

    /// RPC-094: route mouse wheel events that fall inside the
    /// scrollback rect into the focused SessionContext via the new
    /// `Action::ScrollbackMouseWheel{Up,Down}(velocity)` variants.
    ///
    /// TUI-102: before text selection, hit-test the scrollbar gutter
    /// (rightmost column when gutter is reserved) and route left-button
    /// events through `ScrollbarDrag`. On computed offset, emit
    /// `Action::ScrollbackJumpToOffset`.
    ///
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
                self.text_selection_active = false; // COPY-006 rule [7].
                let step = self.scrollback_wheel.step(WheelDirection::Up);
                let velocity = step.unsigned_abs();
                self.emit(Action::ScrollbackMouseWheelUp(velocity));
                Some(EventResult::consumed())
            }
            MouseEventKind::ScrollDown => {
                self.text_selection_active = false; // COPY-006 rule [7].
                let step = self.scrollback_wheel.step(WheelDirection::Down);
                let velocity = step.unsigned_abs();
                self.emit(Action::ScrollbackMouseWheelDown(velocity));
                Some(EventResult::consumed())
            }
            // TUI-102: left press/drag/release — try scrollbar gutter first,
            // then fall through to text selection (COPY-006 rule [1]).
            MouseEventKind::Down(MouseButton::Left)
            | MouseEventKind::Drag(MouseButton::Left)
            | MouseEventKind::Up(MouseButton::Left) => {
                // TUI-102: check if gutter is reserved (scrollbar visible)
                let gutter_reserved = self.last_scrollback_total_rows
                    > self.last_scrollback_viewport as usize;
                let scrollbar_col = rect.x.saturating_add(rect.width).saturating_sub(1);

                if gutter_reserved && ev.column == scrollbar_col {
                    // TUI-102: convert absolute screen row to scrollbar-relative row
                    let local_row = ev.row.saturating_sub(rect.y);
                    // Hit the scrollbar gutter — route through ScrollbarDrag
                    let viewport = self.last_scrollback_viewport as usize;
                    let total = self.last_scrollback_total_rows;
                    let geom = ScrollbarGeometry {
                        area_height: viewport,
                        total_items: total,
                        visible_items: viewport,
                        current_offset: self.last_scrollback_scroll_offset,
                    };
                    // Feed a local mouse event with the relative row
                    let local_ev = MouseEvent {
                        row: local_row,
                        ..ev
                    };
                    if let Some(offset) = self.scrollback_scrollbar_drag.on_mouse(local_ev, geom) {
                        self.emit(Action::ScrollbackJumpToOffset(offset));
                    }
                    // TUI-102: scrollbar interaction exits stick_to_bottom
                    // (handled by App::dispatch on ScrollbackJumpToOffset)
                    return Some(EventResult::consumed());
                }

                // Click outside scrollbar gutter: fall through to text selection
                self.feed_selection_recognizer(ev, rect);
                Some(EventResult::consumed())
            }
            _ => None,
        }
    }

    /// COPY-007: route a left press/drag/release that lands over the
    /// input rect into the composer's own [`MultiLineInput::handle_mouse`]
    /// selection recognizer. On a Commit gesture (`Some(text)`) the
    /// prompt-free selected text is copied via `Action::CopyToClipboard`.
    /// Returns `None` when the event is not a left button event or falls
    /// outside the input rect (so it bubbles to the key/paste path).
    pub(super) fn handle_composer_mouse(&mut self, ev: MouseEvent) -> Option<EventResult> {
        if !matches!(
            ev.kind,
            MouseEventKind::Down(MouseButton::Left)
                | MouseEventKind::Drag(MouseButton::Left)
                | MouseEventKind::Up(MouseButton::Left)
        ) {
            return None;
        }
        let area = self.last_input_area?;
        let inside = ev.column >= area.x
            && ev.column < area.x.saturating_add(area.width)
            && ev.row >= area.y
            && ev.row < area.y.saturating_add(area.height);
        if !inside {
            return None;
        }
        // Pass the FULL input rect: `handle_mouse` derives the body
        // origin (area.x + INPUT_PAD_X + PROMPT_WIDTH) and body width
        // (`input_body_width(area.width)`) itself, matching the render
        // geometry.
        if let Some(text) = self.input.handle_mouse(ev, area) {
            self.emit(Action::CopyToClipboard(text));
        }
        Some(EventResult::consumed())
    }

    /// COPY-006: feed a left press/drag/release to the selection
    /// recognizer with scrollback-relative coords (subtract the rect
    /// origin) and fan the resulting gestures onto the action bus.
    fn feed_selection_recognizer(&mut self, ev: MouseEvent, rect: Rect) {
        let local = MouseEvent {
            column: ev.column.saturating_sub(rect.x),
            row: ev.row.saturating_sub(rect.y),
            ..ev
        };
        let gestures = self.recognizer.on_mouse(local, std::time::Instant::now());
        self.apply_selection_gestures(&gestures);
    }

    /// COPY-006: poll the recognizer from the run loop's render tick so a
    /// stationary long-press fires its `Begin` gesture (~0.5s).
    pub(crate) fn poll_selection_tick(&mut self) {
        let gestures = self.recognizer.tick(std::time::Instant::now());
        self.apply_selection_gestures(&gestures);
    }

    /// COPY-006: translate recognizer gestures into `Action`s and track
    /// the view-local `text_selection_active` flag (rule [10]). Commit
    /// keeps the flag set so the highlight persists (rule [2]).
    fn apply_selection_gestures(&mut self, gestures: &[SelectionGesture]) {
        for gesture in gestures {
            match gesture {
                SelectionGesture::Begin(cell) => {
                    self.text_selection_active = true;
                    self.emit(Action::SelectionBegin(*cell));
                }
                SelectionGesture::BeginLine(cell) => {
                    self.text_selection_active = true;
                    self.emit(Action::SelectionBeginLine(*cell));
                }
                SelectionGesture::Extend(cell) => self.emit(Action::SelectionExtend(*cell)),
                SelectionGesture::Commit => self.emit(Action::SelectionCommit),
                SelectionGesture::Cancel => {
                    self.text_selection_active = false;
                    self.emit(Action::SelectionClear);
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "mouse_dispatch_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "mouse_dispatch_integration_tests.rs"]
mod integration_tests;

#[cfg(test)]
#[path = "tui103_popup_scrollbar_tests.rs"]
mod tui103_tests;
