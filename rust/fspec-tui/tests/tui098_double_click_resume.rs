//! Feature: spec/features/double-click-to-resume-session-in-resume-view.feature
//!
//! Moved out of rpc028_popup_scroll.rs (COV-075: one test file per feature).

use codelet_test_helpers::{OrFail, TestResult};
use crossterm::event::{KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

// ---------------------------------------------------------------------
// SlashCommandPopup
// ---------------------------------------------------------------------

// ---------------------------------------------------------------------
// FileSearchPopup
// ---------------------------------------------------------------------

use codelet_fspec_tui::views::agent::{ResumeSessionView, ResumeSessionViewOutcome};
use codelet_rpc_types::SessionInfo;
use crossterm::event::MouseButton;

fn fake_session(id: &str) -> SessionInfo {
    SessionInfo {
        id: id.to_string(),
        name: id.to_string(),
        status: "idle".to_string(),
        project: String::new(),
        message_count: 0,
        provider_id: None,
        model_id: None,
        is_isolated: false,
        worktree_path: None,
        role: None,
        updated_at_ms: None,
        work_unit_focus: Default::default(),
    }
}

fn sessions(n: usize) -> Vec<SessionInfo> {
    (0..n).map(|i| fake_session(&format!("s{i}"))).collect()
}

// ---------------------------------------------------------------------
// Shared helper — scroll_viewport primitives
// ---------------------------------------------------------------------

// ---------------------------------------------------------------------
// TUI-098 — Double-click to resume session
// ---------------------------------------------------------------------

#[test]
fn tui098_double_click_same_row_resumes_session() -> TestResult {
    // @step Given the /resume session picker is open with 20 sessions and visible_rows is 8
    let mut v = ResumeSessionView::new();
    v.set_sessions(sessions(20));
    let visible_rows = 8;
    // @step And the scroll_offset is 0 so rows 0..7 are visible
    assert_eq!(v.scroll_offset(), 0);
    let body_rect = Rect {
        x: 0,
        y: 2,
        width: 60,
        height: visible_rows as u16,
    };
    // @step When the user double-clicks (two left-button-down events within 300ms) on the third visible row
    // First click — should be Continued (single click)
    let outcome1 = v.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 5,
            row: body_rect.y + 2,
            modifiers: KeyModifiers::NONE,
        },
        body_rect,
        visible_rows,
    );
    match outcome1 {
        ResumeSessionViewOutcome::Continued => {}
        other => return Err(format!("expected Continued for first click, got {other:?}").into()),
    }
    // Second click — should be Selected (double click)
    let outcome2 = v.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 5,
            row: body_rect.y + 2,
            modifiers: KeyModifiers::NONE,
        },
        body_rect,
        visible_rows,
    );
    // @step Then the selected_index becomes 2
    assert_eq!(v.selected_index(), 2);
    // @step And the session at index 2 is resumed (ResumeSessionViewOutcome::Selected is emitted)
    match outcome2 {
        ResumeSessionViewOutcome::Selected(id) => {
            assert_eq!(id.to_string(), "s2");
        }
        other => return Err(format!("expected Selected, got {other:?}").into()),
    }
    // @step And the /resume view closes
    // (The caller handles closing the view when Selected is emitted)
    Ok(())
}

#[test]
fn tui098_two_clicks_over_300ms_are_single_clicks() -> TestResult {
    use std::time::Duration;
    // @step Given the /resume session picker is open with 20 sessions and visible_rows is 8
    let mut v = ResumeSessionView::new();
    v.set_sessions(sessions(20));
    let visible_rows = 8;
    // @step And the scroll_offset is 0 with selected_index 0
    assert_eq!(v.scroll_offset(), 0);
    assert_eq!(v.selected_index(), 0);
    let body_rect = Rect {
        x: 0,
        y: 2,
        width: 60,
        height: visible_rows as u16,
    };
    // @step When the user clicks on row 3 and then clicks the same row again after 500ms
    let outcome1 = v.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 5,
            row: body_rect.y + 3,
            modifiers: KeyModifiers::NONE,
        },
        body_rect,
        visible_rows,
    );
    // @step Then the selected_index becomes 3 after the first click
    match outcome1 {
        ResumeSessionViewOutcome::Continued => {}
        other => return Err(format!("expected Continued, got {other:?}").into()),
    }
    assert_eq!(v.selected_index(), 3);
    // TEST-042: the first click is 500ms in the past, without waiting.
    assert!(v.age_last_click_for_test(Duration::from_millis(500)));
    let outcome2 = v.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 5,
            row: body_rect.y + 3,
            modifiers: KeyModifiers::NONE,
        },
        body_rect,
        visible_rows,
    );
    // @step And the selected_index remains 3 after the second click
    assert_eq!(v.selected_index(), 3);
    // @step And no session is resumed (ResumeSessionViewOutcome::Selected is NOT emitted)
    match outcome2 {
        ResumeSessionViewOutcome::Continued => {}
        other => return Err(format!("expected Continued, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn tui098_footer_hint_text_indicates_double_click_resumes_session() -> TestResult {
    use ratatui::buffer::Buffer;
    // @step Given the /resume session picker is open
    let mut v = ResumeSessionView::new();
    // @step When the view renders the footer
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    v.render(area, &mut buf);
    // @step Then the footer displays "DblClick Resume | Enter Select | ↑↓ Navigate | D Delete | Esc Cancel"
    // The footer is on the last row of the buffer
    let footer_y = area.height - 1;
    let footer_text: String = (0..area.width)
        .map(|x| {
            buf.cell((x, footer_y))
                .map(ratatui::buffer::Cell::symbol)
                .or_fail("cell in bounds")
        })
        .collect::<Result<_, _>>()?;
    assert!(
        footer_text.contains("DblClick"),
        "Footer should contain 'DblClick', got: {footer_text}"
    );
    assert!(
        footer_text.contains("Resume"),
        "Footer should contain 'Resume', got: {footer_text}"
    );
    assert!(
        footer_text.contains("Enter Select"),
        "Footer should contain 'Enter Select', got: {footer_text}"
    );
    Ok(())
}

#[test]
fn tui098_quick_clicks_different_rows_are_single_clicks() -> TestResult {
    // @step Given the /resume session picker is open with 20 sessions and visible_rows is 8
    let mut v = ResumeSessionView::new();
    v.set_sessions(sessions(20));
    let visible_rows = 8;
    // @step And the scroll_offset is 0 with selected_index 0
    assert_eq!(v.scroll_offset(), 0);
    assert_eq!(v.selected_index(), 0);
    let body_rect = Rect {
        x: 0,
        y: 2,
        width: 60,
        height: visible_rows as u16,
    };
    // @step When the user clicks on row 2 and then quickly clicks on row 5 within 200ms
    let outcome1 = v.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 5,
            row: body_rect.y + 2,
            modifiers: KeyModifiers::NONE,
        },
        body_rect,
        visible_rows,
    );
    // @step Then the selected_index becomes 2 after the first click
    match outcome1 {
        ResumeSessionViewOutcome::Continued => {}
        other => return Err(format!("expected Continued, got {other:?}").into()),
    }
    assert_eq!(v.selected_index(), 2);
    let outcome2 = v.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 5,
            row: body_rect.y + 5,
            modifiers: KeyModifiers::NONE,
        },
        body_rect,
        visible_rows,
    );
    // @step And the selected_index becomes 5 after the second click
    assert_eq!(v.selected_index(), 5);
    // @step And no session is resumed (ResumeSessionViewOutcome::Selected is NOT emitted)
    match outcome2 {
        ResumeSessionViewOutcome::Continued => {}
        other => return Err(format!("expected Continued, got {other:?}").into()),
    }
    Ok(())
}
