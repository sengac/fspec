//! RPC-028 + TUI-103 — mouse routing for [`super::file_search_popup::FileSearchPopup`].
//!
//! Feature: spec/features/rpc028-scroll-mouse-wrap-parity.feature
//!
//! Extracted from `file_search_popup.rs` so the popup file stays under
//! the 300-LoC source-shape ceiling (rpc018/rpc020 pins).

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use crate::components::dialog_theme::{dialog_rect, FspecDialog};
use crate::components::scroll_viewport::WheelDirection;
use crate::mouse::rect_contains;
use crate::mouse::scrollbar_drag::ScrollbarGeometry;

use super::file_search_popup::{FilePopupOutcome, FileSearchPopup};

/// TUI-103: derive (scrollbar gutter rect, body origin) from the
/// shrink-to-content dialog rect for `area`.
pub(super) fn scrollbar_geometry(
    dialog: &FspecDialog<'_>,
    visible_rows: usize,
    total_items: usize,
    area: Rect,
) -> (Option<Rect>, Rect) {
    let d_rect = dialog_rect(area, dialog);
    let body_origin = Rect {
        x: d_rect.x + 2,
        y: d_rect.y + 4,
        width: d_rect.width.saturating_sub(4).max(1),
        height: d_rect.height.saturating_sub(4).max(1),
    };
    let show_scrollbar = total_items > visible_rows;
    let sb_rect = if show_scrollbar {
        let scrollbar_col = body_origin.x + body_origin.width - 1;
        Some(Rect {
            x: scrollbar_col,
            y: body_origin.y,
            width: 1,
            height: body_origin.height,
        })
    } else {
        None
    };
    (sb_rect, body_origin)
}

impl FileSearchPopup {
    /// Route a mouse event hit-tested against the popup's last-rendered
    /// rect. Outside the rect → `Ignored` so the caller can bubble.
    ///
    /// TUI-103: left-button press/drag/release on the scrollbar gutter
    /// column are routed through `ScrollbarDrag` before wheel events.
    pub fn handle_mouse(&mut self, ev: MouseEvent, popup_rect: Rect) -> FilePopupOutcome {
        let inside = ev.column >= popup_rect.x
            && ev.column < popup_rect.x + popup_rect.width
            && ev.row >= popup_rect.y
            && ev.row < popup_rect.y + popup_rect.height;
        if !inside {
            return FilePopupOutcome::Ignored;
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
                    let visible = self.visible_rows();
                    if total > visible {
                        // TUI-103: convert absolute screen row to body-local row
                        #[allow(clippy::expect_used)]
                        let body = self
                            .last_body_origin
                            .expect("body origin must be set when scrollbar rect is set");
                        let local_row = ev.row.saturating_sub(body.y);
                        let local_ev = MouseEvent {
                            row: local_row,
                            ..ev
                        };
                        let geom = ScrollbarGeometry {
                            area_height: body.height as usize,
                            total_items: total,
                            visible_items: visible,
                            current_offset: self.scroll_offset,
                        };
                        if let Some(offset) = self.scrollbar_drag.on_mouse(local_ev, geom) {
                            self.scroll_offset = offset;
                            // Adjust selection to stay visible
                            if self.selected_index >= total {
                                self.selected_index = total - 1;
                            }
                        }
                        return FilePopupOutcome::Continued;
                    }
                }
            }
            // Click outside scrollbar: reset drag state on Up
            if matches!(ev.kind, MouseEventKind::Up(MouseButton::Left)) {
                self.scrollbar_drag.reset();
            }
            return FilePopupOutcome::Ignored;
        }

        match ev.kind {
            MouseEventKind::ScrollUp => {
                let step = self.wheel.step(WheelDirection::Up);
                self.move_by(step);
                FilePopupOutcome::Continued
            }
            MouseEventKind::ScrollDown => {
                let step = self.wheel.step(WheelDirection::Down);
                self.move_by(step);
                FilePopupOutcome::Continued
            }
            _ => FilePopupOutcome::Ignored,
        }
    }

}
