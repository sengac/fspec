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
//!
//! BUG-200: the board↔bar ring edges (columns ⇄ items ⇄ chips) are
//! mirrored in the Mux `MenuMove` arm — `menu_move_mux` (the mux arms
//! live in `dispatch_menu_mux`).

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
    /// MENU-009 R3: the board store's Zone C stop count is fed in the
    /// same lockstep — `1` when the board's OWN bar is painted (the
    /// single Board view: the right-aligned `New Agent` button is the
    /// ring's last stop), `0` in Mux (the board pane's bar is
    /// suppressed, MENU-004 R-SUPPRESS — its ring walk has no Zone C
    /// stop, byte-identical to pre-MENU-009).
    pub(crate) fn feed_menu_ring_size(&mut self) {
        let chips =
            crate::views::board::menu_snapshot::active_menu_session_ids(&self.agent_view_store)
                .len();
        self.board_store.set_menu_ring_size(chips);
        self.board_store
            .set_menu_zone_c(if self.navigator.active_view == ViewMode::Mux {
                0
            } else {
                1
            });
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
                | Action::MenuZoneCActivate(_)
        )
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
            Action::MenuZoneCActivate(index) => self.dispatch_menu_zone_c(*index),
            _ => {}
        }
    }
}
