//! RPC-353 — Mouse-wheel handling for ProviderSettingsView (List mode).
//!
//! Feature: spec/features/mode-view-scroll-input.feature
//!
//! Mirrors `/model`'s wheel semantics and the chat view's velocity ramp:
//! `ScrollUp`/`ScrollDown` move the selection (multiple rows under the
//! shared `WheelVelocity` 1×–5× accelerator), then `adjust_scroll()`.
//! Extracted to its own file so `mod.rs` stays under the 300-LoC ceiling.

use crossterm::event::{MouseEvent, MouseEventKind};

use crate::components::scroll_viewport::WheelDirection;

use super::{ProviderSettingsEvent, ProviderSettingsView};

impl ProviderSettingsView {
    /// Route a mouse-wheel event in List mode. `ScrollUp`/`ScrollDown`
    /// advance the selection by the accelerated velocity (1×–5×) and
    /// reconcile the scroll window; other mouse kinds are ignored so they
    /// bubble. Only List mode consumes the wheel — every other mode is a
    /// pass-through (Ignored).
    pub fn handle_mouse(&mut self, ev: MouseEvent) -> ProviderSettingsEvent {
        if !matches!(self.mode, super::ProviderSettingsMode::List) {
            return ProviderSettingsEvent::Ignored;
        }
        let dir = match ev.kind {
            MouseEventKind::ScrollUp => WheelDirection::Up,
            MouseEventKind::ScrollDown => WheelDirection::Down,
            _ => return ProviderSettingsEvent::Ignored,
        };
        let step = self.wheel.step(dir);
        let before = self.selected_index;
        self.move_clamped(step);
        if self.selected_index != before {
            self.clear_test_result();
        }
        ProviderSettingsEvent::Consumed
    }
}
