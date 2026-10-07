//! MENU-004 — the mux's per-frame [`MenuSnapshot`] builder.
//!
//! Feature: spec/features/mux-surface-single-top-of-mux-menu-bar-per-pane-suppression-spinner-draw-tick.feature
//! Cards: R-LAYOUT, R-CHIPS, R-ZONEB.
//!
//! Zone B in the mux bar = the pane view labels (effective_panes()
//! order — the focused pane's label is active, R5) followed by the
//! GLOBAL session chips (RED CARD 3: `active_menu_session_ids` /
//! `chip_inputs` shared with the board bar — the chip list is GLOBAL,
//! not the pane window).

use crate::components::menu_bar::{MenuFocus, MenuSnapshot, ZoneBCell};
use crate::store::AgentViewStore;
use crate::views::multiplex::MultiplexLayout;
use crate::views::multiplex::MuxPaneKind;

/// The bar's pane-label for a mux pane kind (Zone B view labels).
/// `pub(crate)` so `menu_state::refresh_zone_b` shares the same labels
/// (single source of truth for the zone_b view cells).
pub(crate) fn pane_label(kind: MuxPaneKind) -> &'static str {
    match kind {
        MuxPaneKind::Board => "Board",
        MuxPaneKind::Agent => "Agent",
        MuxPaneKind::ChangedFiles => "Files",
        MuxPaneKind::Checkpoints => "Ckpts",
    }
}

/// The full per-frame snapshot the mux render path paints: the owned
/// `focus` / `open_menu` state (from `MultiplexLayout`) + the pane view
/// labels (R-ZONEB: non-agent panes only — agent panes contribute NO
/// view label, their sessions are the chips; focused pane active, R5)
/// + the GLOBAL session chips (RED CARD 3) + clock.
pub fn build_snapshot(
    layout: &MultiplexLayout,
    agent_store: &AgentViewStore,
    focus: Option<MenuFocus>,
    open_menu: Option<(usize, usize)>,
    clock_ms: u64,
) -> MenuSnapshot {
    let active = crate::views::board::menu_snapshot::active_menu_session_ids(agent_store);
    let (chips_zone_b, mut chips) =
        crate::views::board::menu_snapshot::zone_b_and_chips(agent_store, &active, clock_ms);
    // MENU-010 R1: the chip of the session rendered in the FOCUSED
    // Agent pane carries the selected-item highlight (agent-pane
    // parity with the single Agent view); when the focused pane is
    // NOT an agent pane (`focused_session_id` = None) no chip is
    // highlighted — the store's current session must NOT leak into
    // the mux bar (the "stale" chip the board rule forbids).
    if let Some(focused) = layout.focused_session_id() {
        crate::views::board::menu_snapshot::mark_active_chip(&mut chips, &active, &focused);
    }
    let focus_idx = layout.focus();
    let zone_b = layout
        .effective_panes()
        .iter()
        .enumerate()
        .filter(|(_, kind)| **kind != MuxPaneKind::Agent)
        .map(|(i, kind)| ZoneBCell::View {
            label: pane_label(*kind),
            active: i == focus_idx,
        })
        .chain(chips_zone_b)
        .collect();
    MenuSnapshot {
        // The mux bar carries the full MenuCategories registry.
        zone_a: crate::components::menu_bar::items::CATEGORIES,
        focus,
        open_menu,
        zone_b,
        chips,
        // MENU-009 Q1: NO Zone C on the mux top bar (the user's
        // "board view and agent view" directive) — the empty slice
        // keeps the ring walk / layout / paint byte-identical to
        // pre-MENU-009 (scenario 13, the regression guard).
        zone_c: &[],
        clock_ms,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn zone_b_is_pane_labels_then_global_chips() {
        // @step Given a MultiplexLayout with [Board | Files] panes and the Board focused
        let mut layout = MultiplexLayout::new();
        layout.enable_default();
        layout.set_pane_list(vec![MuxPaneKind::Board, MuxPaneKind::ChangedFiles], None);
        layout.set_focus(0);
        let store = AgentViewStore::default();
        // @step When build_snapshot is called with one open session
        let snap = build_snapshot(&layout, &store, None, None, 0);
        // @step Then Zone B is [View(Board, active), View(Files)] with no chips
        assert_eq!(
            snap.zone_b,
            vec![
                ZoneBCell::View {
                    label: "Board",
                    active: true
                },
                ZoneBCell::View {
                    label: "Files",
                    active: false
                }
            ]
        );
    }
}
