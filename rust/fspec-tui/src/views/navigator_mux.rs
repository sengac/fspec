//! Navigator — BUG-183 mux pane-close apply arms (extracted from
//! `navigator.rs` so that file stays under the 300-LoC ceiling pinned
//! by the source-shape cards).
//!
//! Feature: spec/features/mux-pane-esc-close.feature
//!
//! `pub(crate)` so the sibling `navigator` module can call them from
//! `Navigator::apply_action`.

use crate::components::Action;

use super::navigator::Navigator;

impl Navigator {
    /// MUX-001/BUG-175: `Action::BackToBoard` — when the mux grid is
    /// active, "back to board" focuses the board pane WITHIN the grid
    /// (never flip the whole view out of Mux). BUG-175: gate on the
    /// LIVE view, not the persisted `mux.config().enabled` flag (a
    /// saved layout preference that survives restarts — acting on it
    /// while the grid is not entered used to strand BackToBoard as a
    /// no-op).
    pub(crate) fn apply_back_to_board(&mut self) {
        if self.active_view == crate::views::ViewMode::Mux {
            let board_idx = self
                .mux
                .effective_panes()
                .iter()
                .position(|k| *k == crate::views::multiplex::MuxPaneKind::Board)
                .unwrap_or(0);
            self.mux.set_focus(board_idx);
        } else {
            self.active_view = crate::views::ViewMode::Board;
        }
    }

    /// BUG-183: mux pane-close — the focused Files / Checkpoints pane
    /// was dismissed with Esc. Fires ONLY in `ViewMode::Mux` (the lazy
    /// views' `Close` outcome is translated by the mux forwarder,
    /// never in single-view mode — R4: the Navigator's single-view
    /// `Close*View` arms keep their precedence). Removes the pane kind
    /// from the LIVE rendered list only (transient — the saved `tui.mux`
    /// config is untouched, so `/mux off` then `/mux on` restores the
    /// full layout) and rescales the surviving panes over the freed
    /// space. When the close leaves NO rendered panes (degenerate grid:
    /// all-agent layout + zero sessions + the one rendered lazy pane
    /// closed), exit `ViewMode::Mux` back to the pre-mux view (Board —
    /// the same rule `/mux off` uses).
    pub(crate) fn apply_mux_pane_close(&mut self, action: &Action) {
        let kind = if matches!(action, Action::CloseChangedFilesView) {
            crate::views::multiplex::MuxPaneKind::ChangedFiles
        } else {
            crate::views::multiplex::MuxPaneKind::Checkpoints
        };
        let focus = self.mux.focus();
        // Defensive guard: close ONLY the focused pane (the forwarder
        // emits the close for the focused kind only).
        if self.mux.effective_panes().get(focus) == Some(&kind) && !self.mux.close_pane() {
            // R3: the close left no rendered panes — exit mux to
            // the pre-mux view (Board fallback — the same rule
            // `/mux off` uses).
            self.active_view = self.mux.disable();
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use std::sync::Arc;

    use crate::components::Action;
    use crate::theme::Theme;
    use crate::views::multiplex::MuxPaneKind;
    use crate::views::navigator::ViewMode;
    use crate::views::Navigator;
    use tokio::sync::mpsc::unbounded_channel;

    fn fresh() -> (Navigator, tokio::sync::mpsc::UnboundedReceiver<Action>) {
        let (tx, rx) = unbounded_channel();
        (Navigator::new(Arc::new(Theme::default()), tx), rx)
    }

    /// A mux-enabled Navigator with the given rendered pane list and
    /// focus, active view Mux.
    fn mux_nav(panes: &[MuxPaneKind], focus: usize) -> Navigator {
        let (mut nav, _rx) = fresh();
        nav.mux.enable_default();
        nav.mux.set_pane_list(panes.to_vec(), None);
        nav.active_view = ViewMode::Mux;
        nav.mux.set_focus(focus);
        nav
    }

    // ── BUG-183: mux pane-close apply arms ─────────────────────────────

    /// Feature: spec/features/mux-pane-esc-close.feature
    /// Scenario: Esc on the focused Files mux pane closes the pane and keeps the saved layout
    #[test]
    fn close_changed_files_view_in_mux_closes_only_the_pane_and_keeps_the_saved_layout() {
        let mut nav = mux_nav(
            &[
                MuxPaneKind::Board,
                MuxPaneKind::Agent,
                MuxPaneKind::ChangedFiles,
                MuxPaneKind::Checkpoints,
            ],
            2, // Files pane
        );
        // @step When I press the Esc key
        nav.apply_action(&Action::CloseChangedFilesView);
        // @step Then the grid shows Board, Agent and Checkpoints (the Files pane is gone)
        assert_eq!(
            nav.mux.effective_panes(),
            &[
                MuxPaneKind::Board,
                MuxPaneKind::Agent,
                MuxPaneKind::Checkpoints
            ],
        );
        // @step And the saved mux layout still lists the Files pane so a later /mux off then /mux on restores it
        assert!(
            nav.mux.config().panes.contains(&MuxPaneKind::ChangedFiles),
            "the saved tui.mux pane list must be untouched"
        );
        // @step And the TUI is still in mux mode with a surviving pane focused
        assert_eq!(nav.active_view, ViewMode::Mux);
        assert_ne!(
            nav.mux.effective_panes()[nav.mux.focus()],
            MuxPaneKind::ChangedFiles
        );
    }

    /// Feature: spec/features/mux-pane-esc-close.feature
    /// Scenario: Esc on the focused Checkpoints mux pane closes the pane and keeps the saved layout
    #[test]
    fn close_checkpoints_view_in_mux_closes_only_the_pane_and_keeps_the_saved_layout() {
        let mut nav = mux_nav(
            &[
                MuxPaneKind::Board,
                MuxPaneKind::Agent,
                MuxPaneKind::Checkpoints,
            ],
            2, // Checkpoints pane
        );
        nav.apply_action(&Action::CloseCheckpointsView);
        assert_eq!(
            nav.mux.effective_panes(),
            &[MuxPaneKind::Board, MuxPaneKind::Agent],
        );
        assert!(
            nav.mux.config().panes.contains(&MuxPaneKind::Checkpoints),
            "the saved tui.mux pane list must be untouched"
        );
        assert_eq!(nav.active_view, ViewMode::Mux);
    }

    /// Feature: spec/features/mux-pane-esc-close.feature
    /// Scenario: closing the last rendered pane exits mux to the single Board view
    #[test]
    fn closing_the_last_rendered_pane_exits_mux_to_the_pre_mux_view() {
        let mut nav = mux_nav(&[MuxPaneKind::Checkpoints], 0);
        nav.apply_action(&Action::CloseCheckpointsView);
        assert_eq!(
            nav.active_view,
            ViewMode::Board,
            "the close must exit to the pre-mux view (Board)"
        );
        assert!(
            !nav.mux.config().enabled,
            "the mux config must be disabled on exit"
        );
        assert_eq!(
            nav.mux.config().panes,
            vec![MuxPaneKind::Checkpoints],
            "the saved tui.mux pane list must survive the exit"
        );
    }

    /// Feature: spec/features/mux-pane-esc-close.feature
    /// Scenario: Esc on a Files pane mid-initial-load does not close the pane
    /// (the view returns Ignored — NO `Close*View` action is emitted,
    /// so this arm never runs; guard the focused-pane check instead.)
    #[test]
    fn a_close_action_for_an_unfocused_pane_is_a_no_op() {
        let mut nav = mux_nav(
            &[MuxPaneKind::Board, MuxPaneKind::Checkpoints],
            0, // Board pane focused
        );
        nav.apply_action(&Action::CloseCheckpointsView);
        assert_eq!(
            nav.mux.effective_panes(),
            &[MuxPaneKind::Board, MuxPaneKind::Checkpoints],
            "the close must only fire for the FOCUSED pane"
        );
        assert_eq!(nav.active_view, ViewMode::Mux);
    }

    // ── R4: single-view Close*View arms keep their precedence ──────────

    /// Feature: spec/features/mux-pane-esc-close.feature
    /// Scenario: Esc in the single Changed Files view still closes to the Board
    #[test]
    fn close_changed_files_view_in_single_view_still_flips_to_board() {
        let (mut nav, _rx) = fresh();
        nav.active_view = ViewMode::ChangedFiles;
        nav.apply_action(&Action::CloseChangedFilesView);
        assert_eq!(
            nav.active_view,
            ViewMode::Board,
            "the single-view Esc-close must flip to Board (unchanged)"
        );
    }

    /// Single Checkpoints view: the whole view flips to Board.
    #[test]
    fn close_checkpoints_view_in_single_view_still_flips_to_board() {
        let (mut nav, _rx) = fresh();
        nav.active_view = ViewMode::Checkpoints;
        nav.apply_action(&Action::CloseCheckpointsView);
        assert_eq!(nav.active_view, ViewMode::Board);
    }
}
