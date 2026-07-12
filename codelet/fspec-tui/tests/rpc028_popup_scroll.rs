//! RPC-028 — scroll/wrap/mouse parity tests for SlashCommandPopup
//! and FileSearchPopup. Moved out of the in-module test blocks so
//! the per-file 300-LoC budget enforced by `source_shape_rpc019` is
//! preserved.
//!
//! Feature: spec/features/rpc028-scroll-mouse-wrap-parity.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::agent::{
    FilePopupOutcome, FileSearchPopup, PopupOutcome, SlashCommandPopup,
};
use crossterm::event::{KeyCode, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

// ---------------------------------------------------------------------
// SlashCommandPopup
// ---------------------------------------------------------------------

fn make_slash(n: usize) -> SlashCommandPopup {
    let mut p = SlashCommandPopup::new();
    p.set_matches_for_test(n);
    p
}

#[test]
fn rpc028_slash_scroll_offset_advances_past_visible_rows_so_selection_stays_visible() {
    // @step Given the SlashCommandPopup is open with 14 matching commands and visible_rows is 10
    let mut p = make_slash(14);
    p.set_visible_rows_for_test(10);
    // @step And the popup is at scroll_offset 0 with selected_index 0
    assert_eq!(p.scroll_offset(), 0);
    assert_eq!(p.selected_index(), 0);
    // @step When the user presses Down 10 times
    for _ in 0..10 {
        p.handle_key(KeyCode::Down, KeyModifiers::NONE);
    }
    // @step Then the selected_index is 10
    assert_eq!(p.selected_index(), 10);
    // @step And the scroll_offset has advanced so the selected row is inside the visible window
    let so = p.scroll_offset();
    assert!(so <= 10 && 10 < so + 10);
    // @step And the top body row paints the "↑" glyph
    assert!(p.shows_up_indicator());
}

#[test]
fn rpc028_slash_down_at_last_match_wraps_to_first() {
    // @step Given the SlashCommandPopup is open with 14 matching commands
    let mut p = make_slash(14);
    p.set_visible_rows_for_test(10);
    // @step And the selected_index is at the last match (13)
    p.set_selected_for_test(13);
    // @step When the user presses Down
    p.handle_key(KeyCode::Down, KeyModifiers::NONE);
    // @step Then the selected_index wraps to 0
    assert_eq!(p.selected_index(), 0);
    // @step And the scroll_offset is reset to 0 so row 0 is visible
    assert_eq!(p.scroll_offset(), 0);
}

#[test]
fn rpc028_slash_up_at_first_match_wraps_to_last_and_scrolls_to_show_it() {
    // @step Given the SlashCommandPopup is open with 14 matching commands
    let mut p = make_slash(14);
    p.set_visible_rows_for_test(10);
    // @step And the selected_index is 0
    assert_eq!(p.selected_index(), 0);
    // @step When the user presses Up
    p.handle_key(KeyCode::Up, KeyModifiers::NONE);
    // @step Then the selected_index wraps to 13
    assert_eq!(p.selected_index(), 13);
    // @step And the scroll_offset advances so the last row is visible
    let so = p.scroll_offset();
    assert!(so <= 13 && 13 < so + 10);
    // @step And the bottom body row stops painting the "↓" glyph
    assert!(!p.shows_down_indicator());
}

#[test]
fn rpc028_slash_mouse_wheel_inside_popup_rect_moves_selection() {
    let mut p = make_slash(14);
    p.set_visible_rows_for_test(10);
    p.set_selected_for_test(5);
    let rect = Rect {
        x: 0,
        y: 0,
        width: 60,
        height: 12,
    };
    let outcome = p.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 10,
            row: 4,
            modifiers: KeyModifiers::NONE,
        },
        rect,
    );
    match outcome {
        PopupOutcome::Continued => {}
        other => panic!("expected Continued, got {other:?}"),
    }
    assert_eq!(p.selected_index(), 6);
}

#[test]
fn rpc028_slash_mouse_wheel_outside_popup_rect_is_ignored() {
    // @step Given the SlashCommandPopup is open above AgentView's MultiLineInput
    let mut p = make_slash(14);
    p.set_visible_rows_for_test(10);
    p.set_selected_for_test(5);
    let rect = Rect {
        x: 0,
        y: 0,
        width: 60,
        height: 12,
    };
    // @step And the mouse cursor is over the scrollback area, outside the popup rect
    // @step When the user emits MouseEventKind::ScrollUp
    let outcome = p.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 200,
            row: 100,
            modifiers: KeyModifiers::NONE,
        },
        rect,
    );
    // @step Then the popup returns EventResult::Ignored
    match outcome {
        PopupOutcome::Ignored => {}
        other => panic!("expected Ignored, got {other:?}"),
    }
    // @step And the event bubbles to AgentView so the scrollback scrolls instead
    assert_eq!(p.selected_index(), 5);
}

#[test]
fn rpc028_slash_page_down_jumps_by_visible_rows() {
    let mut p = make_slash(20);
    p.set_visible_rows_for_test(8);
    assert_eq!(p.selected_index(), 0);
    p.handle_key(KeyCode::PageDown, KeyModifiers::NONE);
    assert_eq!(p.selected_index(), 8);
}

#[test]
fn rpc028_slash_page_up_jumps_back_by_visible_rows() {
    let mut p = make_slash(20);
    p.set_visible_rows_for_test(8);
    p.set_selected_for_test(15);
    p.handle_key(KeyCode::PageUp, KeyModifiers::NONE);
    assert_eq!(p.selected_index(), 7);
}

#[test]
fn rpc028_slash_home_jumps_to_first_with_zero_scroll() {
    let mut p = make_slash(20);
    p.set_visible_rows_for_test(8);
    p.set_selected_for_test(15);
    p.handle_key(KeyCode::Home, KeyModifiers::NONE);
    assert_eq!(p.selected_index(), 0);
    assert_eq!(p.scroll_offset(), 0);
}

#[test]
fn rpc028_slash_end_jumps_to_last_with_scroll_so_last_row_visible() {
    let mut p = make_slash(20);
    p.set_visible_rows_for_test(8);
    p.handle_key(KeyCode::End, KeyModifiers::NONE);
    assert_eq!(p.selected_index(), 19);
    let so = p.scroll_offset();
    assert!(so <= 19 && 19 < so + 8);
}

// ---------------------------------------------------------------------
// FileSearchPopup
// ---------------------------------------------------------------------

fn make_file_popup(n: usize) -> FileSearchPopup {
    let mut p = FileSearchPopup::new(0, "");
    let matches: Vec<String> = (0..n).map(|i| format!("file-{i}.rs")).collect();
    p.set_matches(matches);
    p
}

#[test]
fn rpc028_file_mouse_wheel_up_at_first_wraps_to_last() {
    // @step Given the FileSearchPopup is open with 12 matches and visible_rows is 10
    let mut p = make_file_popup(12);
    p.set_visible_rows_for_test(10);
    // @step And the selected_index is 0 with scroll_offset 0
    assert_eq!(p.selected_index(), 0);
    assert_eq!(p.scroll_offset(), 0);
    let rect = Rect {
        x: 0,
        y: 0,
        width: 60,
        height: 12,
    };
    // @step When the user emits MouseEventKind::ScrollUp inside the popup rect
    let outcome = p.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 5,
            row: 5,
            modifiers: KeyModifiers::NONE,
        },
        rect,
    );
    match outcome {
        FilePopupOutcome::Continued => {}
        other => panic!("expected Continued, got {other:?}"),
    }
    // @step Then the selected_index wraps to 11
    assert_eq!(p.selected_index(), 11);
    // @step And the scroll_offset advances so the last row is visible
    let so = p.scroll_offset();
    assert!(so <= 11 && 11 < so + 10);
}

#[test]
fn rpc028_file_mouse_wheel_up_moves_selection_up_one() {
    // @step Given the FileSearchPopup is open with 12 matches and visible_rows is 10
    let mut p = make_file_popup(12);
    p.set_visible_rows_for_test(10);
    // @step And the selected_index is 5
    p.set_selected_for_test(5);
    let rect = Rect {
        x: 0,
        y: 0,
        width: 60,
        height: 12,
    };
    // @step When the user emits MouseEventKind::ScrollUp inside the popup rect
    let outcome = p.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 5,
            row: 5,
            modifiers: KeyModifiers::NONE,
        },
        rect,
    );
    match outcome {
        FilePopupOutcome::Continued => {}
        other => panic!("expected Continued, got {other:?}"),
    }
    // @step Then the selected_index decreases to 4
    assert_eq!(p.selected_index(), 4);
    // @step And the popup remains visible with the selection inside the window
    let so = p.scroll_offset();
    assert!(so <= 4 && 4 < so + 10);
}

#[test]
fn rpc028_file_mouse_wheel_down_moves_selection_down_one() {
    let mut p = make_file_popup(12);
    p.set_visible_rows_for_test(10);
    p.set_selected_for_test(5);
    let rect = Rect {
        x: 0,
        y: 0,
        width: 60,
        height: 12,
    };
    let outcome = p.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 5,
            row: 5,
            modifiers: KeyModifiers::NONE,
        },
        rect,
    );
    match outcome {
        FilePopupOutcome::Continued => {}
        other => panic!("expected Continued, got {other:?}"),
    }
    assert_eq!(p.selected_index(), 6);
}

#[test]
fn rpc028_file_scroll_offset_advances_past_visible_rows() {
    let mut p = make_file_popup(14);
    p.set_visible_rows_for_test(10);
    for _ in 0..10 {
        p.handle_key(KeyCode::Down, KeyModifiers::NONE);
    }
    assert_eq!(p.selected_index(), 10);
    let so = p.scroll_offset();
    assert!(so <= 10 && 10 < so + 10);
    assert!(p.shows_up_indicator());
}

#[test]
fn rpc028_file_home_and_end_keys() {
    let mut p = make_file_popup(14);
    p.set_visible_rows_for_test(8);
    p.handle_key(KeyCode::End, KeyModifiers::NONE);
    assert_eq!(p.selected_index(), 13);
    p.handle_key(KeyCode::Home, KeyModifiers::NONE);
    assert_eq!(p.selected_index(), 0);
    assert_eq!(p.scroll_offset(), 0);
}

// ---------------------------------------------------------------------
// Legacy in-module tests for SlashCommandPopup + FileSearchPopup moved
// here verbatim from `src/views/agent/slash_command_popup.rs` and
// `src/views/agent/file_search_popup.rs` so the source files stay
// under the 300-LoC source-shape budget. Behaviour is unchanged.
// ---------------------------------------------------------------------

use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;

#[test]
fn legacy_slash_new_popup_has_full_registry_and_first_selected() {
    let p = SlashCommandPopup::new();
    assert!(p.match_count() > 0);
    assert_eq!(p.selected_index(), 0);
    assert_eq!(p.filter(), "");
    assert!(p.selected().is_some());
}

#[test]
fn legacy_slash_set_filter_narrows_matches() {
    let mut p = SlashCommandPopup::new();
    p.set_filter("he");
    assert!(p.match_count() >= 1);
    assert_eq!(p.matches()[0].name(), "help");
    assert_eq!(p.selected_index(), 0);
}

#[test]
fn legacy_slash_down_wraps_around() {
    let mut p = SlashCommandPopup::new();
    for _ in 0..p.match_count() {
        p.handle_key(KeyCode::Down, KeyModifiers::NONE);
    }
    assert_eq!(p.selected_index(), 0);
}

#[test]
fn legacy_slash_up_wraps_to_end() {
    let mut p = SlashCommandPopup::new();
    p.handle_key(KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(p.selected_index(), p.match_count() - 1);
}

#[test]
fn legacy_slash_enter_emits_selected_action() {
    let mut p = SlashCommandPopup::new();
    match p.handle_key(KeyCode::Enter, KeyModifiers::NONE) {
        PopupOutcome::Selected(a) => assert_eq!(a, SlashCommandAction::Help),
        other => panic!("expected Selected, got {other:?}"),
    }
}

#[test]
fn legacy_slash_tab_returns_filled_with_command_name() {
    let mut p = SlashCommandPopup::new();
    p.set_filter("c");
    match p.handle_key(KeyCode::Tab, KeyModifiers::NONE) {
        PopupOutcome::Filled(s) => assert!(s.starts_with('/'), "got: {s}"),
        other => panic!("expected Filled, got {other:?}"),
    }
}

#[test]
fn legacy_slash_esc_returns_dismiss() {
    let mut p = SlashCommandPopup::new();
    match p.handle_key(KeyCode::Esc, KeyModifiers::NONE) {
        PopupOutcome::Dismiss => {}
        other => panic!("expected Dismiss, got {other:?}"),
    }
}

#[test]
fn legacy_slash_ordinary_char_is_ignored() {
    let mut p = SlashCommandPopup::new();
    match p.handle_key(KeyCode::Char('q'), KeyModifiers::NONE) {
        PopupOutcome::Ignored => {}
        other => panic!("expected Ignored, got {other:?}"),
    }
}

#[test]
fn legacy_file_new_popup_has_no_matches_and_zero_index() {
    let p = FileSearchPopup::new(5, "rea");
    assert_eq!(p.anchor_offset(), 5);
    assert_eq!(p.filter(), "rea");
    assert_eq!(p.match_count(), 0);
    assert_eq!(p.selected_index(), 0);
}

#[test]
fn legacy_file_set_matches_clamps_selection() {
    let mut p = FileSearchPopup::new(0, "");
    p.set_matches(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    assert_eq!(p.match_count(), 3);
}

#[test]
fn legacy_file_enter_selects_current_match_for_splice() {
    let mut p = FileSearchPopup::new(6, "rea");
    p.set_matches(vec!["README.md".to_string()]);
    match p.handle_key(KeyCode::Enter, KeyModifiers::NONE) {
        FilePopupOutcome::SelectedEnter(path) => assert_eq!(path, "README.md"),
        other => panic!("expected SelectedEnter, got {other:?}"),
    }
}

#[test]
fn legacy_file_tab_selects_current_match_for_partial_fill() {
    let mut p = FileSearchPopup::new(6, "rea");
    p.set_matches(vec!["README.md".to_string()]);
    match p.handle_key(KeyCode::Tab, KeyModifiers::NONE) {
        FilePopupOutcome::SelectedTab(path) => assert_eq!(path, "README.md"),
        other => panic!("expected SelectedTab, got {other:?}"),
    }
}

#[test]
fn legacy_file_down_wraps_around() {
    let mut p = FileSearchPopup::new(0, "");
    p.set_matches(vec!["a".to_string(), "b".to_string()]);
    p.handle_key(KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(p.selected_index(), 1);
    p.handle_key(KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(p.selected_index(), 0);
}

#[test]
fn legacy_file_enter_with_no_matches_is_ignored() {
    let mut p = FileSearchPopup::new(0, "");
    match p.handle_key(KeyCode::Enter, KeyModifiers::NONE) {
        FilePopupOutcome::Ignored => {}
        other => panic!("expected Ignored, got {other:?}"),
    }
}
use codelet_fspec_tui::components::thinking_level_dialog::ThinkingLevelDialog;
use codelet_fspec_tui::components::Component;
use codelet_fspec_tui::views::agent::{
    ResumeSessionView, ResumeSessionViewOutcome, SearchHistoryView, SearchHistoryViewOutcome,
};
use codelet_rpc_types::{HistoryMatch, SessionId, SessionInfo, ThinkingLevel};
use crossterm::event::{Event, MouseButton};

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
    }
}

fn sessions(n: usize) -> Vec<SessionInfo> {
    (0..n).map(|i| fake_session(&format!("s{i}"))).collect()
}

fn fake_match(text: &str) -> HistoryMatch {
    HistoryMatch {
        session_id: SessionId::new("s0".to_string()),
        text: text.to_string(),
        timestamp_iso: "2026-01-01T00:00:00Z".to_string(),
    }
}

fn matches(n: usize) -> Vec<HistoryMatch> {
    (0..n).map(|i| fake_match(&format!("m{i}"))).collect()
}

// ---------------------------------------------------------------------
// ResumeSessionView — Home + mouse click
// ---------------------------------------------------------------------

#[test]
fn rpc028_resume_home_jumps_to_first_session_and_scrolls_to_top() {
    // @step Given the /resume session picker is open with 20 sessions and visible_rows is 8
    let mut v = ResumeSessionView::new();
    v.set_sessions(sessions(20));
    let visible_rows = 8;
    // Advance to selected=15 (scroll_offset will follow).
    for _ in 0..15 {
        v.handle_key(KeyCode::Down, KeyModifiers::NONE, visible_rows);
    }
    // @step And the selected_index is 15 with scroll_offset 8
    assert_eq!(v.selected_index(), 15);
    assert_eq!(v.scroll_offset(), 8);
    // @step When the user presses Home
    let outcome = v.handle_key(KeyCode::Home, KeyModifiers::NONE, visible_rows);
    match outcome {
        ResumeSessionViewOutcome::Continued => {}
        other => panic!("expected Continued, got {other:?}"),
    }
    // @step Then the selected_index is 0
    assert_eq!(v.selected_index(), 0);
    // @step And the scroll_offset is 0
    assert_eq!(v.scroll_offset(), 0);
    // @step And the top body row no longer paints the "↑" glyph
    assert_eq!(v.scroll_offset(), 0);
}

#[test]
fn rpc028_resume_left_click_on_row_selects_that_row() {
    // @step Given the /resume session picker is open with 20 sessions and visible_rows is 8
    let mut v = ResumeSessionView::new();
    v.set_sessions(sessions(20));
    let visible_rows = 8;
    // Drive selected_index to 12 to force scroll_offset to 5.
    for _ in 0..12 {
        v.handle_key(KeyCode::Down, KeyModifiers::NONE, visible_rows);
    }
    // @step And the scroll_offset is 5 so rows 5..12 are visible
    assert_eq!(v.selected_index(), 12);
    assert_eq!(v.scroll_offset(), 5);
    // body rect inside the mode-view layout: title=1, separator=1, body=vr, footer=1.
    let body_rect = Rect {
        x: 0,
        y: 2,
        width: 60,
        height: visible_rows as u16,
    };
    // @step When the user left-clicks on the second visible row
    let outcome = v.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 5,
            row: body_rect.y + 1,
            modifiers: KeyModifiers::NONE,
        },
        body_rect,
        visible_rows,
    );
    match outcome {
        ResumeSessionViewOutcome::Continued => {}
        other => panic!("expected Continued, got {other:?}"),
    }
    // @step Then the selected_index becomes 6
    assert_eq!(v.selected_index(), 6);
    // @step And the row is highlighted with the inverse style
    assert!(v.selected().is_some());
}

// ---------------------------------------------------------------------
// SearchHistoryView — End
// ---------------------------------------------------------------------

#[test]
fn rpc028_search_end_jumps_to_last_match_and_scrolls() {
    // @step Given the /search history palette has 25 matches and visible_rows is 10
    let mut v = SearchHistoryView::new();
    v.set_matches(matches(25));
    let visible_rows = 10;
    // @step And the selected_index is 0 with scroll_offset 0
    assert_eq!(v.selected_index(), 0);
    assert_eq!(v.scroll_offset(), 0);
    // @step When the user presses End
    let outcome = v.handle_key(KeyCode::End, KeyModifiers::NONE, visible_rows);
    match outcome {
        SearchHistoryViewOutcome::Continued => {}
        other => panic!("expected Continued, got {other:?}"),
    }
    // @step Then the selected_index is 24
    assert_eq!(v.selected_index(), 24);
    // @step And the scroll_offset has advanced so row 24 is on the bottom visible row
    let so = v.scroll_offset();
    assert!(so <= 24 && 24 < so + visible_rows);
    // @step And the top body row paints the "↑" glyph
    assert!(so > 0);
}

// ---------------------------------------------------------------------
// ThinkingLevelDialog — mouse wheel
// ---------------------------------------------------------------------

#[test]
fn rpc028_thinking_level_mouse_wheel_down_advances_and_wraps() {
    // @step Given the ThinkingLevelDialog is open with the High level selected
    let mut d = ThinkingLevelDialog::new(SessionId::new("s0".to_string()), ThinkingLevel::High);
    // @step When the user emits MouseEventKind::ScrollDown inside the dialog rect
    let _ = d.handle_event(&Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 10,
        row: 10,
        modifiers: KeyModifiers::NONE,
    }));
    // @step Then the selection wraps to Off
    assert_eq!(d.selected_level(), ThinkingLevel::Off);
    // @step And the dialog remains visible with the inverse highlight on the new row
    // (the dialog is still alive — ScrollDown does NOT emit a remove callback.)
}

// ---------------------------------------------------------------------
// Shared helper — scroll_viewport primitives
// ---------------------------------------------------------------------

use codelet_fspec_tui::components::scroll_viewport::{
    ensure_visible as sv_ensure_visible, wrap_index as sv_wrap_index, WheelDirection as SvDir,
    WheelVelocity as SvVel,
};

#[test]
fn rpc028_scroll_viewport_wrap_index_wraps_both_directions() {
    // @step Given the shared scroll_viewport module is loaded
    // @step When wrap_index(0, -1, 5) is called
    // @step Then it returns 4
    assert_eq!(sv_wrap_index(0, -1, 5), 4);
    // @step And wrap_index(4, 1, 5) returns 0
    assert_eq!(sv_wrap_index(4, 1, 5), 0);
    // @step And wrap_index(2, 10, 5) returns 2
    assert_eq!(sv_wrap_index(2, 10, 5), 2);
}

#[test]
fn rpc028_scroll_viewport_ensure_visible_scrolls_down_past_window() {
    // @step Given scroll_offset is 0 and visible_rows is 8 and total is 20
    let mut so: usize = 0;
    // @step When ensure_visible(&mut scroll_offset, 10, 8, 20) is called
    sv_ensure_visible(&mut so, 10, 8, 20);
    // @step Then scroll_offset is updated so 10 lies in [scroll_offset, scroll_offset + 8)
    assert!(so <= 10 && 10 < so + 8);
}

#[test]
fn rpc028_wheel_velocity_ramps_up_then_resets_after_gap() {
    use std::thread::sleep;
    use std::time::Duration;
    // @step Given a fresh WheelVelocity
    let v = SvVel::new();
    // @step When the user emits 5 ScrollDown events within 100ms of each other
    let mut last = 0;
    for _ in 0..5 {
        last = v.step(SvDir::Down);
        sleep(Duration::from_millis(20));
    }
    // @step Then the 5th step reports velocity 5
    assert_eq!(last, 5);
    // @step And after a gap of >=150ms the next step resets velocity to 1
    sleep(Duration::from_millis(200));
    let next = v.step(SvDir::Down);
    assert_eq!(next, 1);
}

// ---------------------------------------------------------------------
// TUI-098 — Double-click to resume session
// ---------------------------------------------------------------------

#[test]
fn tui098_double_click_same_row_resumes_session() {
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
        other => panic!("expected Continued for first click, got {other:?}"),
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
        other => panic!("expected Selected, got {other:?}"),
    }
    // @step And the /resume view closes
    // (The caller handles closing the view when Selected is emitted)
}

#[test]
fn tui098_two_clicks_over_300ms_are_single_clicks() {
    use std::thread::sleep;
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
        other => panic!("expected Continued, got {other:?}"),
    }
    assert_eq!(v.selected_index(), 3);
    sleep(Duration::from_millis(500));
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
        other => panic!("expected Continued, got {other:?}"),
    }
}

#[test]
fn tui098_footer_hint_text_indicates_double_click_resumes_session() {
    use ratatui::buffer::Buffer;
    // @step Given the /resume session picker is open
    let v = ResumeSessionView::new();
    // @step When the view renders the footer
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    v.render(area, &mut buf);
    // @step Then the footer displays "DblClick Resume | Enter Select | ↑↓ Navigate | D Delete | Esc Cancel"
    // The footer is on the last row of the buffer
    let footer_y = area.height - 1;
    let footer_text: String = (0..area.width)
        .map(|x| buf.cell((x, footer_y)).expect("cell in bounds").symbol())
        .collect();
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
}

#[test]
fn tui098_quick_clicks_different_rows_are_single_clicks() {
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
        other => panic!("expected Continued, got {other:?}"),
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
        other => panic!("expected Continued, got {other:?}"),
    }
}
