//! BUG-183 — live mux pane close (Esc on the Files / Checkpoints pane).
//!
//! Feature: spec/features/mux-pane-esc-close.feature
//!
//! Esc on the focused lazy mux pane closes THAT PANE from the LIVE
//! rendered grid only — the saved `tui.mux` config (persisted layout
//! preference: `config.panes` AND `config.splits`) is untouched, same
//! transience as the agent slot dropping on session close, so `/mux off`
//! then `/mux on` restores the full layout verbatim. The close is
//! recorded in `closed_panes` so the next `sync_window` / render
//! re-derivation keeps the pane out of the live list (the transient
//! rule); `closed_panes` clears on every (re)layout.
//!
//! Scale rule: the saved scale is NOT rescaled on a live close — the
//! layout math already renders the surviving panes over the full axis
//! with a (possibly stale) scale: the surviving panes keep their saved
//! shares and the new LAST pane's implicit share absorbs the freed
//! space (example 1: closing the middle Files pane makes the
//! Checkpoints pane absorb its share).

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
    /// The saved `config.panes` + `config.splits` are UNTOUCHED
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
        // Record the close LIVE-ONLY: `config.panes` keeps the pane,
        // `recompute_effective_panes` filters it out of the next
        // rendered derivation (the agent-slot transience rule).
        self.closed_panes.push(kind);
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
        // focus actually moved). The surviving panes rescale over the
        // freed space via the existing rect recompute (the layout math
        // renders the surviving panes against the unchanged saved
        // scale; the new last pane's implicit share absorbs the freed
        // space).
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

    /// A layout with the given config + rendered panes, focus 0.
    fn layout(panes: Vec<MuxPaneKind>, splits: Vec<u16>) -> MultiplexLayout {
        let mut m = MultiplexLayout::new();
        m.config.enabled = true;
        m.config.panes = panes.clone();
        m.config.splits = splits;
        m.rendered_panes = panes;
        m
    }

    #[test]
    fn close_middle_pane_removes_it_live_and_keeps_the_saved_layout() {
        let mut m = layout(
            vec![
                MuxPaneKind::Board,
                MuxPaneKind::Agent,
                MuxPaneKind::ChangedFiles,
                MuxPaneKind::Checkpoints,
            ],
            vec![25, 25, 25],
        );
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
            m.config.panes,
            vec![
                MuxPaneKind::Board,
                MuxPaneKind::Agent,
                MuxPaneKind::ChangedFiles,
                MuxPaneKind::Checkpoints
            ],
            "the saved tui.mux pane list is untouched"
        );
        assert_eq!(
            m.config.splits,
            vec![25, 25, 25],
            "the saved tui.mux scale is untouched (the layout math \
             renders the survivors against it; the new last pane's \
             implicit share absorbs the freed space)"
        );
        assert_eq!(m.focus, 2, "focus stays on a surviving pane");
        assert_eq!(
            m.rendered_panes.get(m.focus),
            Some(&MuxPaneKind::Checkpoints),
            "the surviving pane that slid into the freed slot is now focused"
        );
        assert_eq!(m.pane_rects().len(), 3, "the rect cache rescales");
        let rects = m.pane_rects();
        assert!(
            rects.iter().all(|r| r.width > 0),
            "every surviving pane keeps a non-zero width"
        );
        // The surviving panes + dividers still tile the full 120-wide
        // body: the Checkpoints pane absorbed the Files pane's share.
        assert_eq!(rects.iter().map(|r| r.x + r.width).max().unwrap_or(0), 120,);
    }

    #[test]
    fn close_trailing_pane_keeps_the_saved_layout_verbatim() {
        let mut m = layout(
            vec![
                MuxPaneKind::Board,
                MuxPaneKind::Agent,
                MuxPaneKind::Checkpoints,
            ],
            vec![33, 33],
        );
        m.set_focus(2); // Checkpoints pane (trailing)
        assert!(m.close_pane(), "two panes survive");
        assert_eq!(
            m.config.splits,
            vec![33, 33],
            "the saved scale survives the live close untouched"
        );
        assert_eq!(m.rendered_panes.len(), 2);
        assert_eq!(m.focus, 1, "focus clamps to index 1");
        assert_eq!(m.pane_rects().len(), 2);
    }

    #[test]
    fn closing_the_last_rendered_pane_reports_no_survivors() {
        let mut m = layout(vec![MuxPaneKind::Checkpoints], vec![]);
        m.set_focus(0); // the focused (and only) rendered pane
        assert!(
            !m.close_pane(),
            "no panes survive — the caller must exit mux"
        );
        assert!(m.rendered_panes.is_empty());
        assert!(
            m.flash_pane().is_none(),
            "the flash is dropped on exit (no flash with mux off)"
        );
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
        let mut m = layout(
            vec![MuxPaneKind::Board, MuxPaneKind::ChangedFiles],
            vec![50],
        );
        m.set_focus(1);
        m.close_pane();
        assert_eq!(m.rendered_panes, vec![MuxPaneKind::Board]);
        // `/mux off` → `/mux on` re-enters from the saved layout:
        m.clear_closed_panes();
        m.sync_window(&[]);
        assert_eq!(
            m.rendered_panes,
            vec![MuxPaneKind::Board, MuxPaneKind::ChangedFiles],
            "the re-entry re-derives the closed pane from the saved list"
        );
    }
}
