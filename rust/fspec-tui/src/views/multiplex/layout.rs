//! MUX-001/003/BUG-166 — pure mux split math: orientation + percentage
//! scale → pane `Rect`s.
//!
//! Feature files:
//!   - spec/features/rust-mux-mode.feature
//!   - spec/features/equal-division-pane-splits-no-minimums-with-live-resize.feature
//!   - spec/features/mux-dividers-percentage-scale.feature
//!
//! MUX-003 (supersedes MUX-001 R5): panes divide the terminal EQUALLY by
//! pane count — no per-pane minimum clamps; an explicit split percent is
//! honored as-is.
//!
//! BUG-166 (percentage-scale model): `splits` holds ONE percentage per
//! inter-pane gap — n panes → n-1 entries, `splits[i]` = pane i's share of
//! the available axis (after divider subtraction) in percent. The LAST
//! pane is never an entry: it absorbs the integer remainder so the scale
//! always sums to 100. When a scale entry is missing (legacy/short
//! configs), the pane falls back to its equal share (`available/n`).
//! Every inter-pane gap gets its own 1-col/row divider
//! ([`divider_rects`]), and the live drag override targets a specific
//! divider index (`drag` + `drag_width` in
//! [`calculate_pane_rects_with_override`]).

use ratatui::layout::Rect;

use super::splits::{distribute, is_equal_scale};
use super::{MuxOrientation, MuxPaneKind};

/// Divider thickness (1 col or 1 row).
pub const DIVIDER_SIZE: u16 = 1;

/// The 1-col/row divider rects — one per inter-pane gap, immediately
/// after each of the first `n-1` panes (shared by the render pass and the
/// live recompute — every divider is painted and hit-tested at these
/// positions).
pub fn divider_rects(orientation: MuxOrientation, panes: &[Rect], body: Rect) -> Vec<Rect> {
    panes
        .iter()
        .take(panes.len().saturating_sub(1))
        .map(|pane| match orientation {
            MuxOrientation::Horizontal => Rect {
                x: pane.x + pane.width,
                y: body.y,
                width: DIVIDER_SIZE,
                height: body.height,
            },
            MuxOrientation::Vertical => Rect {
                x: body.x,
                y: pane.y + pane.height,
                width: body.width,
                height: DIVIDER_SIZE,
            },
        })
        .collect()
}

/// Compute absolute pane `Rect`s for `area` given orientation, pane
/// kinds and the percentage scale (`splits` has ≤ n-1 entries;
/// `splits[i]` = pane i's percent of the available axis; panes without
/// an entry take an equal share `available/n`; the last pane takes the
/// remainder).
///
/// Guarantees:
/// - panes are contiguous, non-overlapping, each ≥ 1 cell on the split
///   axis (never empty, even on tiny terminals);
/// - panes divide the area equally unless the scale says otherwise;
/// - the last pane absorbs the integer-division remainder.
pub fn calculate_pane_rects(
    area: Rect,
    orientation: MuxOrientation,
    panes: &[MuxPaneKind],
    splits: &[u16],
) -> Vec<Rect> {
    calculate_pane_rects_with_override(area, orientation, panes, splits, None, None)
}

/// Same as [`calculate_pane_rects`] but with an optional live override
/// for the pane BEFORE divider `drag` (divider drag tracking — the drag
/// follows the cursor; on release the stored percent re-applies).
pub fn calculate_pane_rects_with_override(
    area: Rect,
    orientation: MuxOrientation,
    panes: &[MuxPaneKind],
    splits: &[u16],
    drag: Option<usize>,
    drag_width: Option<u16>,
) -> Vec<Rect> {
    let n = panes.len();
    if n == 0 || area.width == 0 || area.height == 0 {
        return Vec::new();
    }
    let dividers = (n - 1) as u16 * DIVIDER_SIZE;
    let axis = match orientation {
        MuxOrientation::Horizontal => area.width,
        MuxOrientation::Vertical => area.height,
    };
    let available = axis.saturating_sub(dividers);
    let sizes = split_sizes(available, splits, n, drag, drag_width);

    let mut rects = Vec::with_capacity(n);
    let mut cursor = if orientation == MuxOrientation::Horizontal {
        area.x
    } else {
        area.y
    };
    for size in sizes.iter() {
        let rect = match orientation {
            MuxOrientation::Horizontal => Rect {
                x: cursor,
                y: area.y,
                width: *size,
                height: area.height,
            },
            MuxOrientation::Vertical => Rect {
                x: area.x,
                y: cursor,
                width: area.width,
                height: *size,
            },
        };
        rects.push(rect);
        cursor += *size + DIVIDER_SIZE;
    }
    rects
}

/// Split `available` cells across `n` panes (BUG-166 percentage scale):
/// `splits[i]` percent for pane i (equal share `available/n` when the
/// entry is missing); an EQUAL scale (all shares within 1%) renders the
/// exact equal division so default grids stay jitter-free; during a
/// divider drag the dragged pane tracks the cursor, the panes to its
/// LEFT keep their scale sizes, and the panes to its RIGHT absorb the
/// change (split evenly); the LAST pane always takes the remainder.
/// Returns `n` sizes, each ≥ 1, summing to `available` (when
/// `available ≥ n`; otherwise a best-effort 1-cell floor).
fn split_sizes(
    available: u16,
    splits: &[u16],
    n: usize,
    drag: Option<usize>,
    drag_width: Option<u16>,
) -> Vec<u16> {
    if available == 0 {
        return vec![0; n];
    }
    // Not enough room for every pane — degrade to a 1-cell-per-pane
    // floor (never zero, never a panic).
    if available < n as u16 {
        let mut sizes = vec![1u16; n];
        let mut leftover = available;
        for s in sizes.iter_mut() {
            *s = (leftover > 0) as u16;
            leftover = leftover.saturating_sub(1);
        }
        return sizes;
    }

    let drag = drag
        .zip(drag_width)
        .and_then(|(idx, w)| (idx < n - 1).then_some((idx, w)));
    if let Some((d, w)) = drag {
        // Live drag: the dragged pane tracks the cursor; panes to its
        // left keep their scale sizes, the panes to its right split the
        // remaining cells evenly (the stored scale re-applies on
        // release — BUG-166).
        let cw = w.clamp(1, available.saturating_sub((n - 1) as u16).max(1));
        let equal = is_equal_scale(splits, n);
        let mut sizes: Vec<u16> = Vec::with_capacity(n);
        let mut taken: u16 = 0;
        for i in 0..d {
            let raw = if equal {
                available / n as u16
            } else {
                match splits.get(i) {
                    Some(pct) => (available as u32 * (*pct).max(1) as u32 / 100) as u16,
                    None => available / n as u16,
                }
            };
            let size = {
                let headroom = available
                    .saturating_sub(taken)
                    .saturating_sub(n as u16 - i as u16);
                raw.min(headroom.max(1))
            };
            sizes.push(size);
            taken += size;
        }
        let cw = cw.min(
            available
                .saturating_sub(taken)
                .saturating_sub(n as u16 - d as u16)
                .max(1),
        );
        sizes.push(cw);
        taken += cw;
        let rest = available.saturating_sub(taken);
        let after = distribute(rest, &vec![1u32; n - 1 - d]);
        sizes.extend(after);
        return sizes;
    }

    // BUG-166: an equal scale (the default, or one rescaled by the
    // pane-count change) renders the EXACT equal division — flooring
    // each 33%-style entry would render 38/38/42 instead of 39/39/40.
    let equal = is_equal_scale(splits, n);
    let mut sizes: Vec<u16> = Vec::with_capacity(n);
    let mut taken: u16 = 0;
    for i in 0..n {
        let raw = if equal {
            available / n as u16
        } else {
            match splits.get(i) {
                Some(pct) => (available as u32 * (*pct).max(1) as u32 / 100) as u16,
                None => available / n as u16, // legacy short scale
            }
        };
        let size = if i == n - 1 {
            // Last pane absorbs the remainder (no minimum clamp).
            (available - taken).max(1)
        } else {
            // Capped so the remaining panes keep ≥ 1 cell each.
            let headroom = available - taken - (n - 1 - i) as u16;
            raw.min(headroom.max(1))
        };
        sizes.push(size);
        taken += size;
    }

    sizes
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn area(w: u16, h: u16) -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: w,
            height: h,
        }
    }

    #[test]
    fn two_panes_50_50_horizontal() {
        let panes = [MuxPaneKind::Board, MuxPaneKind::Agent];
        let rects = calculate_pane_rects(area(120, 24), MuxOrientation::Horizontal, &panes, &[50]);
        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].x, 0);
        assert_eq!(rects[1].x, rects[0].x + rects[0].width + 1);
        assert_eq!(rects[0].width + rects[1].width + 1, 120);
    }

    #[test]
    fn explicit_split_percent_is_honored_without_minimum_clamps() {
        // MUX-003: a 10% split is honored as-is (no board minimum).
        let panes = [MuxPaneKind::Board, MuxPaneKind::Agent];
        let rects = calculate_pane_rects(area(110, 24), MuxOrientation::Horizontal, &panes, &[10]);
        assert_eq!(rects[0].width, 109 * 10 / 100);
        assert_eq!(rects[1].width, 110 - rects[0].width - 1);
    }

    #[test]
    fn scale_entries_drive_their_own_panes_and_the_last_pane_takes_the_remainder() {
        // BUG-166: 3 panes, scale [40, 30] → 40% / 30% / 30% remainder.
        let panes = [MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent];
        let rects =
            calculate_pane_rects(area(100, 24), MuxOrientation::Horizontal, &panes, &[40, 30]);
        assert_eq!(rects[0].width, 98 * 40 / 100);
        assert_eq!(rects[1].width, 98 * 30 / 100);
        assert_eq!(rects[2].width, 100 - rects[0].width - rects[1].width - 2);
    }

    #[test]
    fn missing_scale_entries_fall_back_to_the_equal_share() {
        // BUG-166: legacy 1-entry scale on a 3-pane grid → pane 1 takes
        // its equal share.
        let panes = [MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent];
        let rects = calculate_pane_rects(area(100, 24), MuxOrientation::Horizontal, &panes, &[40]);
        let available = 98u16;
        assert_eq!(rects[0].width, available * 40 / 100);
        assert_eq!(rects[1].width, available / 3);
        assert_eq!(rects[2].width, 100 - rects[0].width - rects[1].width - 2);
    }

    #[test]
    fn live_drag_override_targets_the_dragged_divider_pane() {
        // BUG-166: dragging divider 1 (between panes 1 and 2) resizes
        // pane 1 only.
        let panes = [MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent];
        let base =
            calculate_pane_rects(area(120, 24), MuxOrientation::Horizontal, &panes, &[33, 33]);
        let dragged = calculate_pane_rects_with_override(
            area(120, 24),
            MuxOrientation::Horizontal,
            &panes,
            &[33, 33],
            Some(1),
            Some(80),
        );
        assert_eq!(dragged[0].width, base[0].width, "pane 0 untouched");
        assert!(
            dragged[1].width > base[1].width,
            "pane 1 grows when its divider is dragged right"
        );
        assert!(
            dragged[2].width < base[2].width,
            "pane 2 shrinks (it absorbs the remainder)"
        );
    }

    #[test]
    fn divider_rects_are_one_per_inter_pane_gap() {
        // BUG-166: 4 panes → 3 dividers, each 1 col wide, each
        // immediately right of its left pane.
        let panes = [
            MuxPaneKind::Board,
            MuxPaneKind::Agent,
            MuxPaneKind::ChangedFiles,
            MuxPaneKind::Checkpoints,
        ];
        let rects = calculate_pane_rects(
            area(200, 24),
            MuxOrientation::Horizontal,
            &panes,
            &[25, 25, 25],
        );
        let dividers = divider_rects(MuxOrientation::Horizontal, &rects, area(200, 24));
        assert_eq!(dividers.len(), 3);
        for i in 0..3 {
            assert_eq!(dividers[i].x, rects[i].x + rects[i].width);
            assert_eq!(dividers[i].width, 1);
            assert_eq!(rects[i + 1].x, dividers[i].x + 1);
        }
        // Vertical: full-width row dividers.
        let vpanes = [MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent];
        let vrects =
            calculate_pane_rects(area(120, 40), MuxOrientation::Vertical, &vpanes, &[33, 33]);
        let vdividers = divider_rects(MuxOrientation::Vertical, &vrects, area(120, 40));
        assert_eq!(vdividers.len(), 2);
        for i in 0..2 {
            assert_eq!(vdividers[i].y, vrects[i].y + vrects[i].height);
            assert_eq!(vdividers[i].height, 1);
            assert_eq!(vdividers[i].width, 120);
        }
    }

    #[test]
    fn vertical_panes_stack() {
        let panes = [MuxPaneKind::Board, MuxPaneKind::Agent];
        let rects = calculate_pane_rects(area(120, 40), MuxOrientation::Vertical, &panes, &[50]);
        assert_eq!(rects[0].y, 0);
        assert!(rects[1].y > rects[0].y);
        assert_eq!(rects[0].width, 120);
    }

    #[test]
    fn tiny_terminal_never_zero() {
        let panes = [MuxPaneKind::Board, MuxPaneKind::Agent];
        let rects = calculate_pane_rects(area(10, 24), MuxOrientation::Horizontal, &panes, &[50]);
        assert_eq!(rects.len(), 2);
        assert!(rects[0].width >= 1 && rects[1].width >= 1);
    }

    #[test]
    fn empty_panes_yield_no_rects() {
        let rects = calculate_pane_rects(area(120, 24), MuxOrientation::Horizontal, &[], &[50]);
        assert!(rects.is_empty());
    }
}
