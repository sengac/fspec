//! BUG-183 — live mux pane close (Esc on the Files / Checkpoints pane).
//!
//! Feature: spec/features/mux-pane-esc-close.feature
//!
//! Esc on the focused lazy mux pane closes THAT PANE from the LIVE
//! rendered grid only — the saved `tui.mux` layout string (MUX-010) is
//! untouched, same
//! transience as the agent slot dropping on session close, so `/mux off`
//! then `/mux on` restores the full layout verbatim. The close is
//! recorded in `closed_panes` so the next `sync_window` / render
//! re-derivation keeps the pane out of the live list (the transient
//! rule); `closed_panes` clears on every (re)layout.
//!
//! Weight rule (MUX-010): the stored weights are NOT changed by a live
//! close — the hidden pane takes no space and the visible items of its
//! group share the freed space by their weights; a group left with no
//! visible pane disappears.

use super::MultiplexLayout;

impl MultiplexLayout {
    /// BUG-183 R1: close the FOCUSED rendered pane (the pane under the
    /// user's cursor — a transient floor may have injected a duplicate
    /// kind earlier in the list, but only the focused occurrence gets
    /// dismissed) and re-derive the live layout. Returns
    /// `true` when at least one pane survived (the mux stays active —
    /// even a lone full-width Board pane, per the user decision: no
    /// auto-exit in that case), `false` when the close left NO rendered
    /// panes (R3 — the caller exits mux to the pre-mux view).
    ///
    /// The stored `config.layout` is UNTOUCHED
    /// (transient — the `/mux off` → `/mux on` cycle restores the full
    /// layout verbatim); only the live `rendered_panes` / focus /
    /// cached rects change.
    pub fn close_pane(&mut self) -> bool {
        let focus = self.focus;
        let Some(&kind) = self.rendered_panes.get(focus) else {
            return true;
        };
        // Close the FOCUSED rendered pane — the spec's "remove the
        // occurrence" targets the pane the user dismissed (the focused
        // one); a transient floor may have injected a duplicate kind
        // earlier in the list, but only the pane under the cursor is
        // dismissed.
        let idx = focus;
        // Record the close LIVE-ONLY: the stored layout keeps the pane,
        // `recompute_effective_panes` filters it out of the next
        // rendered derivation (the agent-slot transience rule).
        self.closed_panes.push(kind);
        if !self.transient_board {
            let leaf = self
                .visible
                .iter()
                .enumerate()
                .filter(|(_, shown)| **shown)
                .nth(idx)
                .map(|(leaf, _)| leaf);
            if let Some(leaf) = leaf {
                self.visible[leaf] = false;
            }
        }
        self.rendered_panes.remove(idx);
        let m = self.rendered_panes.len();
        if m == 0 {
            // R3: nothing left to render — drop the flash and let the
            // caller exit mux (the pre-mux view fallback = Board).
            self.disarm_flash();
            return false;
        }
        // Focus clamps to a surviving pane (the same clamp the window
        // re-derivation uses — `bump_focus` re-arms the flash iff the
        // focus actually moved). The surviving panes of the group share
        // the freed space by their stored weights.
        self.bump_focus(self.focus.min(m - 1));
        self.recompute_rects();
        true
    }

    /// BUG-183: re-entry (`/mux on`, or committing a layout from the
    /// config dialog) restores the SAVED layout verbatim — the
    /// live-only closed-pane set is cleared so the panes the user
    /// Esc-closed come back (the transient rule, same as the agent slot
    /// dropping on session close).
    pub fn clear_closed_panes(&mut self) {
        self.closed_panes.clear();
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::super::*;

    fn layout(text: &str) -> MultiplexLayout {
        let mut m = MultiplexLayout::new();
        m.set_layout(MuxLayout::parse(text).unwrap());
        m
    }

    #[test]
    fn close_middle_pane_removes_it_live_and_keeps_the_saved_layout() {
        let mut m = layout("board, agent, files, checkpoints");
        m.set_focus(2); // Files pane (middle)
        assert!(m.close_pane(), "three panes survive — the mux stays active");
        assert_eq!(
            m.rendered_panes,
            vec![
                MuxPaneKind::Board,
                MuxPaneKind::Agent,
                MuxPaneKind::Checkpoints
            ],
            "the closed pane is gone from the live grid"
        );
        assert!(
            m.closed_panes.contains(&MuxPaneKind::ChangedFiles),
            "the close is recorded live-only (transient filter)"
        );
        assert_eq!(
            m.config.layout.to_string(),
            "board, agent, files, checkpoints",
            "the saved tui.mux layout is untouched"
        );
        assert_eq!(m.focus, 2, "focus stays on a surviving pane");
        assert_eq!(
            m.rendered_panes.get(m.focus),
            Some(&MuxPaneKind::Checkpoints),
            "the surviving pane that slid into the freed slot is now focused"
        );
        assert_eq!(m.pane_rects().len(), 3, "the rect cache rescales");
        let rects = m.pane_rects();
        assert!(rects.iter().all(|r| r.width > 0));
        assert_eq!(rects.iter().map(|r| r.x + r.width).max().unwrap_or(0), 120);
    }

    #[test]
    fn closing_a_pane_in_a_group_lets_its_sibling_fill_the_group() {
        let mut m = layout("board, [agent, files]");
        m.set_focus(2);
        assert!(m.close_pane());
        assert_eq!(
            m.rendered_panes,
            vec![MuxPaneKind::Board, MuxPaneKind::Agent]
        );
        assert_eq!(m.pane_rects()[1].height, m.body_area().height);
        assert_eq!(m.config.layout.to_string(), "board, [agent, files]");
    }

    #[test]
    fn closing_the_last_rendered_pane_reports_no_survivors() {
        let mut m = layout("checkpoints");
        m.set_focus(0); // the focused (and only) rendered pane
        assert!(
            !m.close_pane(),
            "no panes survive — the caller must exit mux"
        );
        assert!(m.rendered_panes.is_empty());
        assert!(m.flash_pane().is_none(), "the flash is dropped on exit");
    }

    #[test]
    fn close_pane_is_a_no_op_when_no_pane_is_rendered() {
        let mut m = MultiplexLayout::new();
        assert!(
            m.close_pane(),
            "an empty live grid is not a last-pane close"
        );
    }

    #[test]
    fn clearing_closed_panes_restores_the_pane_on_the_next_derivation() {
        let mut m = layout("board, files");
        m.set_focus(1);
        m.close_pane();
        assert_eq!(m.rendered_panes, vec![MuxPaneKind::Board]);
        m.clear_closed_panes();
        m.sync_window(&[]);
        assert_eq!(
            m.rendered_panes,
            vec![MuxPaneKind::Board, MuxPaneKind::ChangedFiles],
            "the re-entry re-derives the closed pane from the saved layout"
        );
    }
}
