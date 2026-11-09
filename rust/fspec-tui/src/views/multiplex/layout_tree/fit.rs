//! MUX-010 — the minimum pane size a layout must fit (replaces the old
//! 2..4 pane-count limit).
//!
//! Feature: spec/features/mux-nested-column-and-row-layouts-from-a-bracket-layout-string.feature

use ratatui::layout::Rect;
use thiserror::Error;

use super::{compute, pane_name, MuxLayout};
use crate::views::multiplex::MuxPaneKind;

/// Every pane needs at least this many columns ...
pub const MIN_PANE_WIDTH: u16 = 16;
/// ... and this many rows.
pub const MIN_PANE_HEIGHT: u16 = 5;

/// A layout whose smallest pane is below the minimum at `body`.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error(
    "layout does not fit {body_width}x{body_height}: pane {pane} ({name}) would be \
     {width}x{height}; every pane needs at least {min_width}x{min_height}",
    name = pane_name(*kind),
    min_width = MIN_PANE_WIDTH,
    min_height = MIN_PANE_HEIGHT
)]
pub struct FitError {
    pub body_width: u16,
    pub body_height: u16,
    /// 1-based pane number in reading order.
    pub pane: usize,
    pub kind: MuxPaneKind,
    pub width: u16,
    pub height: u16,
}

/// Check that every pane of `layout` (all panes shown) is at least
/// [`MIN_PANE_WIDTH`] x [`MIN_PANE_HEIGHT`] in `body`; the first pane in
/// reading order that is too small is reported.
pub fn check_fit(body: Rect, layout: &MuxLayout) -> Result<(), FitError> {
    let geometry = compute(body, layout, &[]);
    let kinds = layout.leaves();
    for (index, kind) in kinds.iter().enumerate() {
        let rect = geometry.panes.get(index).copied().unwrap_or_default();
        if rect.width < MIN_PANE_WIDTH || rect.height < MIN_PANE_HEIGHT {
            return Err(FitError {
                body_width: body.width,
                body_height: body.height,
                pane: index + 1,
                kind: *kind,
                width: rect.width,
                height: rect.height,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn five_columns_do_not_fit_80_but_fit_120() {
        let layout = MuxLayout::parse("board, agent, files, checkpoints, board").unwrap();
        let err = check_fit(Rect::new(0, 1, 80, 22), &layout).unwrap_err();
        assert_eq!(
            err.to_string(),
            "layout does not fit 80x22: pane 1 (board) would be 15x22; \
             every pane needs at least 16x5"
        );
        assert!(check_fit(Rect::new(0, 1, 120, 38), &layout).is_ok());
    }

    #[test]
    fn deep_rows_hit_the_height_minimum() {
        let layout = MuxLayout::parse("[board, board, board, board, files]").unwrap();
        let err = check_fit(Rect::new(0, 0, 80, 22), &layout).unwrap_err();
        assert_eq!((err.pane, err.height), (1, 3));
    }
}
