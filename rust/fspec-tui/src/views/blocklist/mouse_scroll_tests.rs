//! Feature: spec/features/blocklist-view-mouse-scroll.feature
//!
//! Moved out of tests.rs (COV-075: one test file per feature).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::test_support::*;
use super::*;

// Feature: spec/features/blocklist-view-mouse-scroll.feature
#[test]
fn wheel_down_moves_the_selection_down() {
    use crossterm::event::MouseEventKind;
    // @step Given the blocklist view has 20 rules loaded
    let mut view = BlocklistView::new();
    view.set_rules(rules(20));
    view.set_visible_rows(8);
    // @step And the selection is at index 0
    view.selected_index = 0;
    view.adjust_scroll();
    // @step When I scroll the mouse wheel down once
    view.handle_mouse(mouse_event(MouseEventKind::ScrollDown));
    // @step Then the selection moves to index 1
    // (fresh WheelVelocity → step is exactly 1× on the first event)
    assert_eq!(view.selected_index, 1);
    // @step And the visible window scrolls to keep the selection visible
    let off = view.scroll_offset();
    assert!(
        view.selected_index >= off && view.selected_index < off + 8,
        "selection {} not inside window [{}, {})",
        view.selected_index,
        off,
        off + 8
    );
}

// Feature: spec/features/blocklist-view-mouse-scroll.feature
#[test]
fn wheel_down_clamps_at_the_last_rule() {
    use crossterm::event::MouseEventKind;
    // @step Given the blocklist view has 20 rules loaded
    let mut view = BlocklistView::new();
    view.set_rules(rules(20));
    view.set_visible_rows(8);
    // @step And the selection is at the last rule index 19
    view.selected_index = 19;
    view.adjust_scroll();
    // @step When I scroll the mouse wheel down once
    view.handle_mouse(mouse_event(MouseEventKind::ScrollDown));
    // @step Then the selection stays at index 19
    assert_eq!(view.selected_index, 19);
}

// Feature: spec/features/blocklist-view-mouse-scroll.feature
#[test]
fn wheel_up_clamps_at_the_first_rule() {
    use crossterm::event::MouseEventKind;
    // @step Given the blocklist view has 20 rules loaded
    let mut view = BlocklistView::new();
    view.set_rules(rules(20));
    view.set_visible_rows(8);
    // @step And the selection is at index 0
    view.selected_index = 0;
    view.adjust_scroll();
    // @step When I scroll the mouse wheel up once
    view.handle_mouse(mouse_event(MouseEventKind::ScrollUp));
    // @step Then the selection stays at index 0
    assert_eq!(view.selected_index, 0);
}

// Feature: spec/features/blocklist-view-mouse-scroll.feature
#[test]
fn non_wheel_mouse_events_are_ignored() {
    use crossterm::event::MouseEventKind;
    // @step Given the blocklist view has 20 rules loaded
    let mut view = BlocklistView::new();
    view.set_rules(rules(20));
    view.set_visible_rows(8);
    // @step And the selection is at index 5
    view.selected_index = 5;
    view.adjust_scroll();
    // @step When I move the mouse over the view
    let outcome = view.handle_mouse(mouse_event(MouseEventKind::Moved));
    // @step Then the selection stays at index 5
    assert_eq!(view.selected_index, 5);
    // @step And the mouse event is reported as ignored
    assert!(
        matches!(outcome, BlocklistEvent::Ignored),
        "expected Ignored, got {outcome:?}"
    );
}

// Feature: spec/features/blocklist-view-mouse-scroll.feature
#[test]
fn the_navigator_routes_wheel_events_to_the_view() {
    use crate::theme::Theme;
    use crate::views::navigator::{Navigator, ViewMode};
    use crossterm::event::{Event, MouseEventKind};
    use std::sync::Arc;
    use tokio::sync::mpsc::unbounded_channel;

    // @step Given the blocklist view has 20 rules loaded
    let (tx, _rx) = unbounded_channel();
    let mut nav = Navigator::new(Arc::new(Theme::default()), tx);
    nav.active_view = ViewMode::Blocklist;
    nav.blocklist.set_rules(rules(20));
    nav.blocklist.set_visible_rows(8);
    // @step And the selection is at index 0
    nav.blocklist.selected_index = 0;
    nav.blocklist.adjust_scroll();
    // @step When a mouse wheel down event is delivered through the navigator
    let event = Event::Mouse(mouse_event(MouseEventKind::ScrollDown));
    let result = nav.handle_blocklist_event(&event);
    // @step Then the navigator reports the event as consumed
    assert!(result.is_consumed(), "navigator should consume wheel event");
    // @step And the selection moves down
    assert!(
        nav.blocklist.selected_index > 0,
        "selection should move down, got {}",
        nav.blocklist.selected_index
    );
}
