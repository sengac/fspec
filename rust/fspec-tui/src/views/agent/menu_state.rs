//! MENU-003 — the agent-view menu bar's presentation state.
//!
//! Feature: spec/features/agent-view-surface-2-zone-bar-row-under-session-header.feature
//!
//! Owned by [`AgentView`] as a PLAIN field — `AgentView` is `&mut`
//! through `render_session_pane`, so no `RefCell` is needed, unlike
//! the board's `BoardView`. The ring = menu items + session chips
//! only (no columns in the agent view); the cached geometry + chip
//! count are refreshed by the FOCUSED pane only (the BUG-163 rule)
//! so mouse hit-testing stays bound to the live composer's pane.
//!
//! The chip count + `AgentView`'s `store_handle` test seam make the
//! ring walk + chip activation possible without a store parameter on
//! key events: the focused pane's render keeps `chip_count` in sync
//! with the painted chip list, and the test harness binds its local
//! `AgentViewStore` so a chip activation can focus the session
//! directly (in production the emitted `MenuChipActivate` is
//! resolved by `App::dispatch_menu` instead).

use std::sync::MutexGuard;

use ratatui::layout::Rect;

use crate::components::menu_bar::{DisplayCell, MenuFocus};
use crate::store::AgentViewStore;

use super::AgentView;

/// The menu bar's presentation state for the agent view.
#[derive(Debug, Default)]
pub struct MenuBarState {
    /// Ring focus (`None` = the input / no bar item is focused).
    focus: Option<MenuFocus>,
    /// The open dropdown: `(category index, cursor row)`. MENU-007:
    /// the agent bar is dropdown-free (its single 'Board View' item is
    /// a plain activation button), so this stays `None` in the agent
    /// view — the field is kept for snapshot plumbing only.
    open: Option<(usize, usize)>,
    /// The number of PAINTED chips — the ring length is
    /// `CATEGORIES.len() + chip_count`. Refreshed by the focused
    /// pane's render.
    chip_count: usize,
    /// The bar row's y from the last focused-pane render (`None` =
    /// no bar painted — the mouse arms are inert).
    bar_row: Option<u16>,
    /// The last painted Zone A item rects (`CATEGORIES` order) —
    /// refreshed by the focused pane only.
    item_rects: Option<Vec<Rect>>,
    /// The last painted Zone B cells + hit rects (display order) —
    /// refreshed by the focused pane only.
    cells: Option<Vec<(DisplayCell, Rect)>>,
    /// The open dropdown's panel rect (if any) — the wheel-cursor
    /// target.
    open_panel: Option<Rect>,
}

impl MenuBarState {
    pub fn focus(&self) -> Option<MenuFocus> {
        self.focus
    }

    pub fn set_focus(&mut self, focus: Option<MenuFocus>) {
        self.focus = focus;
    }

    pub fn open(&self) -> Option<(usize, usize)> {
        self.open
    }

    pub fn set_open(&mut self, open: Option<(usize, usize)>) {
        self.open = open;
    }

    pub fn chip_count(&self) -> usize {
        self.chip_count
    }

    pub fn set_chip_count(&mut self, count: usize) {
        self.chip_count = count;
    }

    pub fn bar_row(&self) -> Option<u16> {
        self.bar_row
    }

    pub fn set_bar_row(&mut self, y: Option<u16>) {
        self.bar_row = y;
    }

    pub fn item_rects(&self) -> Option<&[Rect]> {
        self.item_rects.as_deref()
    }

    pub fn set_item_rects(&mut self, rects: Option<Vec<Rect>>) {
        self.item_rects = rects;
    }

    pub fn cells(&self) -> Option<&[(DisplayCell, Rect)]> {
        self.cells.as_deref()
    }

    pub fn set_cells(&mut self, cells: Option<Vec<(DisplayCell, Rect)>>) {
        self.cells = cells;
    }

    pub fn open_panel(&self) -> Option<Rect> {
        self.open_panel
    }

    pub fn set_open_panel(&mut self, panel: Option<Rect>) {
        self.open_panel = panel;
    }

    /// Clear the focus + dropdown (chip activation, the two-stage
    /// Esc's second stage).
    pub fn clear_focus(&mut self) {
        self.focus = None;
        self.open = None;
    }

    /// Clear the cached geometry (focus untouched) — the focused pane
    /// painted without a bar row this frame.
    pub fn clear_geometry(&mut self) {
        self.chip_count = 0;
        self.bar_row = None;
        self.item_rects = None;
        self.cells = None;
        self.open_panel = None;
    }
}

impl AgentView {
    /// MENU-003: true iff the menu-bar arms may own the frame — no
    /// popup / mode view / turn modal is open (those overlay the bar
    /// or consume the frame before the bar does).
    pub(crate) fn menu_surface_active(&self) -> bool {
        self.slash_popup.is_none()
            && self.file_popup.is_none()
            && self.resume_view.is_none()
            && self.search_view.is_none()
            && self.turn_modal_seq.is_none()
    }

    /// MENU-003: the bar's ring focus (`None` = the input is focused).
    pub fn menu_focus(&self) -> Option<MenuFocus> {
        self.menu_state.focus()
    }

    /// MENU-003: the open dropdown's `(category, cursor row)`, if any.
    pub fn open_menu(&self) -> Option<(usize, usize)> {
        self.menu_state.open()
    }

    /// MENU-003 test seam: lock the bound store. Poison recovery:
    /// take the inner — a panic inside a menu arm must not wedge the
    /// UI.
    pub(crate) fn locked_store(&self) -> Option<MutexGuard<'_, AgentViewStore>> {
        self.store_handle.as_ref().map(|handle| {
            handle
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
        })
    }
}
