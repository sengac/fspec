//! MUX-001/MUX-010 — live pane visibility + pane-rect recomputation.
//!
//! Feature: spec/features/rust-mux-mode.feature +
//! spec/features/mux-nested-column-and-row-layouts-from-a-bracket-layout-string.feature
//!
//! The live grid draws the stored layout with some panes hidden (MUX-002
//! agent slots beyond the open sessions, BUG-183 Esc-closed panes); a
//! group with no shown pane disappears. When no stored pane is shown the
//! grid is a transient single board pane (BUG-174), never stored. During
//! a divider drag the in-flight re-weighted copy is drawn instead.

use ratatui::layout::Rect;

use super::layout_tree::{compute, Geometry, MuxLayout, MuxNode};
use super::{MultiplexLayout, MuxPaneKind};

impl MultiplexLayout {
    /// Show every stored pane (fresh entry / layout change — the session
    /// filter applies on the next `sync_window`).
    pub(super) fn show_all_panes(&mut self) {
        self.rendered_panes = self.config.layout.leaves();
        self.visible = vec![true; self.rendered_panes.len()];
        self.transient_board = false;
    }

    /// Re-derive which stored panes are shown from the open sessions and
    /// the closed panes (MUX-002 / BUG-174 / BUG-183).
    pub(super) fn refresh_visibility(&mut self) {
        let available = self.sessions.len();
        let mut agent_seen = 0usize;
        let leaves = self.config.layout.leaves();
        self.visible = leaves
            .iter()
            .map(|kind| match kind {
                MuxPaneKind::Agent => {
                    agent_seen += 1;
                    agent_seen <= available
                }
                MuxPaneKind::ChangedFiles | MuxPaneKind::Checkpoints => {
                    !self.closed_panes.contains(kind)
                }
                MuxPaneKind::Board => true,
            })
            .collect();
        self.transient_board = !self.visible.iter().any(|shown| *shown);
        self.rendered_panes = if self.transient_board {
            vec![MuxPaneKind::Board]
        } else {
            leaves
                .into_iter()
                .zip(&self.visible)
                .filter_map(|(kind, shown)| shown.then_some(kind))
                .collect()
        };
    }

    /// The geometry of the live grid in `body`.
    pub(super) fn live_geometry(&self, body: Rect) -> Geometry {
        if self.transient_board {
            let board = MuxLayout::with_first(MuxNode::pane(MuxPaneKind::Board), Vec::new());
            return compute(body, &board, &[]);
        }
        let layout = self.drag_layout.as_ref().unwrap_or(&self.config.layout);
        compute(body, layout, &self.visible)
    }

    /// Recompute the cached pane rects + dividers from the live layout
    /// and the last observed body area (rects exist before the first
    /// paint thanks to the seeded body).
    pub(crate) fn recompute_rects(&mut self) {
        let geometry = self.live_geometry(self.body_area());
        self.pane_rects = geometry.panes;
        self.dividers = geometry.dividers;
    }
}
