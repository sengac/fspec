//! MUX-002/MUX-010/MUX-012 — Shift+arrow focus movement across nested
//! panes.
//!
//! Feature: spec/features/multiple-agent-panes-with-grouped-agent-view-cycling.feature +
//! spec/features/mux-nested-column-and-row-layouts-from-a-bracket-layout-string.feature
//!
//! MUX-012: Shift+Left/Right step to the previous/next pane in the layout
//! string's reading order (the order of `rendered_panes`: left to right,
//! depth first) and stop at both ends. They never rotate the agent window.
//! Shift+Right on a last agent pane prompts for a new agent. Shift+Up/Down
//! move spatially to the pane above/below (`layout_tree::spatial`).

use super::layout_tree::{neighbour, Direction};
use super::{MultiplexLayout, MuxPaneKind};

impl MultiplexLayout {
    /// The pane above (`down == false`) or below the focused pane, if any.
    fn vertical_neighbour(&self, down: bool) -> Option<usize> {
        let direction = if down { Direction::Down } else { Direction::Up };
        let panes = self.live_geometry(self.body_area()).panes;
        neighbour(&panes, self.focus, direction)
    }

    /// MUX-010: true iff a pane lies above (`down == false`) or below the
    /// focused pane — only then do Shift+Up/Down move the mux focus;
    /// otherwise they stay the focused pane's keys (agent prompt history).
    pub fn has_vertical_neighbour(&self, down: bool) -> bool {
        self.vertical_neighbour(down).is_some()
    }

    /// MUX-012: true iff Shift+Right on the last pane offers a new agent —
    /// the last pane is an agent, or the grid is the BUG-174 transient
    /// board standing in for the layout's empty agent slots.
    fn last_pane_offers_new_agent(&self) -> bool {
        if self.transient_board {
            return self.agent_slot_count() > 0;
        }
        self.rendered_panes.last() == Some(&MuxPaneKind::Agent)
    }

    /// Shift+Right (MUX-012): focus the next pane in layout order. On the
    /// last pane, request the new-agent prompt when that pane is an agent
    /// (MUX-006: a move re-arms the flash). Returns `true` when a
    /// new-agent prompt was requested.
    pub fn shift_right(&mut self) -> bool {
        if self.focus + 1 < self.rendered_panes.len() {
            self.bump_focus(self.focus + 1);
            return false;
        }
        if self.last_pane_offers_new_agent() {
            self.pending_new_agent = true;
            return true;
        }
        false
    }

    /// Shift+Left (MUX-012): focus the previous pane in layout order;
    /// the first pane stays focused (no wrap).
    pub fn shift_left(&mut self) {
        if self.focus > 0 {
            self.bump_focus(self.focus - 1);
        }
    }

    /// Shift+Up / Shift+Down: focus the pane above / below (no wrap).
    pub fn shift_vertical(&mut self, down: bool) {
        if let Some(next) = self.vertical_neighbour(down) {
            self.bump_focus(next);
        }
    }
}
