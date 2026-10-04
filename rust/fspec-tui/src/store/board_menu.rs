//! MENU-002 — the board's column⇄menu⇄chip continuous-ring state + wrap math.
//!
//! Feature: spec/features/board-surface-header-row-replaced-by-the-2-zone-bar-column-menu-chip-continuous-ring-keys-wheel-mouse.feature
//! Card: MENU-002 (R2, R3, R4, R6, R8, R9).
//!
//! The board's kanban navigation becomes ONE continuous wrap-around ring (R2):
//! the 7 kanban columns, then the Zone A menu items, then the session chips
//! (Zone B), then back to the first column. Left/Right (h/l) and wheel
//! ScrollLeft/ScrollRight walk it; columns, items and chips each occupy
//! exactly one ring stop.
//!
//! Per R8 the store OWNS the ring position + wrap math (this module);
//! `App::dispatch` only feeds the chip count via `set_menu_ring_size`
//! (from `agent_view_store.open_sessions()`) so the store stays
//! session-order-agnostic. `BoardView` emits `Action::MenuMove` /
//! `MenuFocusToColumns` / `MenuOpenDropdown` / `MenuChipActivate` and
//! never computes positions itself.
//!
//! Ring position mapping (0-based, ring order, length = 7 + items + chips):
//!   0..7               → a column (`menu_focus` None, `focused_column` = pos)
//!   7..7+items         → a Zone A menu item (`MenuFocus::Item(pos - 7)`)
//!   7+items..len       → a Zone B chip (`MenuFocus::ZoneB(pos - 7 - items)`)
//!
//! With no open sessions the ring shrinks to columns + items and wraps from
//! the last item straight back to the first column.

use crate::components::menu_bar::items::{MenuAction, CATEGORIES};
use crate::components::menu_bar::MenuFocus;

use super::board::BoardStore;

/// The number of kanban columns that occupy the front of the ring (R2).
pub(crate) const RING_COLUMNS: usize = 7;

impl BoardStore {
    // ── R2: ring focus state ──────────────────────────────────────────────

    /// R2: the current bar focus — `None` when a COLUMN has focus (the
    /// `focused_column` mirror is authoritative then).
    pub fn menu_focus(&self) -> Option<MenuFocus> {
        self.menu_focus
    }

    /// R9: the open dropdown as `(category, cursor)`; `None` when no
    /// dropdown is showing. Rides on top of the ring focus.
    pub fn open_menu(&self) -> Option<(usize, usize)> {
        self.menu_open
    }

    /// R8: dispatch feeds the chip count (from `open_sessions()`); the
    /// ring length is derived as `RING_COLUMNS + CATEGORIES.len() + chips`.
    pub fn set_menu_ring_size(&mut self, chips: usize) {
        self.menu_chips = chips;
    }

    /// The stored chip count (R8). `0` until the first dispatch-fed refresh.
    pub fn menu_chip_count(&self) -> usize {
        self.menu_chips
    }

    /// R2: walk the ring by `delta` (+1 = Right/l/wheel-right, -1 =
    /// Left/h/wheel-left), wrapping at both ends. A `None` focus (a
    /// column) enters the ring at the first item (delta>0) or the last
    /// stop (delta<0). Landing in a column slot re-mirrors
    /// `focused_column`; the column's card selection is untouched.
    pub fn menu_move(&mut self, delta: i32) {
        let items = CATEGORIES.len();
        let chips = self.menu_chips;
        let len = RING_COLUMNS + items + chips;
        let pos = match self.menu_focus {
            None => self.focused_column as i32,
            Some(MenuFocus::Item(i)) => (RING_COLUMNS + i.min(items - 1)) as i32,
            Some(MenuFocus::ZoneB(j)) => {
                (RING_COLUMNS + items + j.min(chips.saturating_sub(1))) as i32
            }
        };
        let next = (pos + delta).rem_euclid(len as i32) as usize;
        self.menu_focus = if next < RING_COLUMNS {
            self.focused_column = next;
            None
        } else if next < RING_COLUMNS + items {
            Some(MenuFocus::Item(next - RING_COLUMNS))
        } else {
            Some(MenuFocus::ZoneB(next - RING_COLUMNS - items))
        };
    }

    /// R3: drop focus back into the focused column — the bar highlight
    /// clears, the column keeps its selection. No-op when already on a
    /// column.
    pub fn menu_focus_to_columns(&mut self) {
        self.menu_focus = None;
    }

    /// R7 (mouse): focus the `idx`-th menu item, clearing any column
    /// bar-highlight.
    pub fn set_menu_focus_item(&mut self, idx: usize) {
        self.menu_focus = Some(MenuFocus::Item(idx));
    }

    /// R4: toggle/advance the `category` item's dropdown. Closed → open it
    /// at row 0 and focus the item. Open on THIS item → close it (a second
    /// open-tick on the already-open item — GUI parity). Open on a
    /// DIFFERENT item → move the open dropdown to this item at row 0.
    pub fn open_or_close_menu(&mut self, category: usize) {
        match self.menu_open {
            Some((c, _)) if c == category => self.menu_open = None,
            _ => {
                // A transition from CLOSED always re-opens at row 0 (the
                // menu re-arms on open — GUI parity). Only a cursor move
                // (Left/Right ring walk while open) preserves `cursor`.
                self.menu_focus = Some(MenuFocus::Item(category));
                self.menu_open = Some((category, 0));
            }
        }
    }

    /// R9: close the open dropdown; the focused item (if any) stays
    /// highlighted. No-op when nothing is open.
    pub fn close_menu(&mut self) {
        self.menu_open = None;
    }

    /// R6: move the open dropdown's cursor by `delta`, wrapping at both
    /// ends within the category's `entries` (clamped so an empty category
    /// cannot panic the modulo). No-op when no dropdown is open.
    pub fn move_dropdown_cursor(&mut self, delta: i32, entries: usize) {
        if entries == 0 {
            return;
        }
        let Some((category, cursor)) = self.menu_open else {
            return;
        };
        let next = ((cursor as i32 + delta).rem_euclid(entries as i32)) as usize;
        self.menu_open = Some((category, next));
    }

    /// R4: execute the open dropdown's `row` in `category` — resolve it to
    /// the registry `MenuAction` and close the panel. Returns the action
    /// (the caller re-emits it on the bus) or `None` when the category/row
    /// falls outside the registry.
    pub fn execute_dropdown_row(&mut self, category: usize, row: usize) -> Option<MenuAction> {
        let action = CATEGORIES.get(category)?.entries.get(row)?.action;
        self.menu_open = None;
        Some(action)
    }
}
