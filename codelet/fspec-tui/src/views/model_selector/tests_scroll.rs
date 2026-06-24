//! PROV-107 — RPC-340 scroll-follows-cursor viewport tests.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::test_support::*;
use super::*;

/// Scenario: Navigating down past the bottom scrolls the viewport to follow the cursor
#[test]
fn down_past_bottom_scrolls_viewport_to_follow_cursor() {
    // @step Given the model selector shows a body viewport 10 rows tall
    let mut v = tall_view();
    render_at(&mut v, 60, 14); // body ≈ 10 list rows after chrome+legend
    let visible = v.visible_rows;
    assert!(visible > 0 && visible < v.rows.len());

    // @step And the list is much longer than the viewport with the cursor at the top
    assert_eq!(v.scroll_offset, 0);

    // @step When I press Down until the selected row would fall below the visible window
    for _ in 0..(visible + 2) {
        v.handle_key(key(KeyCode::Down));
    }

    // @step Then the viewport scrolls down so the selected row becomes the last visible row
    assert_eq!(v.scroll_offset, v.selected_index + 1 - visible);
    // @step And the selected row stays inside the visible window
    assert!(v.selected_index >= v.scroll_offset);
    assert!(v.selected_index < v.scroll_offset + visible);
}

/// Scenario: Navigating back up scrolls the viewport up with the cursor
#[test]
fn up_to_top_scrolls_viewport_back_to_offset_zero() {
    // @step Given the model selector shows a body viewport 10 rows tall
    let mut v = tall_view();
    render_at(&mut v, 60, 14);
    let visible = v.visible_rows;

    // @step And the cursor has been moved down so the viewport is scrolled away from the top
    for _ in 0..(visible + 5) {
        v.handle_key(key(KeyCode::Down));
    }
    assert!(v.scroll_offset > 0);

    // @step When I press Up until the cursor reaches the first row
    let first = crate::components::model_selector_dialog_rows::first_selectable(&v.rows);
    while v.selected_index > first {
        v.handle_key(key(KeyCode::Up));
    }

    // @step Then the viewport scrolls up with the cursor
    assert!(v.selected_index >= v.scroll_offset);
    // @step And the scroll offset returns to 0
    assert_eq!(v.scroll_offset, 0);
}

/// Scenario: Returning to the first model reveals the leading provider header
#[test]
fn returning_to_first_model_reveals_leading_header() {
    // @step Given the model selector has been scrolled down a tall list
    let mut v = tall_view();
    render_at(&mut v, 60, 14);
    let visible = v.visible_rows;
    for _ in 0..(visible + 5) {
        v.handle_key(key(KeyCode::Down));
    }
    assert!(v.scroll_offset > 0, "precondition: viewport scrolled away");
    // Row 0 is the non-selectable provider header.
    assert!(!v.rows[0].selectable, "row 0 must be the provider header");

    // @step When I press Up until the cursor reaches the first selectable model
    let first = crate::components::model_selector_dialog_rows::first_selectable(&v.rows);
    while v.selected_index > first {
        v.handle_key(key(KeyCode::Up));
    }

    // @step Then the scroll offset is 0
    assert_eq!(v.scroll_offset, 0);
    // @step And the provider header row at index 0 is inside the visible window
    assert!(v.visible_rows > 0, "viewport must have at least one row");
    assert!(
        v.scroll_offset < v.scroll_offset + v.visible_rows,
        "row 0 must fall within [offset, offset+visible): offset={}, visible={}",
        v.scroll_offset,
        v.visible_rows
    );
}

/// Scenario: End jumps to the last row and pins it to the bottom edge
#[test]
fn end_pins_last_row_to_bottom_edge() {
    // @step Given the model selector shows a body viewport 10 rows tall
    let mut v = tall_view();
    render_at(&mut v, 60, 14);
    let visible = v.visible_rows;
    let total = v.rows.len();
    assert!(total > visible);

    // @step And the list is taller than the viewport
    // @step When I press End
    v.handle_key(key(KeyCode::End));

    // @step Then the cursor is on the last selectable row
    assert_eq!(
        v.selected_index,
        crate::components::model_selector_dialog_rows::last_selectable(&v.rows)
    );
    // @step And the scroll offset equals total rows minus visible rows
    assert_eq!(v.scroll_offset, total - visible);
    // @step And there are no blank rows rendered after the last row
    assert_eq!(v.scroll_offset + visible, total);
}

/// Scenario: Mouse-wheel navigation scrolls the viewport like the Down key
#[test]
fn wheel_down_scrolls_viewport_like_down_key() {
    use crossterm::event::{MouseEvent, MouseEventKind};

    // @step Given the model selector shows a body viewport 10 rows tall
    let mut v = tall_view();
    render_at(&mut v, 60, 14);
    let visible = v.visible_rows;

    // @step And the list overflows the viewport with the cursor on the last visible row
    // Move down to the bottom edge of the current window (selected ==
    // scroll_offset + visible - 1) without yet scrolling.
    while v.selected_index < v.scroll_offset + visible - 1 {
        v.handle_key(key(KeyCode::Down));
    }
    assert_eq!(v.selected_index, v.scroll_offset + visible - 1);
    let before = v.selected_index;
    let offset_before = v.scroll_offset;

    // @step When I scroll the mouse-wheel down
    let ev = MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 1,
        row: 1,
        modifiers: KeyModifiers::NONE,
    };
    v.handle_mouse(ev);

    // @step Then the selection advances to the next selectable row skipping headers
    assert_eq!(v.selected_index, before + 1);
    assert!(v.rows[v.selected_index].selectable);
    // @step And the viewport scrolls to keep the new selection visible
    assert_eq!(v.scroll_offset, offset_before + 1);
    assert!(v.selected_index < v.scroll_offset + visible);
    assert!(v.selected_index >= v.scroll_offset);
}

/// Scenario: Filtering rebuilds the list and reconciles the scroll offset
#[test]
fn filtering_reconciles_scroll_offset() {
    // @step Given the model selector has been scrolled down a long list
    let mut v = tall_view();
    render_at(&mut v, 60, 14);
    let visible = v.visible_rows;
    v.handle_key(key(KeyCode::End));
    assert!(v.scroll_offset > 0);

    // @step When I type a filter that narrows the results to a few rows
    v.handle_key(key(KeyCode::Char('/')));
    v.handle_key(key(KeyCode::Char('m')));
    v.handle_key(key(KeyCode::Char('1')));

    // @step Then the scroll offset is reconciled so the reset selection is visible
    assert!(v.selected_index >= v.scroll_offset);
    assert!(v.selected_index < v.scroll_offset + visible);
    // @step And there are no blank trailing rows rendered
    let total = v.rows.len();
    assert!(v.scroll_offset <= total.saturating_sub(visible));
}

/// Scenario: A tiny or empty viewport renders gracefully without panic
#[test]
fn tiny_viewport_renders_without_panic() {
    // @step Given the model selector body viewport is only 3 rows tall or the list is empty
    let mut v = tall_view();
    v.handle_key(key(KeyCode::End)); // push selection/offset down first

    // @step When the body is rendered
    render_at(&mut v, 60, 3); // body collapses near zero

    // @step Then the scroll offset is 0
    assert_eq!(v.scroll_offset, 0);
    // @step And the body renders without panic
    // (reaching here without panic satisfies the scenario)
}

/// Scenario: Shrinking the terminal re-clamps the scroll offset on the next paint
#[test]
fn shrinking_terminal_reclamps_offset_on_next_paint() {
    // @step Given the model selector cursor is near the bottom of a tall list
    let mut v = tall_view();
    render_at(&mut v, 60, 14);
    v.handle_key(key(KeyCode::End));
    let total = v.rows.len();

    // @step When the terminal is resized smaller so the body has fewer rows
    render_at(&mut v, 60, 8);
    let visible = v.visible_rows;

    // @step Then on the next paint the scroll offset is re-clamped
    assert_eq!(v.scroll_offset, total - visible);
    // @step And the selected row is still visible
    assert!(v.selected_index >= v.scroll_offset);
    assert!(v.selected_index < v.scroll_offset + visible);
    // @step And there are no blank trailing rows rendered
    assert_eq!(v.scroll_offset + visible, total);
}
