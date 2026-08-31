//! TUI-103 — Popup and full-screen view scrollbar click-and-drag integration tests.
//!
//! Feature: spec/features/popup-and-full-screen-view-scrollbar-click-and-drag-integration.feature
//!
//! Tests ScrollbarDrag integration into SlashCommandPopup, FileSearchPopup,
//! SearchHistoryView, and TurnContentModal mouse handling.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::mouse::scrollbar_drag::{ScrollbarDrag, ScrollbarGeometry};
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

    drag.on_mouse(
        mouse_event(MouseEventKind::Down(MouseButton::Left), 0, 0),
        geom,
    );
    drag.on_mouse(
        mouse_event(MouseEventKind::Drag(MouseButton::Left), 0, 10),
        geom,
    );
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

// =====================================================================
// Integration: SlashCommandPopup handle_mouse routes scrollbar events
// =====================================================================

/// @step Given SlashCommandPopup has more commands than fit in the visible area
/// @step And the scrollbar gutter rect is cached from the last render
/// @step When I click on the scrollbar gutter column
/// @step Then the popup updates its scroll_offset via ScrollbarDrag
#[test]
fn slash_command_popup_handle_mouse_routes_scrollbar_events() {
    use crate::views::agent::slash_command_popup::{PopupOutcome, SlashCommandPopup};

    let mut popup = SlashCommandPopup::new();
    popup.set_matches_for_test(50);
    popup.set_visible_rows_for_test(10);

    // Given: render the popup to cache the scrollbar rect
    let popup_rect = Rect {
        x: 10,
        y: 5,
        width: 40,
        height: 20,
    };
    let mut buf = ratatui::buffer::Buffer::empty(popup_rect);
    popup.render(popup_rect, &mut buf);

    // Use the actual cached scrollbar rect from the render
    let sb_rect = popup
        .last_scrollbar_rect()
        .expect("scrollbar rect should be cached after render");
    let scrollbar_col = sb_rect.x;
    // Click at a row near the bottom of the scrollbar track
    let click_row = sb_rect.y + sb_rect.height - 1;

    // When: click on scrollbar track
    let down_ev = mouse_event(
        MouseEventKind::Down(MouseButton::Left),
        scrollbar_col,
        click_row,
    );
    let down_result = popup.handle_mouse(down_ev, popup_rect);
    assert!(
        matches!(down_result, PopupOutcome::Continued | PopupOutcome::Ignored),
        "Down event should not produce a selection outcome"
    );

    let up_ev = mouse_event(
        MouseEventKind::Up(MouseButton::Left),
        scrollbar_col,
        click_row,
    );
    let up_result = popup.handle_mouse(up_ev, popup_rect);

    // Then: scroll offset should have changed (not 0)
    assert!(
        popup.scroll_offset() > 0,
        "scroll offset should have changed from 0, got {}",
        popup.scroll_offset()
    );
    assert!(
        matches!(up_result, PopupOutcome::Continued),
        "Up event on scrollbar should return Continued"
    );
}

// =====================================================================
// Integration: FileSearchPopup handle_mouse routes scrollbar events
// =====================================================================

/// @step Given FileSearchPopup has more files than fit in the visible area
/// @step And the scrollbar gutter rect is cached from the last render
/// @step When I drag the scrollbar thumb downward
/// @step Then the popup updates its scroll_offset via ScrollbarDrag
#[test]
fn file_search_popup_handle_mouse_routes_scrollbar_drag() {
    use crate::views::agent::file_search_popup::{FilePopupOutcome, FileSearchPopup};

    let mut popup = FileSearchPopup::new(0, "test");
    popup.set_matches(vec!["file.txt".to_string(); 100]);
    popup.set_visible_rows_for_test(10);

    let popup_rect = Rect {
        x: 10,
        y: 5,
        width: 40,
        height: 20,
    };
    // Render to cache the scrollbar rect
    let mut buf = ratatui::buffer::Buffer::empty(popup_rect);
    popup.render(popup_rect, &mut buf);

    // Use the actual cached scrollbar rect from the render
    let sb_rect = popup
        .last_scrollbar_rect()
        .expect("scrollbar rect should be cached after render");
    let scrollbar_col = sb_rect.x;

    // When: press at the top of the scrollbar rect
    let down_ev = mouse_event(
        MouseEventKind::Down(MouseButton::Left),
        scrollbar_col,
        sb_rect.y,
    );
    let down_result = popup.handle_mouse(down_ev, popup_rect);
    assert!(
        matches!(
            down_result,
            FilePopupOutcome::Continued | FilePopupOutcome::Ignored
        ),
        "Down event should not produce a selection outcome"
    );

    // And: drag to the middle of the scrollbar rect
    let drag_row = sb_rect.y + sb_rect.height / 2;
    let drag_ev = mouse_event(
        MouseEventKind::Drag(MouseButton::Left),
        scrollbar_col,
        drag_row,
    );
    let drag_result = popup.handle_mouse(drag_ev, popup_rect);

    // Then: scroll offset should have changed
    assert!(
        popup.scroll_offset() > 0,
        "scroll offset should have changed from 0, got {}",
        popup.scroll_offset()
    );
    assert!(
        matches!(drag_result, FilePopupOutcome::Continued),
        "Drag event on scrollbar should return Continued"
    );
}

// =====================================================================
// Integration: SearchHistoryView handle_mouse routes scrollbar events
// =====================================================================

/// @step Given SearchHistoryView has more matches than fit in the visible area
/// @step And the scrollbar gutter rect is cached from the last render
/// @step When I click on the scrollbar track
/// @step Then the view updates its scroll_offset via ScrollbarDrag
#[test]
fn search_history_view_handle_mouse_routes_scrollbar_events() {
    use crate::views::agent::search_history_view::SearchHistoryView;
    use crate::views::agent::search_history_view::SearchHistoryViewOutcome;

    let mut view = SearchHistoryView::new();
    // Set up matches for testing
    view.set_matches(
        (0..50)
            .map(|i| codelet_rpc_types::HistoryMatch {
                text: format!("match text {i}"),
                timestamp_iso: "2026-01-01T00:00:00Z".to_string(),
                session_id: codelet_rpc_types::SessionId::new("session-1"),
            })
            .collect(),
    );

    let body_rect = Rect {
        x: 0,
        y: 2,
        width: 80,
        height: 20,
    };
    let visible_rows = SearchHistoryView::visible_rows_for(body_rect);

    // Render to cache the scrollbar rect
    let mut buf = ratatui::buffer::Buffer::empty(body_rect);
    view.render(body_rect, &mut buf);

    // Use the actual cached scrollbar rect from the render
    let sb_rect = view
        .last_scrollbar_rect()
        .expect("scrollbar rect should be cached after render");
    let scrollbar_col = sb_rect.x;

    // When: click on scrollbar track near the bottom
    let click_row = sb_rect.y + sb_rect.height - 2;
    let down_ev = mouse_event(
        MouseEventKind::Down(MouseButton::Left),
        scrollbar_col,
        click_row,
    );
    let down_result = view.handle_mouse(down_ev, body_rect, visible_rows);
    assert!(
        matches!(
            down_result,
            SearchHistoryViewOutcome::Continued | SearchHistoryViewOutcome::Ignored
        ),
        "Down event should not produce a selection outcome"
    );

    let up_ev = mouse_event(
        MouseEventKind::Up(MouseButton::Left),
        scrollbar_col,
        click_row,
    );
    let up_result = view.handle_mouse(up_ev, body_rect, visible_rows);

    // Then: scroll offset should have changed
    assert!(
        view.scroll_offset() > 0,
        "scroll offset should have changed from 0, got {}",
        view.scroll_offset()
    );
    assert!(
        matches!(up_result, SearchHistoryViewOutcome::Continued),
        "Up event on scrollbar should return Continued"
    );
}
