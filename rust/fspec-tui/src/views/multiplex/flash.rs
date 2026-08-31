//! MUX-006 — pure focus-flash pattern math (bottom-to-top scan row).
//!
//! Feature: spec/features/mux-focus-flash-animation.feature
//! (geometry superseded 2026-08-31 by MUX-008 —
//! spec/features/focus-flash-scans-bottom-to-top-and-settles-as-a-1-row-top-bar.feature)
//!
//! The focus flash is a 350ms background animation over the ENTIRE
//! newly-selected mux pane, tinted with the mux footer dark purple
//! (MUX-005). This module is PURE: given (pane rect, clock ms) it
//! returns the exact set of cells to tint — no buffer, no state. The
//! pattern is deterministic for a given (rect, clock) (R8).
//!
//! Pattern (MUX-008 R1, single animation — no burst ring, no rain):
//!   a single 1-ROW-HIGH scan row spanning the pane's FULL WIDTH,
//!   sweeping from the pane's BOTTOM edge up to its TOP across the
//!   350ms window (bottom-to-top).
//!
//! The run loop owns the clock (TUI-106 SSR decision): the render path
//! advances the flash clock by [`FLASH_FRAME_MS`] per rendered mux
//! frame; no tokio timers live in the view.
//!
//! MUX-007 (settled final frame, MUX-008 R2): once the 350ms window
//! has elapsed the row does NOT vanish — it SETTLES: the fn is total in
//! `clock_ms` and every clock past the window returns the same cells
//! as [`LAST_PAINT_MS`] (the top-row bar), so the focused pane keeps
//! its accent until focus moves or mux is disabled.

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
/// sweep is timed so the last painted frame sits exactly at the top
/// edge (which is also the settled frame — MUX-007/MUX-008).
pub const LAST_PAINT_MS: u64 = FLASH_MS / FLASH_FRAME_MS * FLASH_FRAME_MS;

/// MUX-006/MUX-008: the exact set of `(x, y)` cells to tint for the
/// frame at `clock_ms` inside `rect`. Deterministic, unique, and always
/// inside `rect`. Empty only for degenerate rects.
///
/// The flash is a single 1-row-high scan row (MUX-008 R1): it spans all
/// columns of the pane and its row travels from the pane's bottom edge
/// up to its top as `clock_ms` grows; at [`LAST_PAINT_MS`] the row sits
/// exactly on the top row. MUX-007/MUX-008 R2: clocks past the 350ms
/// window (`clock_ms >= FLASH_MS`) settle to that same top-row bar
/// instead of yielding nothing — the focused pane keeps its accent
/// until focus moves or mux is disabled.
pub fn flash_cells(rect: Rect, clock_ms: u64) -> Vec<(u16, u16)> {
    if rect.width == 0 || rect.height == 0 {
        return Vec::new();
    }
    let h = rect.height as u64;
    // MUX-007/MUX-008: the row is total in the clock — clocks past the
    // window settle at the last paintable frame (the top row) rather
    // than vanishing.
    let local = clock_ms.min(LAST_PAINT_MS);
    // The row's y travels from `h - 1` (bottom edge) up to `0` (top
    // edge) across the window — always exactly 1 row high, always
    // fully inside the rect.
    let travel = h.saturating_sub(1);
    let offset = travel * local / LAST_PAINT_MS;
    let top_row = travel - offset;
    let y = rect.y + top_row as u16;
    (0..rect.width).map(|x| (rect.x + x, y)).collect()
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
    fn window_elapse_yields_the_settled_top_row_bar() {
        // MUX-007/MUX-008: once the 350ms window has elapsed the flash
        // does not vanish — it SETTLES: the top-row bar (the
        // LAST_PAINT_MS frame) is painted on every subsequent frame of
        // the focused pane. `flash_cells` is total: clocks past the
        // window return the same settled cells as `LAST_PAINT_MS`.
        for clock in [FLASH_MS, FLASH_MS + 16, FLASH_MS + 16 * 10] {
            let settled = flash_cells(pane(), clock);
            assert_eq!(
                flash_cells(pane(), LAST_PAINT_MS),
                settled,
                "clock {clock} must settle to the top-row bar"
            );
            assert!(!settled.is_empty(), "settled bar must paint cells");
            // Top row: y == 0 across the full pane width.
            assert_eq!(
                settled.iter().map(|&(_, y)| y).collect::<Vec<_>>(),
                vec![0; 120],
                "settled bar must sit on the pane's top row: {settled:?}"
            );
        }
        assert!(flash_cells(Rect::new(0, 0, 0, 10), 16).is_empty());
        assert!(flash_cells(Rect::new(0, 0, 10, 0), 16).is_empty());
    }

    #[test]
    fn strip_is_one_row_high_and_full_width() {
        for clock in (0..FLASH_MS).step_by(16) {
            let cells = flash_cells(pane(), clock);
            assert!(!cells.is_empty(), "scan row must paint cells");
            let mut rows: Vec<u16> = cells.iter().map(|&(_, y)| y).collect();
            rows.sort();
            rows.dedup();
            assert_eq!(
                rows.len(),
                1,
                "row must be exactly 1 row high at clock {clock}"
            );
            let cols: Vec<u16> = cells.iter().map(|&(x, _)| x).collect();
            let min = *cols.iter().min().expect("non-empty");
            let max = *cols.iter().max().expect("non-empty");
            assert_eq!(min, 0, "row must start at the pane's left column");
            assert_eq!(
                max, 119,
                "row must reach the pane's right column at clock {clock}"
            );
        }
    }

    #[test]
    fn strip_sweeps_bottom_to_top() {
        // The row's y must be monotonically non-increasing (the row
        // moves upward, never downward) across the window.
        let start_y = flash_cells(pane(), 0)
            .iter()
            .map(|&(_, y)| y)
            .min()
            .expect("non-empty");
        assert_eq!(start_y, 22, "flash must start at the bottom edge");
        let mut prev = start_y;
        for clock in (16..FLASH_MS).step_by(16) {
            let y = flash_cells(pane(), clock)
                .iter()
                .map(|&(_, y2)| y2)
                .min()
                .expect("non-empty");
            assert!(
                y <= prev,
                "strip must sweep bottom-to-top: row y must not increase: {prev} -> {y} at clock {clock}"
            );
            prev = y;
        }
        let end_y = flash_cells(pane(), FLASH_MS - 1)
            .iter()
            .map(|&(_, y)| y)
            .min()
            .expect("non-empty");
        assert_eq!(end_y, 0, "flash must end at the top edge");
    }

    #[test]
    fn short_panes_stay_visible_and_inside() {
        // A 1-row pane: the row covers the pane's only row for the whole
        // window and stays inside.
        for clock in (0..=FLASH_MS).step_by(16) {
            let cells = flash_cells(Rect::new(0, 0, 3, 1), clock);
            assert_eq!(cells.len(), 3, "clock {clock}: {cells:?}");
            assert!(
                cells.iter().all(|&(_, y)| y == 0),
                "1-row pane must paint its only row at clock {clock}"
            );
        }
        // A 2-row pane: the row travels from the bottom row to the top
        // row and stays inside.
        for clock in (0..=FLASH_MS).step_by(16) {
            let cells = flash_cells(Rect::new(4, 2, 2, 2), clock);
            assert_eq!(
                cells.len(),
                2,
                "1 row across a 2-wide pane at clock {clock}: {cells:?}"
            );
            assert!(
                cells
                    .iter()
                    .all(|&(x, y)| (4..6).contains(&x) && (2..4).contains(&y)),
                "clock {clock}: {cells:?}"
            );
        }
        assert_eq!(
            flash_cells(Rect::new(4, 2, 2, 2), 0)
                .iter()
                .map(|&(_, y)| y)
                .collect::<Vec<_>>(),
            vec![3; 2],
            "2-row pane must start on its bottom row"
        );
        assert_eq!(
            flash_cells(Rect::new(4, 2, 2, 2), LAST_PAINT_MS)
                .iter()
                .map(|&(_, y)| y)
                .collect::<Vec<_>>(),
            vec![2; 2],
            "2-row pane must settle on its top row"
        );
    }
}
