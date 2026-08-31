//! MUX-001 — live pane-rect recomputation (extracted from `mod.rs` to
//! keep it under the 300-LoC ceiling).
//!
//! Feature: spec/features/rust-mux-mode.feature
//!
//! `recompute_rects` derives the body area from the last observed pane
//! rects and re-derives the pane + divider rects from the current
//! config (honoring the live `drag_width` override during a divider
//! drag).

use ratatui::layout::Rect;

use super::layout::{calculate_pane_rects_with_override, divider_rects};
use super::MultiplexLayout;

impl MultiplexLayout {
    /// Recompute the cached pane rects from the current config + the
    /// last observed body area (honoring the live `drag_width`
    /// override). MUX-002: only RENDERS the filled panes (agent slots
    /// beyond the open-session count are dropped); when `pane_rects`
    /// is empty, the stored `body_area` drives the recompute so rects
    /// exist before the first paint.
    pub(crate) fn recompute_rects(&mut self) {
        let body = if self.pane_rects.is_empty() {
            self.body_area.unwrap_or_default()
        } else {
            let first = self.pane_rects[0];
            match self.config.orientation {
                super::MuxOrientation::Horizontal => Rect {
                    x: first.x,
                    y: first.y,
                    width: self
                        .pane_rects
                        .iter()
                        .map(|r| r.x + r.width)
                        .max()
                        .unwrap_or(first.x + first.width)
                        - first.x,
                    height: first.height,
                },
                super::MuxOrientation::Vertical => Rect {
                    x: first.x,
                    y: first.y,
                    width: first.width,
                    height: self
                        .pane_rects
                        .iter()
                        .map(|r| r.y + r.height)
                        .max()
                        .unwrap_or(first.y + first.height)
                        - first.y,
                },
            }
        };
        let rects = calculate_pane_rects_with_override(
            body,
            self.config.orientation,
            self.effective_panes(),
            &self.config.splits,
            self.drag_index,
            self.drag_width,
        );
        self.pane_rects = rects.clone();
        self.divider_rects = divider_rects(self.config.orientation, &rects, body);
    }
}
