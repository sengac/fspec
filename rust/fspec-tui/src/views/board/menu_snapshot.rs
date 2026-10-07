//! MENU-002 — the board's per-frame [`MenuSnapshot`] builder.
//!
//! Feature: spec/features/board-surface-header-row-replaced-by-the-2-zone-bar-column-menu-chip-continuous-ring-keys-wheel-mouse.feature
//! Card: MENU-002 (R4, R9).
//!
//! Gathers the per-session rows from [`AgentViewStore`] into the
//! store-free [`MenuSnapshot`] the menu-bar painters read, and exposes
//! the PAINTED chip→session resolution so `MenuChipActivate` (dispatch)
//! and the chip-activation ring math agree. `Cleared` sessions are
//! dropped (R9) — the same rule the chip builder applies — so the ring
//! size, the painted cells, and the activation index stay in lockstep.

use codelet_rpc_types::{SessionId, SessionStatus};

use crate::components::menu_bar::items::MenuAction;
use crate::components::menu_bar::{
    build_chips, ChipInput, MenuChip, MenuSnapshot, ZoneBCell, ZoneCButton,
};
use crate::store::AgentViewStore;

/// MENU-009 R1: the board bar's Zone C — a single right-aligned
/// `New Agent` button (the `.`-key / Kanban-dropdown-row semantics,
/// R8 caller substitution at execute time). `&'static` so the shared
/// painter stays surface-agnostic (the `AGENT_ZONE_A` precedent).
pub const BOARD_ZONE_C: &[ZoneCButton] = &[ZoneCButton {
    label: "New Agent",
    action: MenuAction::NewAgent,
}];

/// The open sessions the bar PAINTS (R9: `Cleared` dropped; a session
/// with no observed status yet counts as active and paints `Idle`).
/// Order = `open_sessions` order; the index into this list is the
/// chip-activation / ring Zone B index.
pub fn active_menu_session_ids(store: &AgentViewStore) -> Vec<SessionId> {
    store
        .open_sessions()
        .iter()
        .filter(|c| match store.session_status_for(&c.id) {
            Some(status) => *status != SessionStatus::Cleared,
            None => true,
        })
        .map(|c| c.id.clone())
        .collect()
}

/// The per-session chip inputs for [`active_menu_session_ids`] —
/// `index` = the global 1-based slot, `status` = the live status
/// (defaulting to `Idle` when unobserved), `wu_id` = the bound work
/// unit id (chip suffix), `active` = `false` (MENU-010: the surface's
/// snapshot builder decides which chip, if any, carries the
/// selected-item highlight — the board never does, the agent view
/// marks the current session, the mux marks it only when an agent
/// pane is focused).
pub fn chip_inputs(store: &AgentViewStore, active: &[SessionId]) -> Vec<ChipInput> {
    active
        .iter()
        .map(|id| ChipInput {
            index: store.session_index_for(id),
            status: store
                .session_status_for(id)
                .copied()
                .unwrap_or(SessionStatus::Idle),
            wu_id: store.work_unit_context_for(id).map(|c| c.id.clone()),
            // MENU-010: the surface decides — always false here.
            active: false,
        })
        .collect()
}

/// MENU-010: mark the chip of `session` (when it sits in the painted
/// `active` list) as the selected-item highlight. No-op when the
/// session is not painted (closed / Cleared). The SURFACE's snapshot
/// builder decides whether/which session gets marked — the board
/// never does (R2), the agent view marks the store's current session
/// (R1), the mux marks the focused agent pane's window session (R1
/// agent-pane parity).
pub fn mark_active_chip(chips: &mut [MenuChip], active: &[SessionId], session: &SessionId) {
    if let Some(pos) = active.iter().position(|id| id == session) {
        if let Some(chip) = chips.get_mut(pos) {
            chip.active = true;
        }
    }
}

/// Build the Zone B display cells (one `Chip(i)` per painted session)
/// and the chip list (R9 drop applied by the builder) from `active`.
pub fn zone_b_and_chips(
    store: &AgentViewStore,
    active: &[SessionId],
    clock_ms: u64,
) -> (Vec<ZoneBCell>, Vec<MenuChip>) {
    let inputs = chip_inputs(store, active);
    let chips = build_chips(&inputs, clock_ms);
    let zone_b = active
        .iter()
        .enumerate()
        .map(|(i, _)| ZoneBCell::Chip(i))
        .collect();
    (zone_b, chips)
}

/// The full per-frame snapshot the board render path paints: the owned
/// `focus` / `open_menu` state (from `BoardStore`) + the freshly built
/// Zone B cells + chips + clock.
pub fn build_snapshot(
    store: &AgentViewStore,
    focus: Option<crate::components::menu_bar::MenuFocus>,
    open_menu: Option<(usize, usize)>,
    clock_ms: u64,
) -> MenuSnapshot {
    let active = active_menu_session_ids(store);
    let (zone_b, chips) = zone_b_and_chips(store, &active, clock_ms);
    MenuSnapshot {
        // The board bar carries the full MenuCategories registry.
        zone_a: crate::components::menu_bar::items::CATEGORIES,
        focus,
        open_menu,
        zone_b,
        chips,
        // MENU-009: the board's single right-aligned New Agent button.
        zone_c: BOARD_ZONE_C,
        clock_ms,
    }
}
