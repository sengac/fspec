//! RPC-028 + TUI-103 — mouse routing for [`super::search_history_view::SearchHistoryView`].
//!
//! Feature: spec/features/rpc028-scroll-mouse-wrap-parity.feature
//!
//! Extracted from `search_history_view.rs` so the mode view stays under
//! the 300-LoC source-shape ceiling (rpc026 pin).

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use crate::components::scroll_viewport::WheelDirection;
use crate::mouse::rect_contains;
use crate::mouse::scrollbar_drag::ScrollbarGeometry;

use super::search_history_view::{SearchHistoryView, SearchHistoryViewOutcome};

impl SearchHistoryView {
    /// Route a mouse event hit-tested against the view's `body_rect`.
    ///
    /// TUI-103: left-button press/drag/release on the scrollbar gutter
    /// column are routed through `ScrollbarDrag` before wheel events.
    pub fn handle_mouse(
        &mut self,
        ev: MouseEvent,
        body_rect: Rect,
        visible_rows: usize,
    ) -> SearchHistoryViewOutcome {
        let inside = ev.column >= body_rect.x
            && ev.column < body_rect.x + body_rect.width
            && ev.row >= body_rect.y
            && ev.row < body_rect.y + body_rect.height;
        if !inside {
            return SearchHistoryViewOutcome::Ignored;
        }

        // TUI-103: handle scrollbar click-and-drag for left-button events
        if matches!(
            ev.kind,
            MouseEventKind::Down(MouseButton::Left)
                | MouseEventKind::Drag(MouseButton::Left)
                | MouseEventKind::Up(MouseButton::Left)
        ) {
            if let Some(sb_rect) = self.last_scrollbar_rect {
                if rect_contains(sb_rect, ev.column, ev.row) {
                    let total = self.matches.len();
                    if total > visible_rows {
                        // TUI-103: convert absolute screen row to body-local row
                        let local_row = ev.row.saturating_sub(sb_rect.y);
                        let local_ev = MouseEvent {
                            row: local_row,
                            ..ev
                        };
                        let geom = ScrollbarGeometry {
                            area_height: sb_rect.height as usize,
                            total_items: total,
                            visible_items: visible_rows,
                            current_offset: self.scroll_offset,
                        };
                        if let Some(offset) = self.scrollbar_drag.on_mouse(local_ev, geom) {
                            self.scroll_offset = offset;
                            // Adjust selection to stay visible
                            if self.selected_index >= total {
                                self.selected_index = total - 1;
                            }
                        }
                        return SearchHistoryViewOutcome::Continued;
                    }
                }
            }
            // Click outside scrollbar: reset drag state on Up
            if matches!(ev.kind, MouseEventKind::Up(MouseButton::Left)) {
                self.scrollbar_drag.reset();
            }
            return SearchHistoryViewOutcome::Ignored;
        }

        match ev.kind {
            MouseEventKind::ScrollUp => {
                let step = self.wheel.step(WheelDirection::Up);
                self.move_by(step, visible_rows);
                SearchHistoryViewOutcome::Continued
            }
            MouseEventKind::ScrollDown => {
                let step = self.wheel.step(WheelDirection::Down);
                self.move_by(step, visible_rows);
                SearchHistoryViewOutcome::Continued
            }
            _ => SearchHistoryViewOutcome::Ignored,
        }
    }
}
