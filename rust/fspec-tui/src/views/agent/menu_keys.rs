//! MENU-003 + MENU-007 — the agent view's 2-zone menu bar + ring
//! keyboard arms.
//!
//! Feature: spec/features/agent-view-surface-2-zone-bar-row-under-session-header.feature
//! Card: MENU-003 (R3, R4, R5, R9, R10).
//!
//! MENU-007: the agent bar's Zone A is a SINGLE 'Board View' item
//! (no dropdowns — the agent view is dropdown-free). Gating, in order:
//!
//! 1. **Bar focus (item or chip)** — Esc clears the bar focus (R10:
//!    NOT the back-to-board cascade); Up/Down drop focus back into the
//!    input; Enter on the 'Board View' item activates it (returns to
//!    the Board view, MENU-007) / Enter on a chip activates it (R5);
//!    Left/Right walk the ring (R4); a bare character key clears the
//!    bar focus and is typed into the input on the SAME event (R9).
//! 2. **No bar focus** — bare Left on an EMPTY input enters the bar at
//!    Item(0) (R3). Bare Right never enters; everything else falls
//!    through untouched (`Unhandled`).
//!
//! The ring = 1 menu item + session chips only (no columns, unlike the
//! board). The chip count is the focused pane's last PAINTED chip
//! list (render-refreshed, R9 `Cleared`-drop parity). Unlike the
//! board, the walk is computed HERE (the agent view owns
//! `menu_state` as a plain field — the board's ring math lives in
//! `BoardStore` because the board paints through `&self`). No
//! `h`/`l`/`j`/`k` arms: those type into the composer.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::components::menu_bar::MenuFocus;
use crate::components::EventResult;

use super::menu_render::AGENT_ZONE_A;
use super::AgentView;

/// The outcome of a menu-bar key arm: `Some` = the arm claimed the
/// event (return it verbatim); `None` = not claimed (the caller
/// continues the SAME event down the input cascade — including the R9
/// character, whose bar focus was already cleared by the arm).
pub(crate) type MenuKeyOutcome = Option<EventResult>;

/// True iff a modifier-free key is pressed.
fn plain(code: KeyCode, key: &KeyEvent) -> bool {
    code == key.code && key.modifiers == KeyModifiers::NONE
}

fn bare_char(key: &KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char(_)) && key.modifiers == KeyModifiers::NONE
}

impl AgentView {
    /// The agent ring length: the single 'Board View' item + painted
    /// chips (MENU-007).
    fn menu_ring_len(&self) -> usize {
        AGENT_ZONE_A.len() + self.menu_state.chip_count()
    }

    /// R4: walk the ring by `delta` (+1 right, -1 left), wrapping at
    /// both ends. The agent bar is dropdown-free (MENU-007) — a closed
    /// bar stays closed.
    pub(crate) fn walk_menu_ring(&mut self, delta: i32) {
        let items = AGENT_ZONE_A.len();
        let len = self.menu_ring_len();
        if len == 0 {
            return;
        }
        let current = match self.menu_state.focus() {
            Some(MenuFocus::Item(i)) => i.min(items.saturating_sub(1)),
            Some(MenuFocus::ZoneB(j)) => {
                items + j.min(self.menu_state.chip_count().saturating_sub(1))
            }
            None => return,
        };
        let next = (current as i32 + delta).rem_euclid(len as i32) as usize;
        let focus = if next < items {
            MenuFocus::Item(next)
        } else {
            MenuFocus::ZoneB(next - items)
        };
        self.menu_state.set_focus(Some(focus));
        // MENU-007: the agent bar never opens a dropdown.
        self.menu_state.set_open(None);
    }

    /// MENU-007: activate the 'Board View' item — emit
    /// `Action::BackToBoard` (single-view flips to the Board view; an
    /// active mux grid keeps its existing BackToBoard behavior) and
    /// clear the bar focus.
    pub(crate) fn activate_board_view_item(&mut self) {
        self.menu_state.clear_focus();
        self.emit(crate::components::Action::BackToBoard);
    }

    /// R5: activate chip `j` — focus the chip's session (the store
    /// seam, when bound; in production `App::dispatch_menu` resolves
    /// the emitted action) and clear the bar focus.
    pub(crate) fn activate_menu_chip(&mut self, j: usize) {
        if let Some(mut store) = self.locked_store() {
            let session = crate::views::board::menu_snapshot::active_menu_session_ids(&store)
                .get(j)
                .cloned();
            if let Some(session) = session {
                let open_idx = store
                    .open_sessions()
                    .iter()
                    .position(|c| c.id == session)
                    .filter(|&idx| idx < store.open_sessions().len());
                if let Some(idx) = open_idx {
                    store.focus_session_index(idx);
                }
            }
        }
        self.menu_state.clear_focus();
        self.emit(crate::components::Action::MenuChipActivate(j));
    }

    /// The agent view's menu-bar key arms (see the module docs).
    pub(crate) fn handle_menu_key(&mut self, key: &KeyEvent) -> MenuKeyOutcome {
        // ── R3/R4/R5/R10: a menu item or chip has the bar focus ────────
        let Some(focus) = self.menu_state.focus() else {
            // R3: bare Left on an EMPTY input enters the bar at the
            // first item. Bare Right never enters; every other key
            // falls through untouched.
            if plain(KeyCode::Left, key) && self.input.is_empty() {
                self.menu_state.set_focus(Some(MenuFocus::Item(0)));
                return Some(EventResult::consumed());
            }
            return None;
        };

        if plain(KeyCode::Esc, key) {
            // R10: clear the bar focus (the input regains focus — the
            // ONE place Esc does NOT go back to the board while
            // bar-focused).
            self.menu_state.clear_focus();
            return Some(EventResult::consumed());
        }
        if plain(KeyCode::Up, key)
            || plain(KeyCode::Char('k'), key)
            || plain(KeyCode::Down, key)
            || plain(KeyCode::Char('j'), key)
        {
            // R10: Up/Down on a closed bar drop focus back into the
            // input.
            self.menu_state.clear_focus();
            return Some(EventResult::consumed());
        }
        if plain(KeyCode::Enter, key) {
            match focus {
                // MENU-007: the 'Board View' item is a plain
                // activation button — Enter returns to the Board view
                // (no dropdown).
                MenuFocus::Item(_) => self.activate_board_view_item(),
                MenuFocus::ZoneB(index) => {
                    self.activate_menu_chip(index);
                }
            }
            return Some(EventResult::consumed());
        }
        if plain(KeyCode::Left, key) {
            self.walk_menu_ring(-1);
            return Some(EventResult::consumed());
        }
        if plain(KeyCode::Right, key) {
            self.walk_menu_ring(1);
            return Some(EventResult::consumed());
        }
        if bare_char(key) {
            // R9: a printable character dismisses the bar AND types
            // into the input on the SAME event — the caller continues
            // the cascade after clearing the focus.
            self.menu_state.clear_focus();
            return None;
        }
        // Any other key while bar-focused is swallowed (the spec:
        // "handle the ring/dropdown and consume").
        Some(EventResult::consumed())
    }
}
