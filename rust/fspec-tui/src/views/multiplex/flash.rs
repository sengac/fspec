//! MUX-006 — pure focus-flash pattern math (right-to-left scan strip).
//!
//! Feature: spec/features/mux-focus-flash-animation.feature
//!
//! The focus flash is a 350ms background animation over the ENTIRE
//! newly-selected mux pane, tinted with the mux footer dark purple
//! (MUX-005). This module is PURE: given (pane rect, clock ms) it
//! returns the exact set of cells to tint — no buffer, no state. The
//! pattern is deterministic for a given (rect, clock) (R8).
//!
//! Pattern (R3, single animation — no burst ring, no rain):
//!   a full-height 2-column scan strip sweeping from the RIGHT edge of
//!   the pane to the LEFT across the 350ms window (right-to-left).
//!
//! The run loop owns the clock (TUI-106 SSR decision): the render path
//! advances the flash clock by [`FLASH_FRAME_MS`] per rendered mux
//! frame; no tokio timers live in the view.
//!
//! MUX-007: once the 350ms window has elapsed the strip does NOT
//! vanish — it SETTLES: the fn is total in `clock_ms` and every clock
//! past the window returns the same cells as [`LAST_PAINT_MS`] (the
//! left-edge strip), so the focused pane keeps its accent until focus
//! moves or mux is disabled.

use ratatui::layout::Rect;

/// MUX-006: total flash window (350ms, R2).
pub const FLASH_MS: u64 = 350;
/// MUX-006: one rendered frame advances the flash clock by this much
/// (16ms — matches the run-loop `RENDER_TICK` and the AgentView
/// animation-clock cadence).
pub const FLASH_FRAME_MS: u64 = 16;
/// MUX-006: the last clock value that gets painted. The render path
/// paints at the current clock and THEN advances, so the frames land at
/// 0, 16, …, 336; the next advance (352 ≥ 350) ends the flash. The
/// sweep is timed so the last painted frame sits exactly at the left
/// edge.
pub const LAST_PAINT_MS: u64 = FLASH_MS / FLASH_FRAME_MS * FLASH_FRAME_MS;

/// MUX-006: the exact set of `(x, y)` cells to tint for the frame at
/// `clock_ms` inside `rect`. Deterministic, unique, and always inside
/// `rect`. Empty only for degenerate rects.
///
/// The flash is a single full-height scan strip (R3): 2 columns wide,
/// all rows of the pane, starting at the pane's right edge and
/// sweeping left as `clock_ms` grows; at [`LAST_PAINT_MS`] the strip
/// sits exactly at the left edge. MUX-007: clocks past the 350ms
/// window (`clock_ms >= FLASH_MS`) settle to that same left-edge strip
/// instead of yielding nothing — the focused pane keeps its accent
/// until focus moves or mux is disabled.
pub fn flash_cells(rect: Rect, clock_ms: u64) -> Vec<(u16, u16)> {
    if rect.width == 0 || rect.height == 0 {
        return Vec::new();
    }
    let w = rect.width as u64;
    // MUX-007: the strip is total in the clock — clocks past the window
    // settle at the last paintable frame (the left edge) rather than
    // vanishing.
    let local = clock_ms.min(LAST_PAINT_MS);
    // The strip's left column travels from `w - 2` (right edge: strip
    // covers columns [w-2, w-1]) down to `0` (left edge: strip covers
    // columns [0, 1]) across the window — always exactly 2 columns
    // wide for panes ≥ 2 columns, always fully inside the rect.
    let travel = w.saturating_sub(2);
    let offset = travel * local / LAST_PAINT_MS;
    let left_col = travel - offset;
    let mut cells = Vec::new();
    for col in left_col..(left_col + 2).min(w) {
        for y in 0..rect.height {
            cells.push((rect.x + col as u16, rect.y + y));
        }
    }
    cells
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use ratatui::layout::Rect;

    fn pane() -> Rect {
        Rect::new(0, 0, 120, 23)
    }

    #[test]
    fn cells_are_deterministic_and_inside_the_rect() {
        for clock in (0..=FLASH_MS).step_by(16) {
            let cells = flash_cells(pane(), clock);
            assert_eq!(flash_cells(pane(), clock), cells, "clock {clock}");
            for (x, y) in &cells {
                assert!(
                    *x < 120 && *y < 23,
                    "cell ({x}, {y}) escaped the pane at clock {clock}"
                );
            }
            let mut dedup = cells.clone();
            dedup.sort();
            dedup.dedup();
            assert_eq!(dedup.len(), cells.len(), "duplicates at clock {clock}");
        }
    }

    #[test]
    fn window_elapse_yields_the_settled_left_edge_strip() {
        // MUX-007: once the 350ms window has elapsed the flash does not
        // vanish — it SETTLES: the left-edge strip (the LAST_PAINT_MS
        // frame) is painted on every subsequent frame of the focused
        // pane. `flash_cells` is total: clocks past the window return the
        // same settled cells as `LAST_PAINT_MS`.
        for clock in [FLASH_MS, FLASH_MS + 16, FLASH_MS + 16 * 10] {
            let settled = flash_cells(pane(), clock);
            assert_eq!(
                flash_cells(pane(), LAST_PAINT_MS),
                settled,
                "clock {clock} must settle to the left-edge strip"
            );
            assert!(!settled.is_empty(), "settled strip must paint cells");
            // Left edge: columns 0..2 of the pane.
            assert!(
                settled
                    .iter()
                    .all(|&(x, _)| x == 0 || x == 1),
                "settled strip must sit at the pane's left edge: {settled:?}"
            );
        }
        assert!(flash_cells(Rect::new(0, 0, 0, 10), 16).is_empty());
        assert!(flash_cells(Rect::new(0, 0, 10, 0), 16).is_empty());
    }

    #[test]
    fn strip_is_full_height_and_two_columns_wide() {
        for clock in (0..FLASH_MS).step_by(16) {
            let cells = flash_cells(pane(), clock);
            assert!(!cells.is_empty(), "scan strip must paint cells");
            let mut rows: Vec<u16> = cells.iter().map(|&(_, y)| y).collect();
            rows.sort();
            rows.dedup();
            assert_eq!(rows.len(), 23, "strip must span all rows at clock {clock}");
            let cols: Vec<u16> = cells.iter().map(|&(x, _)| x).collect();
            let min = *cols.iter().min().expect("non-empty");
            let max = *cols.iter().max().expect("non-empty");
            assert_eq!(
                (max - min).saturating_add(1),
                2,
                "strip must be exactly 2 columns at clock {clock}"
            );
        }
    }

    #[test]
    fn strip_sweeps_right_to_left() {
        // The leftmost strip column must be monotonically non-increasing
        // (the strip moves leftward, never rightward) across the window.
        let start_left = flash_cells(pane(), 0)
            .iter()
            .map(|&(x, _)| x)
            .min()
            .expect("non-empty");
        assert_eq!(start_left, 118, "flash must start at the right edge");
        let mut prev = start_left;
        for clock in (16..FLASH_MS).step_by(16) {
            let left = flash_cells(pane(), clock)
                .iter()
                .map(|&(x, _)| x)
                .min()
                .expect("non-empty");
            assert!(
                left <= prev,
                "strip must sweep right-to-left: leftmost column must not increase: {prev} -> {left} at clock {clock}"
            );
            prev = left;
        }
        let end_left = flash_cells(pane(), FLASH_MS - 1)
            .iter()
            .map(|&(x, _)| x)
            .min()
            .expect("non-empty");
        assert_eq!(end_left, 0, "flash must end at the left edge");
    }

    #[test]
    fn narrow_panes_stay_visible_and_inside() {
        // A 1-column pane: the strip is clamped to the single column.
        let cells = flash_cells(Rect::new(0, 0, 1, 3), 16);
        assert!(!cells.is_empty());
        assert!(cells.iter().all(|&(x, _)| x == 0));
        // A 2-column pane: the strip covers both columns for the whole
        // window and stays inside.
        for clock in (0..FLASH_MS).step_by(16) {
            let cells = flash_cells(Rect::new(0, 0, 2, 4), clock);
            assert_eq!(cells.len(), 8, "clock {clock}: {cells:?}");
            assert!(
                cells.iter().all(|&(x, y)| x < 2 && y < 4),
                "clock {clock}"
            );
        }
    }
}
