//! MENU-004 — the mux's top-row 2-zone menu bar + dropdown overlay paint.
//!
//! Feature: spec/features/mux-surface-single-top-of-mux-menu-bar-per-pane-suppression-spinner-draw-tick.feature
//! Cards: R-LAYOUT, R-MOUSE (geometry cache), R-TICK (clock feed).
//!
//! The bar is row 0 of the mux `area` (full width); the panes + dividers
//! paint within the body BELOW it. The per-frame snapshot comes from
//! [`super::menu_snapshot::build_snapshot`] (Zone B = pane view labels +
//! the GLOBAL session chips); the painters are the shared MENU-001
//! component. The cached geometry (item rects, cells, open panel) is the
//! R-MOUSE hit-test surface — `None`/`menu_bar_painted == false` keeps
//! every mouse arm inert (3-row degradation).

use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::components::menu_bar::{
    dropdown_rect, paint_menu_bar, render_menu_dropdown, MenuLayout,
};
use crate::store::AgentViewStore;
use crate::theme::Theme;

use super::menu_snapshot::build_snapshot;
use super::MultiplexLayout;

/// The wall clock in ms since the epoch — drives the Running chip's
/// braille frame (R-TICK; same source as the board/agent bars'
/// `now_ms`).
pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Paint the mux menu bar into `area` (row 0) and cache the geometry.
/// `None` (painting nothing) when the area is too small for the bar.
pub fn paint_bar(
    layout: &mut MultiplexLayout,
    area: Rect,
    buf: &mut Buffer,
    agent_store: &AgentViewStore,
    theme: &Theme,
) -> Option<MenuLayout> {
    let open = layout.open_menu();
    // BUG-200 R1: re-sync the fed chip count from the LIVE store before
    // deriving the snapshot — `refresh_menubar_ring` otherwise only runs
    // on bus-action dispatch (`App::dispatch` → `feed_menu_ring_size`),
    // so any store change that paints without an intervening dispatch
    // (a session seeded through a test seam, a resumed bootstrap, …)
    // left `menu_chips` one short of the paint: `zone_b_target`
    // resolved the last chip's zone_b index to `None` and the click was
    // Swallowed — "the chips can't be clicked" in mux mode. Paint and
    // hit-test must derive the chip list from the SAME store state.
    layout.refresh_menubar_ring(agent_store);
    let snapshot = build_snapshot(layout, agent_store, layout.menu_focus(), open, now_ms());
    let bar_rect = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 1,
    };
    let menu_layout = paint_menu_bar(bar_rect, buf, &snapshot, theme)?;
    let open_panel = open.and_then(|(cat, _)| {
        let item_x = menu_layout
            .item_rects
            .get(cat)
            .map(|r| r.x)
            .unwrap_or(area.x);
        dropdown_rect(area, item_x, bar_rect.y, cat)
    });
    let cells = menu_layout
        .cells
        .iter()
        .zip(&menu_layout.cell_rects)
        .map(|(cell, rect)| (*cell, *rect))
        .collect();
    layout.cache_menu_geometry(menu_layout.item_rects.clone(), cells, open_panel);
    Some(menu_layout)
}

/// MENU-004 R-MOUSE: paint the open dropdown OVER the panes below the
/// bar (turn-modal overlay precedent — the panel's own bg fill clears
/// what was beneath). No-op when no dropdown is open or the panel did
/// not fit.
pub fn paint_dropdown(layout: &MultiplexLayout, buf: &mut Buffer, theme: &Theme) {
    let Some((cat, cursor)) = layout.open_menu() else {
        return;
    };
    if let Some(panel) = layout.menu_open_panel() {
        render_menu_dropdown(panel, buf, cat, cursor, theme);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::views::multiplex::menu_keys::ZoneBTarget;
    use crate::views::multiplex::MuxPaneKind;

    /// A fresh AgentViewStore + a mux-enabled layout (test-side
    /// constructor — `Navigator::new` is private to the crate, so the
    /// layout is built directly).
    fn enabled_layout(panes: Vec<MuxPaneKind>) -> MultiplexLayout {
        let mut m = MultiplexLayout::new();
        m.enable_default();
        m.set_pane_list(panes, None);
        m
    }

    #[test]
    fn paint_bar_caches_the_item_and_cell_geometry() {
        // @step Given a mux layout with [Board | Files] panes and no sessions
        let mut m = enabled_layout(vec![MuxPaneKind::Board, MuxPaneKind::ChangedFiles]);
        let store = AgentViewStore::default();
        let area = Rect::new(0, 0, 120, 1);
        let mut buf = Buffer::empty(area);
        // @step When the mux bar is painted
        paint_bar(&mut m, area, &mut buf, &store, &Theme::default()).expect("layout");
        // @step Then the geometry cache holds the item rects, the view-label cells, and the painted flag
        assert!(m.menu_bar_painted());
        assert!(
            m.menu_item_rects().is_some_and(|v| v.len() == 4),
            "MENU-008: four Zone A items (Kanban, Tools, Settings, Help)"
        );
        assert!(m.menu_cells().is_some_and(|v| v.len() == 2));
        assert_eq!(m.menu_open_panel(), None);
    }

    #[test]
    fn paint_bar_keeps_the_chips_zone_in_lockstep_with_the_paint() {
        // BUG-200 R2: a repaint re-syncs `menu_chips` (and thus
        // `zone_b_target`) from the live store — `refresh_menubar_ring`
        // only fires on bus-action dispatch, so a session seeded
        // without one (or any store change between dispatch ticks)
        // must NOT leave the hit-test one chip short of the paint.
        // @step Given a mux layout with [Board | Agent | Agent] panes and 2 open sessions
        let mut m = enabled_layout(vec![
            MuxPaneKind::Board,
            MuxPaneKind::Agent,
            MuxPaneKind::Agent,
        ]);
        m.sync_window(&[
            codelet_rpc_types::SessionId::new("s-1"),
            codelet_rpc_types::SessionId::new("s-2"),
        ]);
        let mut store = AgentViewStore::default();
        store.append_session(crate::store::SessionContext::new(
            codelet_rpc_types::SessionId::new("s-1"),
        ));
        store.append_session(crate::store::SessionContext::new(
            codelet_rpc_types::SessionId::new("s-2"),
        ));
        let area = Rect::new(0, 0, 240, 1);
        let mut buf = Buffer::empty(area);
        // @step When the mux bar is painted (no bus-action dispatch has fed the ring)
        paint_bar(&mut m, area, &mut buf, &store, &Theme::default()).expect("layout");
        // @step Then every painted Zone B cell is addressable — the 2 view labels + both chips
        let cells = m
            .menu_cells()
            .expect("painted bar caches its cells")
            .to_vec();
        assert!(
            cells.len() >= 3,
            "the paint shows 1 view label + 2 chips, got {cells:?}"
        );
        for (cell, _rect) in &cells {
            match cell {
                crate::components::menu_bar::DisplayCell::View { orig, .. }
                | crate::components::menu_bar::DisplayCell::Chip { orig }
                | crate::components::menu_bar::DisplayCell::Fold { orig, .. } => {
                    assert!(
                        !matches!(m.zone_b_target(*orig), ZoneBTarget::None),
                        "painted cell at zone_b index {orig} must resolve to a target"
                    );
                }
            }
        }
        assert_eq!(
            m.zone_b_target(m.view_label_count() + 1),
            ZoneBTarget::Chip(1),
            "the second chip (index 1) must be hit-testable"
        );
    }

    #[test]
    fn clear_menu_geometry_resets_the_painted_flag() {
        // @step Given a layout that painted the bar
        let mut m = enabled_layout(vec![MuxPaneKind::Board]);
        let store = AgentViewStore::default();
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);
        paint_bar(&mut m, area, &mut buf, &store, &Theme::default()).expect("layout");
        assert!(m.menu_bar_painted());
        // @step When the 3-row degradation clears the geometry
        m.clear_menu_geometry();
        // @step Then the bar painted flag is off and the caches are empty
        assert!(!m.menu_bar_painted());
        assert_eq!(m.menu_item_rects(), None);
        assert_eq!(m.menu_cells(), None);
    }
}
