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

use super::menu_render::{AGENT_ZONE_A, AGENT_ZONE_C};
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
    /// MENU-009 R3/R5: true iff this view's OWN 2-zone bar painted its
    /// Zone C buttons this frame (the focused single-view pane,
    /// `menu_row=true`). The cached `zone_c_rects` is the PAINTED set
    /// — `None` when the pane has no bar row (the mux agent pane
    /// suppresses it → `clear_geometry`) and EMPTY when the row's
    /// width dropped the buttons (R5). Gating on the painted rects —
    /// not `bar_row` — keeps the ring honest: a truncated bar has no
    /// Zone C stops. The mux agent pane therefore walks a
    /// byte-identical pre-MENU-009 ring (the top-row MUX bar — with an
    /// empty Zone C — is the bar on screen there, walked by
    /// `App::dispatch_menu_mux`).
    fn zone_c_active(&self) -> bool {
        self.menu_state
            .zone_c_rects()
            .is_some_and(|rects| !rects.is_empty())
    }

    /// R4: walk the ring by `delta` (+1 right, -1 left), wrapping at
    /// both ends. The agent bar is dropdown-free (MENU-007) — a closed
    /// bar stays closed. MENU-009 R3: the ring order is
    /// item → chips → Zone C buttons → wrap (the buttons are the ring's
    /// LAST stops when painted).
    pub(crate) fn walk_menu_ring(&mut self, delta: i32) {
        let items = AGENT_ZONE_A.len();
        let chips = self.menu_state.chip_count();
        let zone_c = if self.zone_c_active() {
            AGENT_ZONE_C.len()
        } else {
            0
        };
        let len = items + chips + zone_c;
        if len == 0 {
            return;
        }
        let current = match self.menu_state.focus() {
            Some(MenuFocus::Item(i)) => i.min(items.saturating_sub(1)),
            Some(MenuFocus::ZoneB(j)) => items + j.min(chips.saturating_sub(1)),
            // MENU-009: a Zone C stop (only reachable when painted).
            Some(MenuFocus::ZoneC(i)) => items + chips + i.min(zone_c.saturating_sub(1)),
            None => return,
        };
        let next = (current as i32 + delta).rem_euclid(len as i32) as usize;
        let focus = if next < items {
            MenuFocus::Item(next)
        } else if next < items + chips {
            MenuFocus::ZoneB(next - items)
        } else {
            MenuFocus::ZoneC(next - items - chips)
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

    /// MENU-009 R2/R3: activate Zone C button `index` — clear the bar
    /// focus (the composer regains the keys, the chip/board-view
    /// activation parity) and emit `MenuZoneCActivate(index)` (the
    /// `App::dispatch_menu` arm resolves it per surface: BUG-199 —
    /// `New Agent` mounts the Create Session dialog, `Close Agent
    /// [esc]` runs the `AgentEscPressed` cascade).
    pub(crate) fn activate_menu_zone_c(&mut self, index: usize) {
        self.menu_state.clear_focus();
        self.emit(crate::components::Action::MenuZoneCActivate(index));
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
                // MENU-009 R2: a Zone C button activates its `MenuAction`
                // (New Agent / Close Agent) through the bus.
                MenuFocus::ZoneC(index) => {
                    self.activate_menu_zone_c(index);
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
