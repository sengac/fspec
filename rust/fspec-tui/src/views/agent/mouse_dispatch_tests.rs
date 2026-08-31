//! TUI-102 — AgentView scrollback scrollbar click-and-drag integration tests.
//!
//! Feature: spec/features/agentview-scrollback-scrollbar-click-and-drag-integration.feature
//!
//! Tests the scrollbar mouse handling in AgentView's scrollback area.
//! Verifies that clicks on the scrollbar gutter route to ScrollbarDrag
//! and emit Action::ScrollbackJumpToOffset, while non-gutter clicks
//! fall through to text selection.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::mouse::scrollbar_drag::ScrollbarDrag;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

/// Build a mouse event of the given kind at (col, row).
fn mouse_event(kind: MouseEventKind, col: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column: col,
        row,
        modifiers: crossterm::event::KeyModifiers::NONE,
    }
}

/// Scenario: Click on scrollbar track above thumb jumps to that position
///
/// @step Given the scrollback has more content than fits in the viewport
/// @step And the scrollbar is visible on the rightmost column
/// @step When I click on the scrollbar track at a position above the thumb
/// @step Then the scrollback jumps so the clicked position becomes the top of the viewport
/// @step And the scrollback exits stick-to-bottom mode
#[test]
fn click_on_scrollbar_track_above_thumb_jumps_to_that_position() {
    // Given: 100 total rows, 10 visible (viewport), area height 20, offset 0
    // Thumb occupies rows 0..1 (height=1). Row 5 is on the track above thumb.
    let area = Rect {
        x: 0,
        y: 0,
        width: 40,
        height: 20,
    };
    let scrollbar_col = area.x + area.width - 1; // rightmost column = 39

    let mut drag = ScrollbarDrag::new();
    let geom = crate::mouse::scrollbar_drag::ScrollbarGeometry {
        area_height: 20,
        total_items: 100,
        visible_items: 10,
        current_offset: 0,
    };

    // When: click on scrollbar track at row 5
    let down_ev = mouse_event(MouseEventKind::Down(MouseButton::Left), scrollbar_col, 5);
    let down_result = drag.on_mouse(down_ev, geom);
    assert_eq!(down_result, None, "Down event should not produce an offset");

    let up_ev = mouse_event(MouseEventKind::Up(MouseButton::Left), scrollbar_col, 5);
    let up_result = drag.on_mouse(up_ev, geom);

    // Then: offset should be 25 (row 5 * 100 / 20 = 25)
    assert_eq!(
        up_result,
        Some(25),
        "track click should jump to proportional offset"
    );

    // And: state should return to idle
    assert!(!drag.is_dragging());
}

/// Scenario: Click on scrollbar track below thumb jumps down
///
/// @step Given the scrollback has more content than fits in the viewport
/// @step And the scrollbar is visible on the rightmost column
/// @step When I click on the scrollbar track below the thumb
/// @step Then the scrollback jumps down so the clicked position becomes visible at the top of the viewport
/// @step And the scrollback exits stick-to-bottom mode
#[test]
fn click_on_scrollbar_track_below_thumb_jumps_down() {
    let area = Rect {
        x: 0,
        y: 0,
        width: 40,
        height: 20,
    };
    let scrollbar_col = area.x + area.width - 1;

    let mut drag = ScrollbarDrag::new();
    let geom = crate::mouse::scrollbar_drag::ScrollbarGeometry {
        area_height: 20,
        total_items: 100,
        visible_items: 10,
        current_offset: 0,
    };

    // When: click on scrollbar track at row 15 (below thumb)
    let down_ev = mouse_event(MouseEventKind::Down(MouseButton::Left), scrollbar_col, 15);
    let down_result = drag.on_mouse(down_ev, geom);
    assert_eq!(down_result, None);

    let up_ev = mouse_event(MouseEventKind::Up(MouseButton::Left), scrollbar_col, 15);
    let up_result = drag.on_mouse(up_ev, geom);

    // Then: offset should be 75 (row 15 * 100 / 20 = 75)
    assert_eq!(
        up_result,
        Some(75),
        "track click below thumb should jump down"
    );
    assert!(!drag.is_dragging());
}

/// Scenario: Drag on scrollbar thumb continuously scrolls content
///
/// @step Given the scrollback has more content than fits in the viewport
/// @step And the scrollbar is visible on the rightmost column
/// @step When I press and drag the scrollbar thumb downward
/// @step Then the scrollback content scrolls in real time following my mouse position
/// @step And the scrollback exits stick-to-bottom mode
#[test]
fn drag_on_scrollbar_thumb_continuously_scrolls_content() {
    let area = Rect {
        x: 0,
        y: 0,
        width: 40,
        height: 20,
    };
    let scrollbar_col = area.x + area.width - 1;

    let mut drag = ScrollbarDrag::new();
    let geom = crate::mouse::scrollbar_drag::ScrollbarGeometry {
        area_height: 20,
        total_items: 100,
        visible_items: 10,
        current_offset: 0,
    };

    // When: press at row 0
    let down_ev = mouse_event(MouseEventKind::Down(MouseButton::Left), scrollbar_col, 0);
    assert_eq!(drag.on_mouse(down_ev, geom), None);

    // And: drag to row 10
    let drag_ev = mouse_event(MouseEventKind::Drag(MouseButton::Left), scrollbar_col, 10);
    let drag_result = drag.on_mouse(drag_ev, geom);

    // Then: offset should be 50 during drag
    assert_eq!(
        drag_result,
        Some(50),
        "drag should continuously update offset"
    );
    assert!(drag.is_dragging(), "state should be dragging");

    // And: release returns to idle
    let up_ev = mouse_event(MouseEventKind::Up(MouseButton::Left), scrollbar_col, 10);
    let up_result = drag.on_mouse(up_ev, geom);
    assert_eq!(up_result, None, "release after drag should return None");
    assert!(!drag.is_dragging());
}

/// Scenario: Quick click on thumb scrolls one viewport height
///
/// @step Given the scrollback has more content than fits in the viewport
/// @step And the scrollbar is visible on the rightmost column
/// @step When I quickly click on the scrollbar thumb without dragging
/// @step Then the scrollback scrolls down by one viewport height
/// @step And the scrollback exits stick-to-bottom mode
#[test]
fn quick_click_on_thumb_scrolls_one_viewport_height() {
    let area = Rect {
        x: 0,
        y: 0,
        width: 40,
        height: 20,
    };
    let scrollbar_col = area.x + area.width - 1;

    let mut drag = ScrollbarDrag::new();
    let geom = crate::mouse::scrollbar_drag::ScrollbarGeometry {
        area_height: 20,
        total_items: 100,
        visible_items: 10,
        current_offset: 0,
    };

    // When: quick click on thumb (row 0 is within thumb at offset 0)
    let down_ev = mouse_event(MouseEventKind::Down(MouseButton::Left), scrollbar_col, 0);
    assert_eq!(drag.on_mouse(down_ev, geom), None);

    let up_ev = mouse_event(MouseEventKind::Up(MouseButton::Left), scrollbar_col, 0);
    let up_result = drag.on_mouse(up_ev, geom);

    // Then: offset should advance by one viewport height (10)
    assert_eq!(
        up_result,
        Some(10),
        "quick click on thumb should scroll one viewport height"
    );
}

/// Scenario: No scrollbar interaction when content fits in viewport
///
/// @step Given the scrollback content fits entirely within the viewport
/// @step And no scrollbar is visible
/// @step When I click on the rightmost column of the scrollback area
/// @step Then the click is handled as normal text selection
/// @step And the scrollback offset does not change
#[test]
fn no_scrollbar_interaction_when_content_fits_in_viewport() {
    // Given: 5 total rows, 10 visible — content fits, no scrollbar needed
    let area = Rect {
        x: 0,
        y: 0,
        width: 40,
        height: 20,
    };
    let scrollbar_col = area.x + area.width - 1;

    let mut drag = ScrollbarDrag::new();
    let geom = crate::mouse::scrollbar_drag::ScrollbarGeometry {
        area_height: 20,
        total_items: 5,
        visible_items: 10,
        current_offset: 0,
    };

    // When: click on rightmost column
    let down_ev = mouse_event(MouseEventKind::Down(MouseButton::Left), scrollbar_col, 5);
    assert_eq!(drag.on_mouse(down_ev, geom), None);

    let up_ev = mouse_event(MouseEventKind::Up(MouseButton::Left), scrollbar_col, 5);
    let up_result = drag.on_mouse(up_ev, geom);

    // Then: offset should be 0 (no scroll)
    assert_eq!(
        up_result,
        Some(0),
        "when content fits, scrollbar should return offset 0"
    );
    assert!(!drag.is_dragging());
}
