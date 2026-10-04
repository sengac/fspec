//! MENU-003 + MENU-007 — the agent pane's 2-zone menu bar paint.
//!
//! Feature: spec/features/agent-view-surface-2-zone-bar-row-under-session-header.feature
//!
//! Factored out of `pane_render.rs` so that file stays under the
//! 300-LoC ceiling pinned by `rpc018` / `rpc020` source-shape. The
//! snapshot is built from the store (chips = open sessions, R9
//! `Cleared` drop — the same builder the board bar uses, so the ring
//! size, painted cells, and activation index stay in lockstep).
//!
//! MENU-007: the agent bar's Zone A is a SINGLE 'Board View' item
//! (no dropdowns — the agent view is dropdown-free). The board/mux
//! bars keep the full `CATEGORIES` registry. The geometry caches are
//! refreshed by the FOCUSED bar-row pane only (the BUG-163 rule).

use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::buffer::Buffer;

use crate::components::menu_bar::items::MenuCategory;
use crate::components::menu_bar::{paint_menu_bar, MenuSnapshot};
use crate::store::AgentViewStore;
use crate::theme::Theme;

use super::chrome_paint::ChromeAreas;
use super::AgentView;

/// MENU-007: the agent bar's single Zone A item. It carries NO
/// dropdown entries — it is a plain activation button (Enter/click
/// emits `Action::BackToBoard`), so the dropdown geometry pass always
/// finds an empty entry list and paints nothing.
pub(crate) const AGENT_ZONE_A: &[MenuCategory] = &[MenuCategory {
    id: "board-view",
    label: "Board View",
    entries: &[],
}];

/// The wall clock in ms since the epoch — drives the Running chip's
/// braille frame (MENU-003, parity with the board bar).
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// The per-frame agent-bar [`MenuSnapshot`]: the store's painted chips
/// (R9 `Cleared` drop parity) + the single 'Board View' Zone A item
/// (MENU-007) + the view's owned focus / open state.
fn agent_bar_snapshot(
    store: &AgentViewStore,
    focus: Option<crate::components::menu_bar::MenuFocus>,
    open_menu: Option<(usize, usize)>,
    clock_ms: u64,
) -> MenuSnapshot {
    let active = crate::views::board::menu_snapshot::active_menu_session_ids(store);
    let (zone_b, chips) =
        crate::views::board::menu_snapshot::zone_b_and_chips(store, &active, clock_ms);
    MenuSnapshot {
        zone_a: AGENT_ZONE_A,
        focus,
        open_menu,
        zone_b,
        chips,
        clock_ms,
    }
}

impl AgentView {
    /// MENU-003: paint the 2-zone bar row into `areas.menu` and refresh
    /// the view-level geometry caches (focused bar-row pane only).
    pub(crate) fn paint_menu_bar_row(
        &mut self,
        areas: &ChromeAreas,
        buf: &mut Buffer,
        store: &AgentViewStore,
        is_focused: bool,
    ) {
        let snapshot = agent_bar_snapshot(
            store,
            self.menu_state.focus(),
            self.menu_state.open(),
            now_ms(),
        );
        let Some(layout) = paint_menu_bar(areas.menu, buf, &snapshot, &Theme::default()) else {
            if is_focused {
                self.menu_state.clear_geometry();
            }
            return;
        };
        // BUG-163: only the FOCUSED bar-row pane refreshes the caches
        // (unfocused panes paint from the local layout, no caching).
        if !is_focused {
            return;
        }
        let bar_y = areas.menu.y;
        self.menu_state.set_chip_count(snapshot.chips.len());
        self.menu_state.set_bar_row(Some(bar_y));
        self.menu_state
            .set_item_rects(Some(layout.item_rects.clone()));
        let cells = layout
            .cells
            .iter()
            .zip(&layout.cell_rects)
            .map(|(cell, rect)| (*cell, *rect))
            .collect();
        self.menu_state.set_cells(Some(cells));
        // MENU-007: the agent bar is dropdown-free — the open-panel
        // cache stays empty (the overlay painter is a no-op).
        self.menu_state.set_open_panel(None);
    }
}
