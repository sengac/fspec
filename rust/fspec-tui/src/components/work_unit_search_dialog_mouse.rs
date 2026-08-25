//! BUG-162 — Mouse handling + scrollbar gutter geometry for
//! [`super::work_unit_search_dialog::WorkUnitSearchDialog`].
//!
//! Feature: spec/features/board-search-dialog-mouse-wheel-scroll.feature
//!
//! Extracted from `work_unit_search_dialog.rs` so that file stays under the
//! 300-LoC budget. Mirrors `views/agent/file_search_popup.rs::handle_mouse`
//! (the reference consumer): hit-test the last-rendered dialog rect, route
//! left-button press/drag/release on the scrollbar gutter through the
//! shared `ScrollbarDrag` state machine with `ScrollbarGeometry` (absolute
//! row converted to body-local row), and advance the shared
//! `WheelVelocity` for ScrollUp/ScrollDown. Outside the rect every event is
//! `Ignored` so it bubbles to the BoardView behind the modal.
//!
//! The dialog's mouse-state fields are `pub(super)` so this sibling
//! module reads them directly (no visibility-widening accessor methods).

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use super::scroll_viewport::{WheelDirection, WheelVelocity};
use super::work_unit_search_dialog::WorkUnitSearchDialog;
use super::EventResult;
use crate::mouse::rect_contains;
use crate::mouse::scrollbar_drag::ScrollbarGeometry;

/// BUG-162: the 1-column scrollbar gutter rect for a dialog `rect` whose
/// body shows `visible` rows. `None` when the rect is too small.
///
/// Bordered body geometry (see `render_dialog_at`, mirrors
/// `help_dialog_scroll::gutter_rect` plus the BUG-159 pinned query row):
///   * body.y = rect.y + 2 (1 border + 1 padding);
///   * content starts at body.y + 2 (title + gap) + 1 (query row)
///     = rect.y + 5;
///   * body rightmost column = rect.x + rect.width - 3
///     (1 border + 1 padding on the right, then the last body column).
pub(super) fn scrollbar_gutter(rect: Rect, visible: usize) -> Option<Rect> {
    if rect.width < 4 || visible == 0 {
        return None;
    }
    let content_y = rect.y + 5;
    let gutter_x = rect.x + rect.width - 3;
    let height = (visible.min(rect.height.saturating_sub(6) as usize)) as u16;
    Some(Rect {
        x: gutter_x,
        y: content_y,
        width: 1,
        height,
    })
}

impl WorkUnitSearchDialog {
    /// BUG-162: reset the wheel-velocity accumulator and the scrollbar
    /// drag state machine so a stale drag cannot misfire after the
    /// query or the search mode changes the match list.
    pub(super) fn reset_mouse_state(&mut self) {
        self.wheel = WheelVelocity::new();
        self.scrollbar_drag.reset();
    }

    /// BUG-162: route a mouse event hit-tested against the dialog's
    /// last-rendered rect. Outside the rect → `Ignored` so the event
    /// bubbles to the BoardView behind the modal.
    ///
    /// ScrollUp/ScrollDown inside the rect advance the shared
    /// `WheelVelocity` (1x–5x ramp) and move the selection via the same
    /// `move_by` the keyboard navigation uses. Left-button
    /// press/drag/release on the scrollbar gutter is routed through the
    /// shared `ScrollbarDrag` state machine; the returned offset is
    /// applied to `scroll_offset` and the selection is clamped.
    pub(super) fn handle_mouse(&mut self, ev: MouseEvent) -> EventResult {
        let Some(rect) = self.last_dialog_rect else {
            return EventResult::ignored();
        };
        if !rect_contains(rect, ev.column, ev.row) {
            return EventResult::ignored();
        }
        if matches!(
            ev.kind,
            MouseEventKind::Down(MouseButton::Left)
                | MouseEventKind::Drag(MouseButton::Left)
                | MouseEventKind::Up(MouseButton::Left)
        ) {
            let total = self.matches.len();
            let visible = self.visible_rows();
            let Some(sb_rect) = self.last_scrollbar_rect else {
                return EventResult::ignored();
            };
            if total > visible && rect_contains(sb_rect, ev.column, ev.row) {
                // Convert the absolute screen row to a body-local row
                // (same trick as file_search_popup.rs).
                let local_row = ev.row.saturating_sub(sb_rect.y);
                let local_ev = MouseEvent {
                    row: local_row,
                    ..ev
                };
                let geom = ScrollbarGeometry {
                    area_height: visible,
                    total_items: total,
                    visible_items: visible,
                    current_offset: self.scroll_offset,
                };
                if let Some(offset) = self.scrollbar_drag.on_mouse(local_ev, geom) {
                    self.scroll_offset = offset;
                    if self.selected >= total {
                        self.selected = total - 1;
                    }
                }
                return EventResult::consumed();
            }
            // Click outside the scrollbar: reset the drag state on Up so
            // a stale press cannot misfire later.
            if matches!(ev.kind, MouseEventKind::Up(MouseButton::Left)) {
                self.scrollbar_drag.reset();
            }
            return EventResult::ignored();
        }
        match ev.kind {
            MouseEventKind::ScrollUp => {
                let step = self.wheel.step(WheelDirection::Up);
                self.move_by(step);
                EventResult::consumed()
            }
            MouseEventKind::ScrollDown => {
                let step = self.wheel.step(WheelDirection::Down);
                self.move_by(step);
                EventResult::consumed()
            }
            _ => EventResult::ignored(),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn scrollbar_gutter_is_one_column_on_the_right_of_the_body() {
        // @step Given a fixed dialog rect at (10, 3) sized 60x18
        let r = Rect {
            x: 10,
            y: 3,
            width: 60,
            height: 18,
        };
        // @step When the scrollbar gutter is computed for 10 visible rows
        let g = scrollbar_gutter(r, 10).expect("gutter");
        // @step Then the gutter is one column wide on the body right edge
        assert_eq!(g.width, 1);
        assert_eq!(g.x, 10 + 60 - 3);
        // @step And it starts below the title, gap and pinned query row
        assert_eq!(g.y, 3 + 5);
        assert_eq!(g.height, 10);
    }

    #[test]
    fn scrollbar_gutter_is_none_when_the_rect_is_too_small() {
        assert!(scrollbar_gutter(Rect::new(0, 0, 3, 10), 5).is_none());
        assert!(scrollbar_gutter(Rect::new(0, 0, 60, 10), 0).is_none());
    }
}
