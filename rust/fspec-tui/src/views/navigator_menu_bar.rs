//! MENU-004 R-TICK — the "menu bar animating" draw-tick gate.
//!
//! Feature: spec/features/mux-surface-single-top-of-mux-menu-bar-per-pane-suppression-spinner-draw-tick.feature
//! Card: R-TICK.
//!
//! `Navigator::is_menu_bar_animating()` is true iff the ACTIVE surface
//! paints a menu bar (Board, Agent or Mux view) AND any open session is
//! Running or Compacting. The run loop ORs it into `tick_should_draw`
//! (new 6th operand) so the 16ms tick keeps redrawing the bar and the
//! Running chip's braille frame advances even when the user sits still
//! (the BUG-194 class of frozen spinner).
//!
//! The flag is CACHED by `Navigator::render_with_stores` (the only
//! point that holds an `&mut AgentViewStore` — the busy scan) and read
//! here without a store argument, exactly like `is_mux_flash_active`.
//!
//! Extracted from `navigator.rs` so that file stays under the 300-LoC
//! ceiling pinned by the source-shape cards.

use super::navigator::Navigator;

impl Navigator {
    /// MENU-004 R-TICK: true iff the active surface painted a menu bar
    /// on the last rendered frame AND any open session is
    /// Running/Compacting (BUG-194 parity — the scan is over ALL open
    /// sessions, not just the focused one). Cached by
    /// `render_with_stores`; the run loop feeds this into the 6th
    /// `tick_should_draw` operand.
    pub fn is_menu_bar_animating(&self) -> bool {
        self.menu_bar_animating
    }

    /// MENU-004 R-TICK: recompute the cached gate from the LAST
    /// rendered frame. Called by `render_with_stores` AFTER the active
    /// child painted (so the surface's bar-geometry caches reflect this
    /// frame): the surface painted a bar (Board: cached
    /// `last_menu_bar_geometry`; Agent: `menu_state.bar_row()`; Mux:
    /// `mux.menu_bar_painted()`) AND any open session is
    /// Running/Compacting.
    pub(crate) fn recompute_menu_bar_gate(&mut self, agent_store: &crate::store::AgentViewStore) {
        let bar_painted = match self.active_view {
            crate::views::ViewMode::Board => self.board.last_menu_bar_geometry.borrow().is_some(),
            crate::views::ViewMode::Agent => self.agent.menu_state.bar_row().is_some(),
            crate::views::ViewMode::Mux => self.mux.menu_bar_painted(),
            _ => false,
        };
        self.menu_bar_animating = bar_painted && agent_store.any_session_busy();
    }
}
