//! RPC-028 + TUI-101 — mouse routing for [`super::resume_session_view::ResumeSessionView`].
//!
//! Feature: spec/features/rpc028-scroll-mouse-wrap-parity.feature
//!
//! Extracted from `resume_session_view.rs` so the mode view stays under
//! the 300-LoC source-shape ceiling (rpc026 pin).

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use std::time::Instant;

use crate::components::scroll_viewport::WheelDirection;
use crate::mouse::rect_contains as mouse_rect_contains;
use crate::mouse::scrollbar_drag::ScrollbarGeometry;

use super::resume_session_view::{ResumeSessionView, ResumeSessionViewOutcome};
use codelet_rpc_types::SessionId;

/// Hit-test `ev` against `body_rect`.
fn rect_contains(ev: MouseEvent, body_rect: Rect) -> bool {
    ev.column >= body_rect.x
        && ev.column < body_rect.x + body_rect.width
        && ev.row >= body_rect.y
        && ev.row < body_rect.y + body_rect.height
}

impl ResumeSessionView {
    /// Route a mouse event hit-tested against the view's `body_rect`.
    pub fn handle_mouse(
        &mut self,
        ev: MouseEvent,
        body_rect: Rect,
        visible_rows: usize,
    ) -> ResumeSessionViewOutcome {
        if !rect_contains(ev, body_rect) {
            return ResumeSessionViewOutcome::Ignored;
        }

        // TUI-101: handle scrollbar click-and-drag first for left-button events.
        if matches!(
            ev.kind,
            MouseEventKind::Down(MouseButton::Left)
                | MouseEventKind::Drag(MouseButton::Left)
                | MouseEventKind::Up(MouseButton::Left)
        ) {
            if let Some(sb_rect) = self.last_scrollbar_rect {
                if mouse_rect_contains(sb_rect, ev.column, ev.row) {
                    let visible_sessions = visible_rows / 2;
                    let total = self.sessions.len();
                    if total > visible_sessions {
                        let geom = ScrollbarGeometry {
                            area_height: visible_rows,
                            total_items: total,
                            visible_items: visible_sessions,
                            current_offset: self.scroll_offset,
                        };
                        if let Some(offset) = self.scrollbar_drag.on_mouse(ev, geom) {
                            self.scroll_offset = offset;
                            // Adjust selection to stay visible
                            if self.selected_index >= total {
                                self.selected_index = total - 1;
                            }
                        }
                        return ResumeSessionViewOutcome::Continued;
                    }
                }
            }
            // Click outside scrollbar: reset drag state on Up
            if matches!(ev.kind, MouseEventKind::Up(MouseButton::Left)) {
                self.scrollbar_drag.reset();
            }
            // Fall through to click handling
        }

        match ev.kind {
            MouseEventKind::ScrollUp => {
                let step = self.wheel.step(WheelDirection::Up);
                self.move_by(step, visible_rows);
                ResumeSessionViewOutcome::Continued
            }
            MouseEventKind::ScrollDown => {
                let step = self.wheel.step(WheelDirection::Down);
                self.move_by(step, visible_rows);
                ResumeSessionViewOutcome::Continued
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let candidate = self.scroll_offset + (ev.row - body_rect.y) as usize;
                if candidate < self.sessions.len() {
                    let now = Instant::now();
                    if self.double_click.record_click(candidate, now) {
                        // Double-click: resume session immediately
                        let info = &self.sessions[candidate];
                        return ResumeSessionViewOutcome::Selected(SessionId::new(
                            info.id.clone(),
                        ));
                    }
                    // Single-click: move selection
                    self.selected_index = candidate;
                    self.adjust_scroll(visible_rows);
                    ResumeSessionViewOutcome::Continued
                } else {
                    ResumeSessionViewOutcome::Ignored
                }
            }
            _ => ResumeSessionViewOutcome::Ignored,
        }
    }
}
