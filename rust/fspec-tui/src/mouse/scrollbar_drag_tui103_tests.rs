//! TUI-103 — popup / full-screen-view scrollbar click-and-drag integration
//! tests for [`super::ScrollbarDrag`].
//!
//! Split out of `scrollbar_drag.rs` so that file stays under the 300-LoC
//! ceiling pinned by `tests/source_shape_rpc023.rs` (the TUI-103 scenario
//! pins originally lived here; only the module moved).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{ScrollbarDrag, ScrollbarGeometry};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

/// Build a mouse event of the given kind at (col, row) (TUI-103 helpers).
fn mouse_event(kind: MouseEventKind, col: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column: col,
        row,
        modifiers: crossterm::event::KeyModifiers::NONE,
    }
}

// =====================================================================
// Scenario: Click on scrollbar track in SlashCommandPopup jumps to that position
// =====================================================================

/// @step Given the slash command popup is open with more commands than fit in the visible area
/// @step And the scrollbar gutter is reserved on the rightmost column
/// @step When I click the left mouse button on the scrollbar track below the thumb
/// @step Then the popup scroll offset jumps to the position corresponding to the click
/// @step And the popup continues to display the newly visible commands
#[test]
fn slash_command_popup_click_on_scrollbar_track_jumps_to_position() {
    // Given: 50 commands, 10 visible, area height 20, offset 0
    let mut drag = ScrollbarDrag::new();
    let geom = ScrollbarGeometry {
        area_height: 20,
        total_items: 50,
        visible_items: 10,
        current_offset: 0,
    };

    // When: click on scrollbar track at row 15 (below thumb)
    let down = mouse_event(MouseEventKind::Down(MouseButton::Left), 0, 15);
    assert_eq!(drag.on_mouse(down, geom), None);

    let up = mouse_event(MouseEventKind::Up(MouseButton::Left), 0, 15);
    let result = drag.on_mouse(up, geom);

    // Then: offset should be 37 (row 15 * 50 / 20 = 37)
    assert_eq!(
        result,
        Some(37),
        "track click should jump to proportional offset"
    );
    assert!(!drag.is_dragging());
}

// =====================================================================
// Scenario: Drag scrollbar thumb in FileSearchPopup continuously scrolls content
// =====================================================================

/// @step Given the file search popup is open with more files than fit in the visible area
/// @step And the scrollbar gutter is reserved on the rightmost column
/// @step When I press the left mouse button on the scrollbar thumb
/// @step And I drag the mouse downward
/// @step Then the file list scrolls in real time following the mouse position
/// @step And releasing the mouse button stops the drag
#[test]
fn file_search_popup_drag_scrollbar_thumb_continuously_scrolls() {
    // Given: 100 files, 10 visible, area height 20, offset 0
    let mut drag = ScrollbarDrag::new();
    let geom = ScrollbarGeometry {
        area_height: 20,
        total_items: 100,
        visible_items: 10,
        current_offset: 0,
    };

    // When: press at row 0 (on thumb)
    let down = mouse_event(MouseEventKind::Down(MouseButton::Left), 0, 0);
    assert_eq!(drag.on_mouse(down, geom), None);

    // And: drag to row 10
    let drag_ev = mouse_event(MouseEventKind::Drag(MouseButton::Left), 0, 10);
    let result = drag.on_mouse(drag_ev, geom);

    // Then: offset should be 50 during drag
    assert_eq!(result, Some(50), "drag should continuously update offset");
    assert!(drag.is_dragging(), "state should be dragging");

    // And: release returns to idle
    let up = mouse_event(MouseEventKind::Up(MouseButton::Left), 0, 10);
    assert_eq!(
        drag.on_mouse(up, geom),
        None,
        "release after drag should return None"
    );
    assert!(!drag.is_dragging());
}

// =====================================================================
// Scenario: Quick click on scrollbar thumb in SearchHistoryView scrolls one viewport height
// =====================================================================

/// @step Given the search history view is open with more matches than fit in the visible area
/// @step And the scrollbar gutter is reserved on the rightmost column
/// @step When I quickly click and release the left mouse button on the scrollbar thumb
/// @step Then the match list scrolls down by one viewport height
#[test]
fn search_history_view_quick_click_on_thumb_scrolls_one_viewport_height() {
    // Given: 50 matches, 10 visible, area height 20, offset 0
    let mut drag = ScrollbarDrag::new();
    let geom = ScrollbarGeometry {
        area_height: 20,
        total_items: 50,
        visible_items: 10,
        current_offset: 0,
    };

    // When: quick click on thumb (row 0 is within thumb at offset 0)
    let down = mouse_event(MouseEventKind::Down(MouseButton::Left), 0, 0);
    assert_eq!(drag.on_mouse(down, geom), None);

    let up = mouse_event(MouseEventKind::Up(MouseButton::Left), 0, 0);
    let result = drag.on_mouse(up, geom);

    // Then: offset should advance by one viewport height (10)
    assert_eq!(
        result,
        Some(10),
        "quick click on thumb should scroll one viewport height"
    );
    assert!(!drag.is_dragging());
}

// =====================================================================
// Scenario: Click on scrollbar track in TurnContentModal jumps to that position
// =====================================================================

/// @step Given a turn content modal is open with more lines than fit in the visible area
/// @step And the scrollbar gutter is reserved on the rightmost column
/// @step When I click the left mouse button on the scrollbar track near the bottom
/// @step Then the modal jumps to show content near the bottom of the turn
#[test]
fn turn_content_modal_click_on_scrollbar_track_jumps_to_position() {
    // Given: 200 lines, 15 visible, area height 15, offset 0
    let mut drag = ScrollbarDrag::new();
    let geom = ScrollbarGeometry {
        area_height: 15,
        total_items: 200,
        visible_items: 15,
        current_offset: 0,
    };

    // When: click on scrollbar track near the bottom (row 13)
    let down = mouse_event(MouseEventKind::Down(MouseButton::Left), 0, 13);
    assert_eq!(drag.on_mouse(down, geom), None);

    let up = mouse_event(MouseEventKind::Up(MouseButton::Left), 0, 13);
    let result = drag.on_mouse(up, geom);

    // Then: offset should be 173 (row 13 * 200 / 15 = 173, clamped to max 185)
    assert_eq!(
        result,
        Some(173),
        "track click near bottom should jump to proportional offset"
    );
    assert!(!drag.is_dragging());
}

// =====================================================================
// Scenario: Scrollbar interaction is ignored when content fits in viewport
// =====================================================================

/// @step Given a popup is open with fewer items than fit in the visible area
/// @step And no scrollbar gutter is reserved
/// @step When I click the left mouse button on the scrollbar area
/// @step Then the scroll offset remains unchanged
#[test]
fn scrollbar_ignored_when_content_fits_in_viewport() {
    // Given: 5 items, 10 visible — content fits, no scrollbar needed
    let mut drag = ScrollbarDrag::new();
    let geom = ScrollbarGeometry {
        area_height: 20,
        total_items: 5,
        visible_items: 10,
        current_offset: 0,
    };

    // When: click on scrollbar area
    let down = mouse_event(MouseEventKind::Down(MouseButton::Left), 0, 5);
    assert_eq!(drag.on_mouse(down, geom), None);

    let up = mouse_event(MouseEventKind::Up(MouseButton::Left), 0, 5);
    let result = drag.on_mouse(up, geom);

    // Then: offset should be 0 (no scroll)
    assert_eq!(
        result,
        Some(0),
        "when content fits, scrollbar should return offset 0"
    );
    assert!(!drag.is_dragging());
}

// =====================================================================
// Scenario: Scrollbar drag state resets when popup content changes
// =====================================================================

/// @step Given a popup is open and I am in the middle of a scrollbar drag
/// @step When the popup match list is replaced with new content
/// @step Then the drag state is reset to idle
/// @step And subsequent mouse drag events are ignored
#[test]
fn scrollbar_drag_state_resets_when_content_changes() {
    // Given: in the middle of a drag
    let mut drag = ScrollbarDrag::new();
    let geom = ScrollbarGeometry {
        area_height: 20,
        total_items: 100,
        visible_items: 10,
        current_offset: 0,
    };

    drag.on_mouse(mouse_event(MouseEventKind::Down(MouseButton::Left), 0, 0), geom);
    drag.on_mouse(mouse_event(MouseEventKind::Drag(MouseButton::Left), 0, 10), geom);
    assert!(drag.is_dragging());

    // When: content changes → reset
    drag.reset();

    // Then: state should be idle
    assert!(!drag.is_dragging());

    // And: subsequent drag events are ignored
    let result = drag.on_mouse(
        mouse_event(MouseEventKind::Drag(MouseButton::Left), 0, 5),
        geom,
    );
    assert_eq!(result, None, "drag after reset should return None");
}
