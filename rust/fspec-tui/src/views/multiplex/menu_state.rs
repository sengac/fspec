//! MENU-004 — the mux menu bar's ring/dropdown state on `MultiplexLayout`.
//!
//! Feature: spec/features/mux-surface-single-top-of-mux-menu-bar-per-pane-suppression-spinner-draw-tick.feature
//! Cards: R-KEYS, R-CHIPS, R-ZONEB, R-STATE.
//!
//! RED CARD 1 (resolved): the bar's state lives on the MUX layer, NOT
//! the board store — the bar belongs to the mux surface.
//! `App::dispatch_menu` picks the holder by `navigator.active_view`
//! (Board/Agent views keep their MENU-002/003 holders untouched).
//!
//! The ring = Zone A items + Zone B cells (pane view labels + the
//! GLOBAL session chips — RED CARD 3). The chip count is fed by
//! `App::dispatch` through [`MultiplexLayout::refresh_menubar_ring`]
//! (R8 lockstep, the board `set_menu_ring_size` mirror) so key-time
//! ring math agrees with the bar's paint; the ring walk itself runs
//! against a per-frame [`MenuSnapshot`] (the dispatched walk uses the
//! painted zone_b, built with `menu_snapshot::build_snapshot`).

use crate::components::menu_bar::items::CATEGORIES;
use crate::components::menu_bar::{items, MenuFocus};
use crate::store::AgentViewStore;

use super::{MultiplexLayout, MuxPaneKind};

impl MultiplexLayout {
    // ── accessors (the test + dispatch surface) ─────────────────────────

    /// MENU-004: the mux bar's ring focus (`None` = a pane has focus).
    pub fn menu_focus(&self) -> Option<MenuFocus> {
        self.menu_focus
    }

    /// MENU-004: the open dropdown `(category index, cursor row)`.
    pub fn open_menu(&self) -> Option<(usize, usize)> {
        self.open_menu
    }

    /// MENU-004 R-KEYS: true iff the mux bar's ring is ACTIVE (an item
    /// or Zone B cell has the ring focus, or a dropdown is open).
    /// `App::dispatch_menu` uses this to keep `MenuMove` walks on the
    /// MUX layer while the bar is engaged; when the bar is INACTIVE
    /// (a plain pane has focus) the same `MenuMove` token belongs to the
    /// focused pane's OWN surface (the board pane's continuous
    /// column⇄item⇄chip ring — MENU-002, which feeds the bar only via
    /// its edge rule `MenuMoveToItem(0)`).
    pub fn menu_ring_active(&self) -> bool {
        self.menu_focus.is_some() || self.open_menu.is_some()
    }

    /// MENU-004 R-TICK: true iff the last render painted the mux bar
    /// (the guard for the 3-row degradation: the mouse arms stay
    /// inert and the tick gate stays closed when no bar painted).
    pub fn menu_bar_painted(&self) -> bool {
        self.menu_bar_painted
    }

    /// MENU-004 R-MOUSE: the cached Zone A item rects (CATEGORIES
    /// order) from the last painted frame (`None` = no bar painted).
    pub fn menu_item_rects(&self) -> Option<&[ratatui::layout::Rect]> {
        self.menu_item_rects.as_deref()
    }

    /// MENU-004 R-MOUSE: the cached Zone B display cells + hit rects
    /// (display order) from the last painted frame.
    pub fn menu_cells(
        &self,
    ) -> Option<
        &[(
            crate::components::menu_bar::DisplayCell,
            ratatui::layout::Rect,
        )],
    > {
        self.menu_cells.as_deref()
    }

    /// MENU-004 R-MOUSE: the open dropdown's panel rect, if any.
    pub fn menu_open_panel(&self) -> Option<ratatui::layout::Rect> {
        self.menu_open_panel
    }

    /// The number of Zone B view-label cells the bar paints (the
    /// non-agent effective panes, R-ZONEB) — the offset from a store
    /// chip index to the painted Zone B cell index.
    pub fn view_label_count(&self) -> usize {
        self.effective_panes()
            .iter()
            .filter(|k| **k != MuxPaneKind::Agent)
            .count()
    }

    /// MENU-004 R8: the fed painted-chip count (the ring's Zone B chip
    /// cells). The App's dispatched `MenuMove` mirror (BUG-200) reads
    /// it to compute the bar's last Zone B cell index.
    pub fn menu_chips(&self) -> usize {
        self.menu_chips
    }

    // ── ring state ──────────────────────────────────────────────────────

    /// MENU-004: focus the `idx`-th Zone A item, clearing any other
    /// bar highlight.
    pub fn menu_move_to_item(&mut self, idx: usize) {
        self.menu_focus = Some(MenuFocus::Item(idx));
    }

    /// MENU-004 R-ZONEB: land the ring focus on an exact ring position
    /// (the App's dispatched walk via `MenuSnapshot::advance` — the
    /// board store's `menu_move` mirror for the mux bar).
    pub(crate) fn menu_move_to_ring_position(&mut self, focus: MenuFocus) {
        self.menu_focus = Some(focus);
    }

    /// MENU-004 R-MOUSE: click on an item — focus it and toggle its
    /// dropdown (the already-open item closes — GUI parity, R4).
    pub fn menu_open_or_close(&mut self, category: usize) {
        self.menu_focus = Some(MenuFocus::Item(category));
        match self.open_menu {
            Some((c, _)) if c == category => self.open_menu = None,
            _ => self.open_menu = Some((category, 0)),
        }
    }

    /// MENU-004 R-KEYS: move the open dropdown's cursor by `delta`
    /// (wrapping within the category's entries). No-op when nothing is
    /// open or the category is unknown.
    pub fn menu_dropdown_cursor(&mut self, delta: i32) {
        let Some((category, cursor)) = self.open_menu else {
            return;
        };
        let Some(entries) = CATEGORIES
            .get(category)
            .map(|c| c.entries.len())
            .filter(|e| *e > 0)
        else {
            return;
        };
        let next = ((cursor as i32 + delta).rem_euclid(entries as i32)) as usize;
        self.open_menu = Some((category, next));
    }

    /// MENU-004 R-KEYS: close the open dropdown; the focused item (if
    /// any) stays highlighted.
    pub fn menu_close_dropdown(&mut self) {
        self.open_menu = None;
    }

    /// MENU-004 R-KEYS: a character key (or Up/Down from a closed bar)
    /// dismisses the bar — the ring focus clears and the focused pane
    /// regains its input (the draft is preserved).
    pub fn menu_dismiss(&mut self) {
        self.menu_focus = None;
        self.open_menu = None;
    }

    /// MENU-004 R-EXEC: resolve the open dropdown's `row` in `category`
    /// to the registry `MenuAction` and clear the bar (the caller
    /// re-emits the action on the bus).
    pub fn menu_execute_item(&mut self, category: usize, row: usize) -> Option<items::MenuAction> {
        let action = CATEGORIES.get(category)?.entries.get(row)?.action;
        self.menu_focus = None;
        self.open_menu = None;
        Some(action)
    }

    /// MENU-004 R-STATE: drop the bar state (mux exit / disable) — no
    /// stale focus/dropdown survives on the saved layout.
    pub fn menu_reset(&mut self) {
        self.menu_focus = None;
        self.open_menu = None;
        self.menu_item_rects = None;
        self.menu_cells = None;
        self.menu_open_panel = None;
        self.menu_bar_painted = false;
        self.menu_chips = 0;
    }

    /// After a ring walk landing: an item re-anchors an open dropdown
    /// at row 0 (R-KEYS: "Left/Right walk the ring + re-anchor"); a
    /// Zone B cell closes it.
    pub(crate) fn menu_reanchor_or_close(&mut self) {
        match self.menu_focus {
            Some(MenuFocus::Item(i)) if self.open_menu.is_some() => {
                self.open_menu = Some((i, 0));
            }
            Some(MenuFocus::ZoneB(_)) => self.open_menu = None,
            _ => {}
        }
    }

    // ── render cache (MENU-004 R-LAYOUT) ────────────────────────────────

    /// Stash this frame's painted-bar geometry for the mouse arms.
    /// The bar row is always row 0 of the mux area — the mouse arms
    /// derive its absolute y from the pane rects (`pane_rects()[0].y - 1`).
    pub(crate) fn cache_menu_geometry(
        &mut self,
        item_rects: Vec<ratatui::layout::Rect>,
        cells: Vec<(
            crate::components::menu_bar::DisplayCell,
            ratatui::layout::Rect,
        )>,
        open_panel: Option<ratatui::layout::Rect>,
    ) {
        self.menu_item_rects = Some(item_rects);
        self.menu_cells = Some(cells);
        self.menu_open_panel = open_panel;
        self.menu_bar_painted = true;
    }

    /// The 3-row degradation: the last frame painted no bar — the
    /// mouse arms stay inert and the tick gate closes.
    pub(crate) fn clear_menu_geometry(&mut self) {
        self.menu_item_rects = None;
        self.menu_cells = None;
        self.menu_open_panel = None;
        self.menu_bar_painted = false;
    }
}

/// Refresh the mux bar's fed ring size (R8 lockstep) from the live
/// store — `App::dispatch` calls this at the top of every tick.
impl MultiplexLayout {
    pub(crate) fn refresh_menubar_ring(&mut self, agent_store: &AgentViewStore) {
        self.menu_chips =
            crate::views::board::menu_snapshot::active_menu_session_ids(agent_store).len();
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::components::menu_bar::MenuSnapshot;

    /// A layout with the given pane list, fed ring, and focus.
    fn layout(focus: Option<MenuFocus>, chips: usize) -> MultiplexLayout {
        let mut m = MultiplexLayout::new();
        m.enable_default();
        m.set_pane_list(vec![MuxPaneKind::Board, MuxPaneKind::Agent], None);
        m.menu_chips = chips;
        m.menu_focus = focus;
        m
    }

    /// Walk one ring stop against a synthetic painted snapshot
    /// (items + chips zone_b — mirrors the dispatched walk).
    fn walk(m: &mut MultiplexLayout, chips: usize, delta: i32) {
        let Some(focus) = m.menu_focus() else {
            return;
        };
        let snap = MenuSnapshot {
            zone_b: (0..chips)
                .map(crate::components::menu_bar::ZoneBCell::Chip)
                .collect(),
            ..MenuSnapshot::default()
        };
        m.menu_focus = Some(snap.advance(Some(focus), delta));
        m.menu_reanchor_or_close();
    }

    #[test]
    fn ring_walks_items_then_chips_then_wraps() {
        // @step Given a MultiplexLayout with 2 open sessions and the ring focus on Item(0)
        let mut m = layout(Some(MenuFocus::Item(0)), 2);
        // @step When the ring walks Right six stops
        walk(&mut m, 2, 1);
        walk(&mut m, 2, 1);
        walk(&mut m, 2, 1);
        walk(&mut m, 2, 1);
        walk(&mut m, 2, 1);
        walk(&mut m, 2, 1);
        // @step Then the focus wraps from the last chip back to Item(0)
        // (4 items + 2 chips = 6 ring stops)
        assert_eq!(m.menu_focus(), Some(MenuFocus::Item(0)));
    }

    #[test]
    fn ring_walks_left_from_the_first_item_to_the_last_chip() {
        let mut m = layout(Some(MenuFocus::Item(0)), 2);
        walk(&mut m, 2, -1);
        assert_eq!(m.menu_focus(), Some(MenuFocus::ZoneB(1)));
    }

    #[test]
    fn walk_into_an_item_reanchors_the_open_dropdown_at_row_0() {
        // @step Given the Kanban dropdown is open with the cursor on row 1
        let mut m = layout(Some(MenuFocus::Item(0)), 1);
        m.open_menu = Some((0, 1));
        // @step When the ring walks Right onto Tools
        walk(&mut m, 1, 1);
        // @step Then Tools has the focus and its dropdown is open at row 0
        assert_eq!(m.menu_focus(), Some(MenuFocus::Item(1)));
        assert_eq!(m.open_menu(), Some((1, 0)));
    }

    #[test]
    fn walk_onto_a_zone_b_cell_closes_the_dropdown() {
        let mut m = layout(Some(MenuFocus::Item(0)), 1);
        m.open_menu = Some((0, 0));
        // @step When the ring walks Right four stops (past all four items) onto the chip
        walk(&mut m, 1, 1);
        walk(&mut m, 1, 1);
        walk(&mut m, 1, 1);
        walk(&mut m, 1, 1);
        assert_eq!(m.menu_focus(), Some(MenuFocus::ZoneB(0)));
        assert_eq!(m.open_menu(), None);
    }

    #[test]
    fn open_or_close_toggles_the_same_item_and_reopens_others_at_row_0() {
        let mut m = layout(Some(MenuFocus::Item(1)), 0);
        m.menu_open_or_close(1);
        assert_eq!(m.open_menu(), Some((1, 0)));
        m.menu_open_or_close(1);
        assert_eq!(m.open_menu(), None, "the second tick closes");
        assert_eq!(m.menu_focus(), Some(MenuFocus::Item(1)));
        m.menu_open_or_close(0);
        assert_eq!(
            m.open_menu(),
            Some((0, 0)),
            "another item re-opens at row 0"
        );
    }

    #[test]
    fn dropdown_cursor_wraps_within_the_category_entries() {
        let mut m = layout(Some(MenuFocus::Item(0)), 0);
        m.open_menu = Some((0, 3));
        m.menu_dropdown_cursor(1);
        assert_eq!(m.open_menu(), Some((0, 0)), "Kanban has 4 rows");
        m.menu_dropdown_cursor(-1);
        assert_eq!(m.open_menu(), Some((0, 3)));
    }

    #[test]
    fn execute_resolves_the_registry_row_and_clears_the_bar() {
        let mut m = layout(Some(MenuFocus::Item(0)), 0);
        m.open_menu = Some((1, 1));
        let action = m.menu_execute_item(1, 1);
        assert_eq!(
            action,
            Some(crate::components::menu_bar::items::MenuAction::Checkpoints)
        );
        assert_eq!(m.menu_focus(), None, "the bar focus clears on execute");
        assert_eq!(m.open_menu(), None);
    }

    #[test]
    fn reset_clears_every_bar_field() {
        let mut m = layout(Some(MenuFocus::Item(0)), 2);
        m.menu_bar_painted = true;
        m.menu_reset();
        assert_eq!(m.menu_focus(), None);
        assert_eq!(m.open_menu(), None);
        assert!(!m.menu_bar_painted());
        assert_eq!(m.menu_chips, 0);
    }

    #[test]
    fn view_label_count_counts_the_non_agent_panes() {
        let mut m = MultiplexLayout::new();
        m.enable_default();
        m.set_pane_list(
            vec![
                MuxPaneKind::Board,
                MuxPaneKind::Agent,
                MuxPaneKind::Agent,
                MuxPaneKind::Checkpoints,
            ],
            None,
        );
        assert_eq!(m.view_label_count(), 2, "Board + Ckpts labels");
    }
}
