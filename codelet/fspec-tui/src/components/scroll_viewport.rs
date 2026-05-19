//! RPC-028 — Shared scroll-viewport helpers.
//!
//! Feature: spec/features/rpc028-scroll-mouse-wrap-parity.feature
//!
//! This file is intentionally written test-first under the testing
//! phase of RPC-028. The implementation comes in a follow-up step.
//!
//! Three primitives live here:
//!   * `wrap_index(current, delta, total)` — rem_euclid wrap-around for
//!     selection movement (mirrors `BoardStore::move_selection` at
//!     `store/board_viewport.rs:42-44`).
//!   * `ensure_visible(scroll_offset, selected, visible_rows, total)` —
//!     popup-flavour ensure-visible (simpler than BoardView's two-pass
//!     arrow correction; popups embed their `↑`/`↓` glyphs into the
//!     same rows they render).
//!   * `WheelVelocity` — 1×–5× ramp shared by views that want
//!     mouse-wheel acceleration; mirrors TS
//!     `AgentView.tsx:4435-4458`.

use std::cell::Cell;
use std::time::{Duration, Instant};

/// Compute the new index when moving by `delta` over a list of length
/// `total`, wrapping around in both directions. Returns 0 when
/// `total == 0`.
///
/// Uses `i64::rem_euclid` so any signed delta — including ones whose
/// magnitude exceeds `total` — produces a valid `[0, total)` index.
pub fn wrap_index(current: usize, delta: i32, total: usize) -> usize {
    if total == 0 {
        return 0;
    }
    let total_i = total as i64;
    let proposed = (current as i64).saturating_add(delta as i64);
    proposed.rem_euclid(total_i) as usize
}

/// Adjust `scroll_offset` so `selected` lies inside the visible window
/// `[scroll_offset, scroll_offset + visible_rows)`.
///
/// `visible_rows == 0` or `total == 0` resets `scroll_offset` to 0.
///
/// Unlike BoardView's `adjust_scroll_offset`, this helper does NOT
/// account for arrow glyphs consuming rows — popups draw their `↑`/`↓`
/// glyphs into the body rows alongside the cards.
pub fn ensure_visible(
    scroll_offset: &mut usize,
    selected: usize,
    visible_rows: usize,
    total: usize,
) {
    if visible_rows == 0 || total == 0 {
        *scroll_offset = 0;
        return;
    }
    if selected < *scroll_offset {
        *scroll_offset = selected;
    } else if selected >= *scroll_offset + visible_rows {
        *scroll_offset = selected + 1 - visible_rows;
    }
    // Clamp to keep the window inside `total` whenever possible.
    let max_offset = total.saturating_sub(visible_rows);
    if *scroll_offset > max_offset {
        *scroll_offset = max_offset;
    }
}

/// Direction of a wheel step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WheelDirection {
    Up,
    Down,
}

/// Mouse-wheel velocity accumulator.
///
/// `step` returns the magnitude (with sign) of the move the caller
/// should apply. Velocity caps at 5 when wheel events arrive faster
/// than every 150 ms; a 150 ms+ gap resets velocity to 1.
pub struct WheelVelocity {
    last: Cell<Option<Instant>>,
    velocity: Cell<u32>,
}

impl Default for WheelVelocity {
    fn default() -> Self {
        Self::new()
    }
}

impl WheelVelocity {
    pub const MAX_VELOCITY: u32 = 5;
    pub const GAP_THRESHOLD: Duration = Duration::from_millis(150);

    pub fn new() -> Self {
        Self {
            last: Cell::new(None),
            velocity: Cell::new(1),
        }
    }

    /// Advance the accumulator and return the signed step the caller
    /// should apply (`±velocity`).
    pub fn step(&self, dir: WheelDirection) -> i32 {
        self.step_at(dir, Instant::now())
    }

    /// Same as [`step`] but injectable for tests.
    pub fn step_at(&self, dir: WheelDirection, now: Instant) -> i32 {
        let velocity = match self.last.get() {
            Some(prev) if now.saturating_duration_since(prev) < Self::GAP_THRESHOLD => {
                (self.velocity.get() + 1).min(Self::MAX_VELOCITY)
            }
            _ => 1,
        };
        self.velocity.set(velocity);
        self.last.set(Some(now));
        match dir {
            WheelDirection::Up => -(velocity as i32),
            WheelDirection::Down => velocity as i32,
        }
    }

    /// Current velocity (1..=MAX_VELOCITY). Exposed for assertions.
    pub fn velocity(&self) -> u32 {
        self.velocity.get()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    // -------------------------------------------------------------
    // wrap_index — RPC-028 scenario "scroll_viewport::wrap_index
    // wraps in both directions using rem_euclid"
    // -------------------------------------------------------------

    #[test]
    fn wrap_index_returns_zero_when_total_is_zero() {
        // @step Given the shared scroll_viewport module is loaded
        // @step When wrap_index(0, -1, 5) is called
        // @step Then it returns 4
        assert_eq!(wrap_index(0, -1, 5), 4);
    }

    #[test]
    fn wrap_index_wraps_forward_past_end() {
        // @step And wrap_index(4, 1, 5) returns 0
        assert_eq!(wrap_index(4, 1, 5), 0);
    }

    #[test]
    fn wrap_index_wraps_when_delta_exceeds_total() {
        // @step And wrap_index(2, 10, 5) returns 2
        assert_eq!(wrap_index(2, 10, 5), 2);
    }

    #[test]
    fn wrap_index_handles_zero_total() {
        assert_eq!(wrap_index(0, 1, 0), 0);
        assert_eq!(wrap_index(0, -1, 0), 0);
    }

    #[test]
    fn wrap_index_handles_large_negative_delta() {
        // -7 over a list of 5 — should land on (0 - 7).rem_euclid(5) = 3
        assert_eq!(wrap_index(0, -7, 5), 3);
    }

    // -------------------------------------------------------------
    // ensure_visible — RPC-028 scenario "scroll_viewport::ensure_visible
    // scrolls down when selected is past the window"
    // -------------------------------------------------------------

    #[test]
    fn ensure_visible_scrolls_down_when_selected_is_past_window() {
        // @step Given scroll_offset is 0 and visible_rows is 8 and total is 20
        let mut scroll_offset = 0usize;
        // @step When ensure_visible(&mut scroll_offset, 10, 8, 20) is called
        ensure_visible(&mut scroll_offset, 10, 8, 20);
        // @step Then scroll_offset is updated so 10 lies in [scroll_offset, scroll_offset + 8)
        assert!(scroll_offset <= 10);
        assert!(10 < scroll_offset + 8);
        // Specifically: selected = 10, window of 8 → offset = 3 (rows 3..11)
        assert_eq!(scroll_offset, 3);
    }

    #[test]
    fn ensure_visible_scrolls_up_when_selected_is_before_window() {
        let mut scroll_offset = 10usize;
        ensure_visible(&mut scroll_offset, 4, 8, 20);
        assert_eq!(scroll_offset, 4);
    }

    #[test]
    fn ensure_visible_clamps_to_max_offset() {
        let mut scroll_offset = 100usize;
        // total - visible_rows = 20 - 8 = 12
        ensure_visible(&mut scroll_offset, 19, 8, 20);
        assert_eq!(scroll_offset, 12);
    }

    #[test]
    fn ensure_visible_resets_when_visible_rows_zero() {
        let mut scroll_offset = 5usize;
        ensure_visible(&mut scroll_offset, 3, 0, 20);
        assert_eq!(scroll_offset, 0);
    }

    #[test]
    fn ensure_visible_resets_when_total_zero() {
        let mut scroll_offset = 5usize;
        ensure_visible(&mut scroll_offset, 0, 8, 0);
        assert_eq!(scroll_offset, 0);
    }

    #[test]
    fn ensure_visible_leaves_offset_alone_when_selected_inside_window() {
        let mut scroll_offset = 4usize;
        ensure_visible(&mut scroll_offset, 6, 8, 20);
        assert_eq!(scroll_offset, 4);
    }

    // -------------------------------------------------------------
    // WheelVelocity — RPC-028 scenario "WheelVelocity ramps up to 5x
    // within 150ms then resets after the gap"
    // -------------------------------------------------------------

    #[test]
    fn wheel_velocity_ramps_up_within_threshold() {
        // @step Given a fresh WheelVelocity
        let wv = WheelVelocity::new();
        let t0 = Instant::now();
        // @step When the user emits 5 ScrollDown events within 100ms of each other
        let _ = wv.step_at(WheelDirection::Down, t0);
        let _ = wv.step_at(WheelDirection::Down, t0 + Duration::from_millis(50));
        let _ = wv.step_at(WheelDirection::Down, t0 + Duration::from_millis(100));
        let _ = wv.step_at(WheelDirection::Down, t0 + Duration::from_millis(140));
        let step5 = wv.step_at(WheelDirection::Down, t0 + Duration::from_millis(180));
        // @step Then the 5th step reports velocity 5
        // (note: 180ms from t0 but the gap between step4 and step5 is 40ms — still under 150ms)
        assert_eq!(step5, 5);
        assert_eq!(wv.velocity(), 5);
    }

    #[test]
    fn wheel_velocity_resets_after_gap() {
        let wv = WheelVelocity::new();
        let t0 = Instant::now();
        // Ramp velocity up
        for i in 0..5 {
            let _ = wv.step_at(WheelDirection::Down, t0 + Duration::from_millis(i * 50));
        }
        assert_eq!(wv.velocity(), 5);
        // @step And after a gap of >=150ms the next step resets velocity to 1
        let step = wv.step_at(WheelDirection::Down, t0 + Duration::from_millis(5 * 50 + 200));
        assert_eq!(step, 1);
        assert_eq!(wv.velocity(), 1);
    }

    #[test]
    fn wheel_velocity_up_is_negative() {
        let wv = WheelVelocity::new();
        let step = wv.step_at(WheelDirection::Up, Instant::now());
        assert_eq!(step, -1);
    }

    #[test]
    fn wheel_velocity_caps_at_max() {
        let wv = WheelVelocity::new();
        let t0 = Instant::now();
        for i in 0..20 {
            let _ = wv.step_at(WheelDirection::Down, t0 + Duration::from_millis(i * 30));
        }
        assert_eq!(wv.velocity(), WheelVelocity::MAX_VELOCITY);
    }
}
