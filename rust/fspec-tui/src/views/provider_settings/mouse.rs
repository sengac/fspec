//! RPC-353 — Mouse-wheel handling for ProviderSettingsView (List mode).
//!
//! Feature: spec/features/mode-view-scroll-input.feature
//!
//! Mirrors `/model`'s wheel semantics and the chat view's velocity ramp:
//! `ScrollUp`/`ScrollDown` move the selection (multiple rows under the
//! shared `WheelVelocity` 1×–5× accelerator), then `adjust_scroll()`.
//! Extracted to its own file so `mod.rs` stays under the 300-LoC ceiling.
//!
//! TUI-101: also handles scrollbar click-and-drag via `ScrollbarDrag`.

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::components::scroll_viewport::WheelDirection;
use crate::mouse::rect_contains;
use crate::mouse::scrollbar_drag::ScrollbarGeometry;

use super::{ProviderSettingsEvent, ProviderSettingsView};

impl ProviderSettingsView {
    /// Route a mouse event in List mode. Wheel events advance the selection
    /// by the accelerated velocity (1×–5×) and reconcile the scroll window.
    ///
    /// TUI-101: scrollbar click-and-drag events are handled when the cursor
    /// lands on the scrollbar gutter.
    pub fn handle_mouse(&mut self, ev: MouseEvent) -> ProviderSettingsEvent {
        // BUG-575: a cursor move drops any pending return to a row.
        let before = self.selected_index;
        let event = self.route_mouse(ev);
        if self.selected_index != before {
            self.note_navigation();
        }
        event
    }

    fn route_mouse(&mut self, ev: MouseEvent) -> ProviderSettingsEvent {
        if !matches!(self.mode, super::ProviderSettingsMode::List) {
            return ProviderSettingsEvent::Ignored;
        }

        // TUI-101: handle scrollbar click-and-drag first.
        if matches!(
            ev.kind,
            MouseEventKind::Down(MouseButton::Left)
                | MouseEventKind::Drag(MouseButton::Left)
                | MouseEventKind::Up(MouseButton::Left)
        ) {
            let total = self.nav_items.len();
            let visible = self.visible_rows;
            if total > visible {
                if let Some(sb_rect) = self.last_scrollbar_rect {
                    // BUG-578 F20: a drag that started on the thumb goes on
                    // off the gutter column until the button is released.
                    let follows_drag = self.scrollbar_drag.is_active()
                        && !matches!(ev.kind, MouseEventKind::Down(_));
                    if follows_drag || rect_contains(sb_rect, ev.column, ev.row) {
                        let geom = ScrollbarGeometry {
                            area_height: visible,
                            total_items: total,
                            visible_items: visible,
                            current_offset: self.scroll_offset,
                        };
                        if let Some(offset) = self.scrollbar_drag.on_mouse(ev, geom) {
                            self.scroll_to(offset);
                        }
                        return ProviderSettingsEvent::Consumed;
                    }
                }
                // Click outside scrollbar: reset drag state on Up
                if matches!(ev.kind, MouseEventKind::Up(MouseButton::Left)) {
                    self.scrollbar_drag.reset();
                }
            }
            return ProviderSettingsEvent::Ignored;
        }

        let dir = match ev.kind {
            MouseEventKind::ScrollUp => WheelDirection::Up,
            MouseEventKind::ScrollDown => WheelDirection::Down,
            _ => return ProviderSettingsEvent::Ignored,
        };
        let step = self.wheel.step(dir);
        self.move_clamped(step);
        ProviderSettingsEvent::Consumed
    }
}

impl ProviderSettingsView {
    /// BUG-578 F20: the scrollbar moved the window to `offset`; the
    /// selection moves to the nearest row inside it, so the window stays
    /// where the thumb put it.
    fn scroll_to(&mut self, offset: usize) {
        let total = self.nav_items.len();
        let last_offset = total.saturating_sub(self.visible_rows);
        self.scroll_offset = offset.min(last_offset);
        let last_visible = (self.scroll_offset + self.visible_rows)
            .min(total)
            .saturating_sub(1);
        self.selected_index = self
            .selected_index
            .clamp(self.scroll_offset, last_visible.max(self.scroll_offset));
        self.adjust_scroll();
    }
}
