//! MENU-001 — single-line menu bar + anchored dropdown.
//!
//! Feature: spec/features/menubar-component-2-zone-bar-gui-menu-items-anchored-dropdown-with-view-chip-switcher-menucategories-registry-pure-painter.feature
//!
//! The shared menu component painted by three surfaces (board, agent,
//! mux):
//!
//! - **Zone A** — the GUI menu bar: top-level menu items
//!   ([`items::CATEGORIES`]: `Kanban`, `Tools`, `Settings`, `Help`
//!   — MENU-008). Selecting an item (Enter/click) opens its
//!   [`dropdown`] anchored under the item.
//! - **Zone B** — the view/chip switcher: non-mux mode shows the
//!   per-open-session status chips only; mux mode shows panes in pane
//!   order with agent panes as chips (e.g. `Board #1 #2 Files #3 Ckpts`).
//!
//! The unified ring focus ([`MenuFocus`]) walks items → Zone B cells →
//! wrap; the dropdown-open state rides on top of it (a focus on an item
//! plus `open_menu` means that item's dropdown is showing).
//!
//! File map (300-LoC ceiling):
//!   - `mod.rs`           — `MenuSnapshot` / `MenuFocus` / `ZoneBCell` + ring math
//!   - `items.rs`         — the `MenuCategories` registry (R2 single source of truth)
//!   - `chips.rs`         — pure chip snapshot builder (R4 glyphs, R9 Cleared drop)
//!   - `layout.rs`        — pure geometry pass + R6 truncation ladder
//!   - `paint.rs`         — the 1-row bar painter
//!   - `dropdown.rs`      — dropdown geometry (R7 rect/clamp/clip/scroll window)
//!   - `dropdown_paint.rs`— the dropdown panel painter
//!   - `dropdown_text.rs` — key-column + description truncation helpers (R7)
//!
//! All painters are stateless over the owned per-frame
//! [`MenuSnapshot`] (SessionHeader pattern) — the widget never holds
//! focus/dropdown state; the owning surface does.

pub mod chips;
pub mod dropdown;
pub mod dropdown_paint;
pub mod dropdown_text;
pub mod items;
pub mod layout;
pub mod paint;

#[cfg(test)]
mod items_tests;

pub use chips::{build_chips, index_prefix, ChipInput, MenuChip};
pub use dropdown::{dropdown_rect, dropdown_row_at, scroll_window, visible_entry_rows};
pub use dropdown_paint::render_menu_dropdown;
pub use dropdown_text::{key as dropdown_key, truncate as dropdown_truncate, KEY_COL};
pub use items::{MenuAction, MenuCategory, MenuEntry, CATEGORIES, HELP, KANBAN, SETTINGS, TOOLS};
pub use layout::{cell_width, chip_index, menu_bar_layout, DisplayCell, MenuLayout};
pub use paint::paint_menu_bar;

/// Unified ring focus on the menu bar (R3). `None` (held by the caller
/// as `Option<MenuFocus>`) means no bar item is focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuFocus {
    /// A Zone A menu item — index into [`items::CATEGORIES`].
    Item(usize),
    /// A Zone B display cell — index into [`MenuSnapshot::zone_b`]
    /// (a view label in mux mode, or a chip everywhere).
    ZoneB(usize),
}

/// One Zone B cell, in display order (R3). Mux mode interleaves view
/// labels (pane order); non-mux mode is chips only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneBCell {
    /// A mux-mode view label (e.g. `Board` / `Files` / `Ckpts`).
    /// `active` marks the focused pane (bold label, R5).
    View { label: &'static str, active: bool },
    /// A chip from the global [`MenuSnapshot::chips`] list.
    Chip(usize),
}

/// Owned per-frame snapshot the painters read (R3). Built fresh by each
/// surface (board / agent / mux render path) — the widget holds no state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuSnapshot {
    /// The Zone A items the surface paints (board/mux = the full
    /// [`items::CATEGORIES`] registry; the agent view = its single
    /// 'Board View' item, MENU-007 — no dropdowns there). The ring
    /// math, the geometry pass and the painter all read this list.
    pub zone_a: &'static [MenuCategory],
    /// Ring focus (`None` = columns / input / pane has focus instead).
    pub focus: Option<MenuFocus>,
    /// The open dropdown: `(category index into zone_a, cursor row)`.
    /// `None` = no dropdown showing.
    pub open_menu: Option<(usize, usize)>,
    /// Zone B cells in display order.
    pub zone_b: Vec<ZoneBCell>,
    /// The global chip list (addressed by [`ZoneBCell::Chip`]).
    pub chips: Vec<MenuChip>,
    /// Frame clock driving the Running braille frame (R4).
    pub clock_ms: u64,
}

impl Default for MenuSnapshot {
    /// The board/mux shape: the full MenuCategories registry as Zone A.
    fn default() -> Self {
        Self {
            zone_a: items::CATEGORIES,
            focus: None,
            open_menu: None,
            zone_b: Vec::new(),
            chips: Vec::new(),
            clock_ms: 0,
        }
    }
}

impl MenuSnapshot {
    /// R3 ring math: advance the focus by `delta` (+1 right, -1 left)
    /// around items → Zone B cells (display order) → wrap. A `None`
    /// focus has not entered the ring yet: `+1` enters at the first
    /// item, `-1` enters at the LAST cell (the "runs out of menu items
    /// → view switcher" edge, and its mirror). Every subsequent step
    /// is a `rem_euclid` walk, so `+1` from the last Zone B cell lands
    /// on the first item and `-1` from the first item lands on the
    /// last Zone B cell.
    pub fn advance(&self, focus: Option<MenuFocus>, delta: i32) -> MenuFocus {
        let items = self.zone_a.len();
        let len = items + self.zone_b.len();
        if len == 0 {
            return MenuFocus::Item(0);
        }
        let next = match focus {
            None => {
                if delta < 0 {
                    len - 1
                } else {
                    0
                }
            }
            Some(MenuFocus::Item(i)) => {
                let pos = i.min(items.saturating_sub(1)) as i32;
                ((pos + delta).rem_euclid(len as i32)) as usize
            }
            Some(MenuFocus::ZoneB(i)) => {
                let pos = (items + i.min(self.zone_b.len().saturating_sub(1))) as i32;
                ((pos + delta).rem_euclid(len as i32)) as usize
            }
        };
        if next < items {
            MenuFocus::Item(next)
        } else {
            MenuFocus::ZoneB(next - items)
        }
    }

    /// Clamp a cursor into the open category's row range (wrap-safe).
    pub fn dropdown_cursor(&self, category: usize, cursor: usize) -> usize {
        let len = self
            .zone_a
            .get(category)
            .map(|c| c.entries.len())
            .unwrap_or(1);
        cursor % len.max(1)
    }

    /// The chip index a Zone B display cell refers to (`None` for view
    /// labels) — the chip-activation resolution helper for the surfaces.
    pub fn chip_for_cell(&self, cell: usize) -> Option<usize> {
        match self.zone_b.get(cell) {
            Some(ZoneBCell::Chip(i)) => Some(*i),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn chip(i: usize) -> MenuChip {
        MenuChip {
            index: (i + 1, 3),
            glyph: "●".to_string(),
            glyph_style: Default::default(),
            wu_id: None,
            active: false,
        }
    }

    /// 2 items + 3 chips, all in Zone B (non-mux shape).
    fn snap3() -> MenuSnapshot {
        let chips = vec![chip(0), chip(1), chip(2)];
        MenuSnapshot {
            zone_b: vec![ZoneBCell::Chip(0), ZoneBCell::Chip(1), ZoneBCell::Chip(2)],
            chips,
            ..Default::default()
        }
    }

    #[test]
    fn ring_walks_items_then_zone_b_then_wraps_forward() {
        // @step Given a MenuSnapshot with MenuFocus on the 0th menu item
        let s = snap3();
        assert_eq!(
            s.advance(None, 1),
            MenuFocus::Item(0),
            "no focus + right starts at the first item"
        );
        assert_eq!(s.advance(Some(MenuFocus::Item(0)), 1), MenuFocus::Item(1));
        assert_eq!(
            s.advance(Some(MenuFocus::Item(3)), 1),
            MenuFocus::ZoneB(0),
            "running out of items lands on the first Zone B cell"
        );
        assert_eq!(s.advance(Some(MenuFocus::ZoneB(2)), 1), MenuFocus::Item(0));
    }

    #[test]
    fn ring_walks_backwards_with_wrap() {
        // @step Given a MenuSnapshot with 2 open sessions and MenuFocus on chip 1
        let s = snap3();
        assert_eq!(s.advance(Some(MenuFocus::Item(0)), -1), MenuFocus::ZoneB(2));
        assert_eq!(s.advance(Some(MenuFocus::ZoneB(0)), -1), MenuFocus::Item(3));
    }

    #[test]
    fn ring_follows_the_display_order_in_mux_mode() {
        // Mux: Zone B = Board view, chip 1, chip 0, chip 2 (pane order).
        let chips = vec![chip(0), chip(1), chip(2)];
        let s = MenuSnapshot {
            zone_b: vec![
                ZoneBCell::View {
                    label: "Board",
                    active: true,
                },
                ZoneBCell::Chip(1),
                ZoneBCell::Chip(0),
                ZoneBCell::Chip(2),
            ],
            chips,
            ..Default::default()
        };
        assert_eq!(s.advance(Some(MenuFocus::Item(3)), 1), MenuFocus::ZoneB(0));
        assert_eq!(s.advance(Some(MenuFocus::ZoneB(0)), 1), MenuFocus::ZoneB(1));
        assert_eq!(s.advance(Some(MenuFocus::ZoneB(3)), 1), MenuFocus::Item(0));
    }

    #[test]
    fn empty_zone_b_ring_wraps_items_onto_themselves() {
        let s = MenuSnapshot::default();
        assert_eq!(s.advance(None, 1), MenuFocus::Item(0));
        assert_eq!(s.advance(Some(MenuFocus::Item(0)), 1), MenuFocus::Item(1));
        assert_eq!(s.advance(Some(MenuFocus::Item(3)), 1), MenuFocus::Item(0));
    }

    #[test]
    fn dropdown_cursor_clamps_into_the_category_range() {
        let s = MenuSnapshot::default();
        assert_eq!(s.dropdown_cursor(0, 3), 3, "Kanban has 4 rows");
        assert_eq!(s.dropdown_cursor(0, 4), 0, "wraps past the end");
        assert_eq!(s.dropdown_cursor(3, 1), 1, "Help has 2 rows");
        assert_eq!(s.dropdown_cursor(3, 2), 0, "Help wraps");
    }

    #[test]
    fn chip_for_cell_resolves_only_chip_cells() {
        let s = snap3();
        assert_eq!(s.chip_for_cell(0), Some(0));
        assert_eq!(s.chip_for_cell(2), Some(2));
        assert_eq!(s.chip_for_cell(9), None, "out of range");
    }
}
