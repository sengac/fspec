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
pub mod item_keys;
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
    /// A Zone C action button — index into [`MenuSnapshot::zone_c`]
    /// (MENU-009: the right-aligned New Agent / Close buttons).
    ZoneC(usize),
}

/// One Zone C action button (MENU-009, R1): a right-aligned button the
/// surface paints at the row's right edge. Each surface picks its own
/// `&'static` slice (the `AGENT_ZONE_A` precedent, MENU-007) — the
/// shared painter stays surface-agnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneCButton {
    /// The label painted as the button (e.g. `"New Agent"`).
    pub label: &'static str,
    /// The action the button executes (Enter / left click).
    pub action: items::MenuAction,
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
    /// MENU-009: the Zone C action buttons, right-aligned (R1). The
    /// surface picks its `&'static` slice — board: `[New Agent]`,
    /// agent: `[New Agent, Close]`, mux: `&[]` (empty ⇒ the
    /// ring / layout / paint arms are all byte-identical to
    /// pre-MENU-009).
    pub zone_c: &'static [ZoneCButton],
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
            zone_c: &[],
            clock_ms: 0,
        }
    }
}

impl MenuSnapshot {
    /// R3 ring math (MENU-009: items → Zone B → Zone C → wrap):
    /// advance the focus by `delta` (+1 right, -1 left) around the
    /// ring. A `None` focus has not entered the ring yet: `+1` enters
    /// at the first item, `-1` enters at the LAST cell (the "runs out
    /// of menu items → view switcher" edge, and its mirror — now the
    /// last Zone C button when the surface carries one). Every
    /// subsequent step is a `rem_euclid` walk, so `+1` from the last
    /// stop (Zone C button or last Zone B cell when `zone_c` is
    /// empty) lands on the first item and `-1` from the first item
    /// lands on the last stop. An empty `zone_c` (mux) keeps the
    /// pre-MENU-009 walk byte-identical.
    pub fn advance(&self, focus: Option<MenuFocus>, delta: i32) -> MenuFocus {
        let items = self.zone_a.len();
        let zone_b = self.zone_b.len();
        let zone_c = self.zone_c.len();
        let len = items + zone_b + zone_c;
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
                let pos = (items + i.min(zone_b.saturating_sub(1))) as i32;
                ((pos + delta).rem_euclid(len as i32)) as usize
            }
            Some(MenuFocus::ZoneC(i)) => {
                let pos = (items + zone_b + i.min(zone_c.saturating_sub(1))) as i32;
                ((pos + delta).rem_euclid(len as i32)) as usize
            }
        };
        if next < items {
            MenuFocus::Item(next)
        } else if next < items + zone_b {
            MenuFocus::ZoneB(next - items)
        } else {
            MenuFocus::ZoneC(next - items - zone_b)
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
mod tests;
