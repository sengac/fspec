//! TUI-103 — Popup and full-screen view scrollbar click-and-drag integration tests.
//!
//! Feature: spec/features/popup-and-full-screen-view-scrollbar-click-and-drag-integration.feature
//!
//! Tests ScrollbarDrag integration into SlashCommandPopup, FileSearchPopup,
//! SearchHistoryView, and TurnContentModal mouse handling.
//!
//! The pure state-machine scenarios (track-click jumps, thumb drag, quick
//! click, fit-viewport, content-change reset) live in
//! `crate::mouse::scrollbar_drag::tests` (relocated to keep this file
//! under the 300-LoC ceiling).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crossterm::event::{MouseButton, MouseEventKind};
use ratatui::layout::Rect;

/// Build a mouse event of the given kind at (col, row).
fn mouse_event(kind: MouseEventKind, col: u16, row: u16) -> crossterm::event::MouseEvent {
    crossterm::event::MouseEvent {
        kind,
        column: col,
        row,
        modifiers: crossterm::event::KeyModifiers::NONE,
    }
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
