//! MENU-004 R-MOUSE unit tests for the mux bar hit-test arms.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::views::multiplex::MuxPaneKind;

fn enabled_layout(panes: Vec<MuxPaneKind>) -> MultiplexLayout {
    let mut m = MultiplexLayout::new();
    m.enable_default();
    m.set_layout(crate::views::multiplex::MuxLayout::columns(&panes).unwrap());
    m
}

#[test]
fn inert_until_the_bar_paints() {
    // @step Given a mux layout that never painted the bar
    let mut m = enabled_layout(vec![MuxPaneKind::Board, MuxPaneKind::Agent]);
    // The pane rects exist but NO paint ever ran — the painted
    // flag is off, so every arm stays inert regardless of geometry.
    m.pane_rects.push(ratatui::layout::Rect::new(0, 1, 120, 20));
    m.menu_item_rects = None;
    // @step When a mouse event is classified
    let event = Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollRight,
        column: 5,
        row: 0,
        modifiers: crossterm::event::KeyModifiers::NONE,
    });
    // @step Then the arms stay inert (no paint → no geometry → Pass)
    assert!(matches!(
        classify_bar_mouse(&m, &event),
        MuxBarMouseDecision::Pass
    ));
}

#[test]
fn wheel_over_the_bar_with_no_focus_is_ignored() {
    // @step Given a painted bar with item + cell geometry but no focus
    let mut m = enabled_layout(vec![MuxPaneKind::Board, MuxPaneKind::Agent]);
    m.pane_rects.push(ratatui::layout::Rect::new(0, 1, 120, 20));
    m.menu_bar_painted = true;
    m.menu_bar_rect = Some(ratatui::layout::Rect::new(0, 0, 120, 1));
    m.menu_item_rects = Some(vec![
        ratatui::layout::Rect::new(1, 0, 7, 1),
        ratatui::layout::Rect::new(9, 0, 4, 1),
    ]);
    m.menu_cells = Some(Vec::new());
    m.menu_open_panel = None;
    m.menu_focus = None;
    let event = Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollRight,
        column: 5,
        row: 0,
        modifiers: crossterm::event::KeyModifiers::NONE,
    });
    // @step When wheel right is sent over the bar row
    // @step Then the event is ignored (no focus, no open dropdown)
    assert!(matches!(
        classify_bar_mouse(&m, &event),
        MuxBarMouseDecision::Ignore
    ));
}

#[test]
fn click_outside_an_open_dropdown_closes_and_lands() {
    // @step Given a painted bar with an open dropdown panel
    let mut m = enabled_layout(vec![MuxPaneKind::Board, MuxPaneKind::Agent]);
    m.pane_rects.push(ratatui::layout::Rect::new(0, 1, 120, 20));
    m.menu_bar_painted = true;
    m.menu_bar_rect = Some(ratatui::layout::Rect::new(0, 0, 120, 1));
    m.menu_item_rects = Some(vec![
        ratatui::layout::Rect::new(1, 0, 7, 1),
        ratatui::layout::Rect::new(9, 0, 4, 1),
    ]);
    m.menu_cells = Some(Vec::new());
    m.menu_open_panel = Some(ratatui::layout::Rect::new(1, 1, 30, 9));
    m.menu_focus = Some(crate::components::menu_bar::MenuFocus::Item(0));
    m.open_menu = Some((0, 0));
    let event = Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 60,
        row: 10, // inside a pane, outside the panel
        modifiers: crossterm::event::KeyModifiers::NONE,
    });
    // @step When a left click lands on a pane outside the panel
    // @step Then the dropdown closes AND the click still lands
    assert!(matches!(
        classify_bar_mouse(&m, &event),
        MuxBarMouseDecision::CloseOutsideThenLand
    ));
}
