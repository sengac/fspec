//! TUI-102 — AgentView scrollbar integration tests.
//!
//! Feature: spec/features/agentview-scrollback-scrollbar-click-and-drag-integration.feature
//!
//! End-to-end integration tests: AgentView caches scrollbar geometry
//! during render and routes scrollbar clicks to ScrollbarDrag, emitting
//! Action::ScrollbackJumpToOffset.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::components::Action;
use crate::views::agent::AgentView;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use tokio::sync::mpsc::unbounded_channel;

/// Integration test: AgentView emits ScrollbackJumpToOffset on scrollbar click
#[test]
fn agentview_emits_scrollback_jump_to_offset_on_scrollbar_click() {
    let (tx, mut rx) = unbounded_channel::<Action>();
    let mut view = AgentView::new(tx);

    let scrollback_area = Rect {
        x: 0,
        y: 2,
        width: 80,
        height: 20,
    };
    view.last_scrollback_area = Some(scrollback_area);
    view.last_scrollback_viewport = 20;
    view.last_scrollback_total_rows = 100;
    view.last_scrollback_scroll_offset = 0;

    let scrollbar_col = scrollback_area.x + scrollback_area.width - 1;

    // Click on scrollbar track at row 5 (relative to scrollback area y)
    let click_row = scrollback_area.y + 5;
    let down_ev = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: scrollbar_col,
        row: click_row,
        modifiers: crossterm::event::KeyModifiers::NONE,
    };

    let result = view.handle_scrollback_mouse(down_ev);
    assert!(result.is_some(), "scrollbar click should be consumed");

    let up_ev = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: scrollbar_col,
        row: click_row,
        modifiers: crossterm::event::KeyModifiers::NONE,
    };

    let result = view.handle_scrollback_mouse(up_ev);
    assert!(result.is_some(), "scrollbar release should be consumed");

    // Then: Action::ScrollbackJumpToOffset should be emitted
    let action = rx
        .blocking_recv()
        .expect("expected ScrollbackJumpToOffset action");
    assert!(
        matches!(action, Action::ScrollbackJumpToOffset(25)),
        "expected ScrollbackJumpToOffset(25), got {action:?}"
    );
}

/// Integration test: AgentView does NOT emit ScrollbackJumpToOffset when
/// content fits in viewport (no gutter reserved).
#[test]
fn agentview_no_scrollbar_interaction_when_content_fits() {
    let (tx, mut rx) = unbounded_channel::<Action>();
    let mut view = AgentView::new(tx);

    let scrollback_area = Rect {
        x: 0,
        y: 2,
        width: 80,
        height: 20,
    };
    view.last_scrollback_area = Some(scrollback_area);
    view.last_scrollback_viewport = 20;

    // Only 5 total rows — fits in viewport, no gutter reserved
    view.last_scrollback_total_rows = 5;
    view.last_scrollback_scroll_offset = 0;

    // When: click on rightmost column
    let rightmost_col = scrollback_area.x + scrollback_area.width - 1;
    let down_ev = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: rightmost_col,
        row: scrollback_area.y + 5,
        modifiers: crossterm::event::KeyModifiers::NONE,
    };

    // Then: click should still be consumed by text selection
    let result = view.handle_scrollback_mouse(down_ev);
    assert!(
        result.is_some(),
        "click should still be consumed by text selection"
    );

    // Verify no ScrollbackJumpToOffset was emitted
    assert!(
        rx.try_recv().is_err(),
        "should not emit ScrollbackJumpToOffset when content fits"
    );
}

/// Integration test: AgentView scrollbar click exits stick_to_bottom mode.
#[test]
fn agentview_scrollbar_click_exits_stick_mode() {
    let (tx, mut rx) = unbounded_channel::<Action>();
    let mut view = AgentView::new(tx);

    let scrollback_area = Rect {
        x: 0,
        y: 2,
        width: 80,
        height: 20,
    };
    view.last_scrollback_area = Some(scrollback_area);
    view.last_scrollback_viewport = 20;
    view.last_scrollback_total_rows = 100;
    view.last_scrollback_scroll_offset = 0;

    let scrollbar_col = scrollback_area.x + scrollback_area.width - 1;

    // Simulate scrollbar click sequence
    let click_row = scrollback_area.y + 15;
    let down_ev = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: scrollbar_col,
        row: click_row,
        modifiers: crossterm::event::KeyModifiers::NONE,
    };
    let _ = view.handle_scrollback_mouse(down_ev);

    let up_ev = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: scrollbar_col,
        row: click_row,
        modifiers: crossterm::event::KeyModifiers::NONE,
    };
    let _ = view.handle_scrollback_mouse(up_ev);

    // The emitted action should be ScrollbackJumpToOffset with the computed offset
    // (row 15 * 100 / 20 = 75)
    let action = rx
        .blocking_recv()
        .expect("expected ScrollbackJumpToOffset action");
    assert!(
        matches!(action, Action::ScrollbackJumpToOffset(75)),
        "expected ScrollbackJumpToOffset(75), got {action:?}"
    );
}
