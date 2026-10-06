//! MENU-002/MENU-003 — `App::dispatch` arms for the 2-zone menu bar
//! (`MenuMove`, `MenuFocusToColumns`, `MenuOpenDropdown`,
//! `MenuCloseDropdown`, `MenuDropdownCursor`, `MenuExecuteItem`,
//! `MenuChipActivate`, `MenuMoveToItem`).
//!
//! Feature: spec/features/board-surface-header-row-replaced-by-the-2-zone-bar-column-menu-chip-continuous-ring-keys-wheel-mouse.feature
//! Card: MENU-002 (R4, R6, R7, R8).
//!
//! Mirror of the MUX-001 `dispatch_mux` pattern: an `is_menu_action`
//! guard arm + a `dispatch_menu` body, so `dispatch.rs` stays under the
//! 300-LoC ceiling pinned by `rpc013-source-shape` /
//! `rpc024-source-shape`. The board store owns the board ring math
//! (R8); this module only routes.
//!
//! MENU-003 context-awareness: `MenuChipActivate` and `MenuExecuteItem`
//! resolve against the ACTIVE surface — the board store when the
//! Board view is active (MENU-002, unchanged), the agent view's local
//! `menu_state` when the Agent view is active (the agent view already
//! moved its focus locally / focused the chip's session through its
//! test seam, so the arm only closes the dropdown or re-emits the
//! executed entry).

use crate::components::menu_bar::items::{MenuAction, CATEGORIES};
use crate::components::Action;
use crate::views::ViewMode;

use super::state::App;

impl App {
    /// MENU-002 R8: feed the ring its chip count — the PAINTED chip
    /// list (open sessions minus Cleared, `menu_snapshot` parity) — so
    /// the store's column⇄item⇄chip walk stays in lockstep with the
    /// bar's paint. Called at the top of every `dispatch` tick.
    /// MENU-004 R8: the mux bar's fed ring (`menu_chips`) gets the same
    /// count in lockstep (the board `set_menu_ring_size` mirror) so the
    /// mux key-time ring math agrees with the bar's paint.
    pub(crate) fn feed_menu_ring_size(&mut self) {
        let chips =
            crate::views::board::menu_snapshot::active_menu_session_ids(&self.agent_view_store)
                .len();
        self.board_store.set_menu_ring_size(chips);
        self.navigator
            .mux
            .refresh_menubar_ring(&self.agent_view_store);
    }

    /// True iff `action` is a MENU-002 board menu-bar variant.
    pub(crate) fn is_menu_action(action: &Action) -> bool {
        matches!(
            action,
            Action::MenuMove(_)
                | Action::MenuFocusToColumns
                | Action::MenuOpenDropdown(_)
                | Action::MenuCloseDropdown
                | Action::MenuDismissBar
                | Action::MenuDropdownCursor(_)
                | Action::MenuExecuteItem { .. }
                | Action::MenuChipActivate(_)
                | Action::MenuMoveToItem(_)
                | Action::MenuFocusPane(_)
        )
    }

    /// MENU-004 R-STATE: the Mux-view half of `dispatch_menu` — the bar's
    /// ring/dropdown state lives on `navigator.mux` (RED CARD 1), so the
    /// arms mutate the mux layout instead of the board store. Executed
    /// rows re-dispatch onto the bus (R-EXEC); chip activation focuses the
    /// session WITHOUT flipping out of Mux (R-CHIPS).
    pub(crate) fn dispatch_menu_mux(&mut self, action: &Action) {
        let mux = &mut self.navigator.mux;
        match action {
            Action::MenuMove(delta) => {
                // R-ZONEB: the ring = items + Zone B (pane labels + the
                // GLOBAL chips). Walk it against a per-frame snapshot
                // (the dispatched walk uses the painted zone_b, mirroring
                // the board store's R8 `menu_move`).
                //
                // MENU-004 R-KEYS: the MUX bar owns the walk ONLY while
                // its ring is active (an item/cell has the focus or a
                // dropdown is open — entry into the bar is always the
                // direct `MenuMoveToItem(0)` edge rule, never a
                // `MenuMove`). When the mux bar is INACTIVE the focused
                // pane has the keys: a `MenuMove` then belongs to that
                // pane's OWN surface — the board pane's continuous
                // column⇄item⇄chip ring (MENU-002, the board store) —
                // and a walk landing on the board store's (suppressed)
                // bar segment keeps operating via the board pane's own
                // key arms, exactly as in the single Board view.
                if mux.menu_ring_active() {
                    let focus = mux.menu_focus();
                    let open = mux.open_menu();
                    let snapshot = crate::views::multiplex::menu_snapshot::build_snapshot(
                        mux,
                        &self.agent_view_store,
                        focus,
                        open,
                        crate::views::multiplex::menu_render::now_ms(),
                    );
                    let next = snapshot.advance(focus, *delta);
                    mux.menu_move_to_ring_position(next);
                    mux.menu_reanchor_or_close();
                } else if self.mux_board_pane_focused() {
                    self.board_store.menu_move(*delta);
                }
            }
            Action::MenuFocusToColumns => mux.menu_dismiss(),
            Action::MenuOpenDropdown(category) => mux.menu_open_or_close(*category),
            Action::MenuCloseDropdown => mux.menu_close_dropdown(),
            Action::MenuDismissBar => mux.menu_dismiss(),
            Action::MenuDropdownCursor(delta) => mux.menu_dropdown_cursor(*delta),
            Action::MenuMoveToItem(index) => mux.menu_move_to_item(*index),
            Action::MenuFocusPane(pane) => {
                // R-ZONEB: Enter/click on a pane view label focuses that
                // pane (set_focus) — no view flip, no session change.
                mux.set_focus(*pane);
            }
            Action::MenuChipActivate(index) => {
                // R-CHIPS: resolve against the GLOBAL painted chip list and
                // focus that session's open-session slot — but do NOT flip
                // out of Mux (the agent pane showing it stays put).
                let sessions = crate::views::board::menu_snapshot::active_menu_session_ids(
                    &self.agent_view_store,
                );
                let Some(session) = sessions.get(*index) else {
                    return;
                };
                if let Some(open_idx) = self
                    .agent_view_store
                    .open_sessions()
                    .iter()
                    .position(|c| &c.id == session)
                {
                    self.agent_view_store.focus_session_index(open_idx);
                }
                // Stay in Mux — no `active_view` flip, no navigation target.
            }
            Action::MenuExecuteItem { category, row } => {
                let Some(menu_action) = mux.menu_execute_item(*category, *row) else {
                    return;
                };
                // R-EXEC: the NewAgent row carries NO session payload —
                // substitute the live current-session snapshot at execute
                // time (MENU-002/003 parity).
                let target = if menu_action == MenuAction::NewAgent {
                    self.agent_view_store.current_session().cloned()
                } else {
                    None
                };
                self.dispatch(menu_action.to_action(target));
            }
            _ => {}
        }
    }

    /// Route a MENU-002/003 menu-bar action (store-level half).
    /// Executed actions are re-dispatched onto the bus so the existing
    /// variant arms (view flips, dialogs, session focus) run unchanged.
    pub(crate) fn dispatch_menu(&mut self, action: &Action) {
        // MENU-004 R-STATE: in the Mux view the bar's state lives on the
        // mux layout (RED CARD 1) — the arms mutate `navigator.mux`,
        // never the board store.
        if self.navigator.active_view == ViewMode::Mux {
            self.dispatch_menu_mux(action);
            return;
        }
        match action {
            Action::MenuMove(delta) => self.board_store.menu_move(*delta),
            Action::MenuFocusToColumns => self.board_store.menu_focus_to_columns(),
            Action::MenuOpenDropdown(category) => self.board_store.open_or_close_menu(*category),
            Action::MenuCloseDropdown => self.board_store.close_menu(),
            Action::MenuDismissBar => self.board_store.dismiss_menu(),
            Action::MenuDropdownCursor(delta) => {
                let entries = self
                    .board_store
                    .open_menu()
                    .map(|(cat, _)| CATEGORIES.get(cat))
                    .and_then(|c| c.map(|c| c.entries.len()))
                    .unwrap_or(0);
                self.board_store.move_dropdown_cursor(*delta, entries);
            }
            Action::MenuExecuteItem { category, row } => {
                // MENU-003: the Agent view executes against its LOCAL
                // state (the dropdown closed in-view; the entry is
                // re-emitted on the bus so the view flip runs).
                if self.navigator.active_view == ViewMode::Agent {
                    let Some(entry) = CATEGORIES.get(*category).and_then(|c| c.entries.get(*row))
                    else {
                        return;
                    };
                    self.navigator.agent.menu_state.set_open(None);
                    // The NewAgent row carries NO session payload in the
                    // registry — substitute the live current-session
                    // snapshot at execute time (R8).
                    let target = if entry.action == MenuAction::NewAgent {
                        self.agent_view_store.current_session().cloned()
                    } else {
                        None
                    };
                    self.dispatch(entry.action.to_action(target));
                    return;
                }
                let Some(menu_action) = self.board_store.execute_dropdown_row(*category, *row)
                else {
                    return;
                };
                // R8: the NewAgent row carries NO session payload in the
                // registry — substitute the live current-session snapshot
                // at execute time (BOARD-023 R5 semantics).
                let target = if menu_action == MenuAction::NewAgent {
                    self.agent_view_store.current_session().cloned()
                } else {
                    None
                };
                self.dispatch(menu_action.to_action(target));
            }
            Action::MenuChipActivate(index) => {
                // Resolve the chip against the PAINTED chip list (open
                // sessions with status != Cleared — R9 drop parity with
                // the bar's paint; see `menu_snapshot`).
                let sessions = crate::views::board::menu_snapshot::active_menu_session_ids(
                    &self.agent_view_store,
                );
                let Some(session) = sessions.get(*index) else {
                    return;
                };
                // The chip's position in the painted list → the
                // open-session slot (`open_sessions` order with Cleared
                // removed).
                let Some(open_idx) = self
                    .agent_view_store
                    .open_sessions()
                    .iter()
                    .position(|c| &c.id == session)
                else {
                    return;
                };
                if self.navigator.active_view == ViewMode::Agent {
                    // BUG-195: on the AGENT view the chip's session focus
                    // used to be assumed to have happened in-view through
                    // the view's `store_handle` test seam — `None` in
                    // production, so both the in-view focus AND the old
                    // early-return were no-ops (a chip click /
                    // Enter-on-chip never switched sessions). The agent
                    // view already cleared its bar focus in-view; run the
                    // RPC-024 session switch here (no view flip — the
                    // active view is already Agent).
                    self.switch_to_session_index(open_idx);
                } else {
                    // MENU-002 (board surface): focus the slot and flip
                    // to the Agent view on it.
                    self.agent_view_store.focus_session_index(open_idx);
                    self.agent_view_store
                        .set_navigation_target(Some(session.clone()));
                    self.navigator.active_view = ViewMode::Agent;
                    // BUG-197 R3: the chip activation leaves the board
                    // bar — drop the board's ring focus so returning to
                    // the board paints NO stale chip highlight (only
                    // the MENU-006 active-chip path may highlight).
                    self.board_store.dismiss_menu();
                }
            }
            Action::MenuMoveToItem(index) => self.board_store.set_menu_focus_item(*index),
            _ => {}
        }
    }
}
