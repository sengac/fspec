//! BLOCK-011 — Mouse-wheel handling for BlocklistView.
//!
//! Feature: spec/features/blocklist-view-mouse-scroll.feature
//!
//! Mirrors `/model`'s wheel semantics (`model_selector/dispatch.rs`):
//! `ScrollUp`/`ScrollDown` advance the selection by the accelerated
//! velocity (1×–5× under the shared `WheelVelocity` ramp), then the
//! `move_up`/`move_down` movers reconcile the scroll window. Other mouse
//! kinds are ignored so they bubble. Extracted to its own file so
//! `mod.rs` stays under the 300-LoC ceiling.

use crossterm::event::{MouseEvent, MouseEventKind};

use crate::components::scroll_viewport::WheelDirection;

use super::{BlocklistEvent, BlocklistView};

impl BlocklistView {
    /// Route a mouse-wheel event: `ScrollUp`/`ScrollDown` advance the
    /// selection by the accelerated velocity (1×–5×); the movers clamp
    /// the selection and reconcile the scroll window. Every other mouse
    /// kind is ignored so it bubbles to the caller.
    pub fn handle_mouse(&mut self, ev: MouseEvent) -> BlocklistEvent {
        let dir = match ev.kind {
            MouseEventKind::ScrollUp => WheelDirection::Up,
            MouseEventKind::ScrollDown => WheelDirection::Down,
            _ => return BlocklistEvent::Ignored,
        };
        let step = self.wheel.step(dir);
        let mover: fn(&mut Self) = match dir {
            WheelDirection::Up => Self::move_up,
            WheelDirection::Down => Self::move_down,
        };
        for _ in 0..step.unsigned_abs() {
            mover(self);
        }
        BlocklistEvent::Consumed
    }
}
