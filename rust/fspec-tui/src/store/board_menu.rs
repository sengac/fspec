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
    ///
    /// BUG-198 R1: the walk re-anchors an open dropdown — landing on a
    /// Zone A item moves the open panel under that item at row 0 (GUI
    /// parity: the dropdown stays open while cycling items with the
    /// arrows), landing on a column or a chip closes it. This is the
    /// board mirror of the mux surface's `menu_reanchor_or_close`.
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
        // BUG-198 R1: re-anchor the open dropdown under the new focus —
        // an item landing opens (or moves) the panel at row 0; landing
        // off the items (a column or a chip) closes it (the same rule
        // the mux bar walks, `MultiplexLayout::menu_reanchor_or_close`,
        // which has no column stop so its `_` arm is inert there).
        match self.menu_focus {
            Some(MenuFocus::Item(cat)) if self.menu_open.is_some() => {
                self.menu_open = Some((cat, 0));
            }
            _ => {
                self.menu_open = None;
            }
        }
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
                // menu re-arms on open — GUI parity). A Left/Right ring
                // walk landing on this item re-anchors the panel at row
                // 0 the same way (BUG-198 R1, `menu_move`).
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

    /// BUG-196 R3: the click-away gesture — close the open dropdown AND
    /// drop the bar's ring focus (de-select the parent item) so the
    /// surface's key bindings are live again. Unlike `close_menu`
    /// (R9: the keyboard Esc close keeps the item highlighted), this
    /// is the 'leave the bar' gesture.
    pub fn dismiss_menu(&mut self) {
        self.menu_open = None;
        self.menu_focus = None;
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
    /// falls outside the registry. BUG-196 R2: the bar's ring focus
    /// clears on execute (the panel closes AND the parent item
    /// de-selects — the mux layout's `menu_execute_item` parity), so
    /// the surface's key bindings are live immediately.
    pub fn execute_dropdown_row(&mut self, category: usize, row: usize) -> Option<MenuAction> {
        let action = CATEGORIES.get(category)?.entries.get(row)?.action;
        self.menu_open = None;
        self.menu_focus = None;
        Some(action)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::components::menu_bar::MenuFocus;

    /// A fresh store on the given ring (chip count) with the dropdown
    /// state seeded directly (the dispatch-fed count path stays
    /// unchanged — the tests exercise the store's ring math).
    fn store(chips: usize) -> BoardStore {
        let mut s = BoardStore::default();
        s.set_menu_ring_size(chips);
        s
    }

    #[test]
    fn ring_walk_into_an_item_reanchors_the_open_dropdown_at_row_0() {
        // @step Given the board ring is focused on the Kanban item with its dropdown open on row 2
        let mut s = store(1);
        s.menu_focus = Some(MenuFocus::Item(0));
        s.menu_open = Some((0, 2));
        // @step When the ring walks Right one stop (onto Tools)
        s.menu_move(1);
        // @step Then the Tools item has the focus and its dropdown is open at row 0
        assert_eq!(s.menu_focus(), Some(MenuFocus::Item(1)));
        assert_eq!(
            s.open_menu(),
            Some((1, 0)),
            "the open dropdown must follow the ring walk, re-anchored at row 0"
        );
    }

    #[test]
    fn ring_walk_left_into_an_item_reanchors_the_open_dropdown_at_row_0() {
        // @step Given the board ring is focused on the Tools item with the Kanban dropdown open on row 1
        let mut s = store(0);
        s.menu_focus = Some(MenuFocus::Item(1));
        s.menu_open = Some((0, 1));
        // @step When the ring walks Left one stop (back onto Kanban)
        s.menu_move(-1);
        // @step Then the Kanban item has the focus and its dropdown is open at row 0
        assert_eq!(s.menu_focus(), Some(MenuFocus::Item(0)));
        assert_eq!(s.open_menu(), Some((0, 0)));
    }

    #[test]
    fn ring_walk_off_the_items_closes_the_open_dropdown() {
        // @step Given the board ring is focused on the last item (Help) with chips and its dropdown open
        let mut s = store(2);
        s.menu_focus = Some(MenuFocus::Item(3));
        s.menu_open = Some((3, 1));
        // @step When the ring walks Right onto the first chip
        s.menu_move(1);
        // @step Then the chip has the focus and the dropdown is closed
        assert_eq!(s.menu_focus(), Some(MenuFocus::ZoneB(0)));
        assert_eq!(s.open_menu(), None, "a Zone B landing closes the panel");
    }

    #[test]
    fn ring_walk_into_a_column_closes_the_open_dropdown() {
        // @step Given the board ring is focused on the first item (Kanban) with its dropdown open
        let mut s = store(0);
        s.menu_focus = Some(MenuFocus::Item(0));
        s.menu_open = Some((0, 0));
        // @step When the ring walks Left back into the last column
        s.menu_move(-1);
        // @step Then the column has the focus (bar highlight cleared) and the dropdown is closed
        assert_eq!(s.menu_focus(), None, "a column landing drops the bar focus");
        assert_eq!(
            s.focused_column_index(),
            RING_COLUMNS - 1,
            "the focused column mirrors the ring slot"
        );
        assert_eq!(s.open_menu(), None, "a column landing closes the panel");
    }

    #[test]
    fn ring_walk_without_an_open_dropdown_is_unchanged() {
        // @step Given the board ring is focused on the Kanban item with NO dropdown open
        let mut s = store(1);
        s.menu_focus = Some(MenuFocus::Item(0));
        // @step When the ring walks Right one stop
        s.menu_move(1);
        // @step Then the focus moves and the dropdown stays closed
        assert_eq!(s.menu_focus(), Some(MenuFocus::Item(1)));
        assert_eq!(
            s.open_menu(),
            None,
            "a closed dropdown stays closed through the walk"
        );
    }
}
