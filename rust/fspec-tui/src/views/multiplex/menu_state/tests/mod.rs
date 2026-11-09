#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

use crate::components::menu_bar::MenuSnapshot;

/// A layout with the given pane list, fed ring, and focus.
fn layout(focus: Option<MenuFocus>, chips: usize) -> MultiplexLayout {
    let mut m = MultiplexLayout::new();
    m.enable_default();
    m.set_layout(
        crate::views::multiplex::MuxLayout::columns(&[MuxPaneKind::Board, MuxPaneKind::Agent])
            .unwrap(),
    );
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

mod part_1;
