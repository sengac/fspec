//! RPC-026 — Widget tests for ResumePicker.
//!
//! Feature: spec/features/rpc026-resume-picker.feature
//!
//! Exercises the standalone ResumePicker widget surface — set_sessions,
//! handle_key navigation, Enter / Esc / Tab outcomes, and the rendered
//! placeholder / list bodies. App-level wiring (dispatch into
//! AgentViewStore) lives in `app_dispatch_resume_search_rpc026.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::agent::resume_picker::{ResumePicker, ResumePickerOutcome};
use codelet_rpc_types::{SessionId, SessionInfo};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

fn session_info(id: &str, name: &str) -> SessionInfo {
    SessionInfo {
        id: id.to_string(),
        name: name.to_string(),
        status: "idle".to_string(),
        project: "/tmp/parity".to_string(),
        message_count: 0,
        provider_id: None,
        model_id: None,
        is_isolated: false,
        worktree_path: None,
        role: None,
    }
}

fn render_to_string(p: &ResumePicker) -> String {
    let mut buf = Buffer::empty(Rect::new(0, 0, 80, 24));
    p.render(buf.area, &mut buf);
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

/// Scenario: A new ResumePicker has no sessions and selected_index == 0
#[test]
fn new_resume_picker_has_no_sessions_and_zero_index() {
    // @step Given a fresh ResumePicker
    let p = ResumePicker::new();
    // @step Then resume_picker.session_count() equals 0
    assert_eq!(p.session_count(), 0);
    // @step And resume_picker.selected_index() equals 0
    assert_eq!(p.selected_index(), 0);
    // @step And resume_picker.selected() returns None
    assert!(p.selected().is_none());
}

/// Scenario: set_sessions populates the rows and resets selection to the first row
#[test]
fn set_sessions_populates_rows_and_resets_selection() {
    // @step Given a fresh ResumePicker
    let mut p = ResumePicker::new();
    // @step When resume_picker.set_sessions is called with three SessionInfos in order ["s-1", "s-2", "s-3"]
    p.set_sessions(vec![
        session_info("s-1", "first"),
        session_info("s-2", "second"),
        session_info("s-3", "third"),
    ]);
    // @step Then resume_picker.session_count() equals 3
    assert_eq!(p.session_count(), 3);
    // @step And resume_picker.selected_index() equals 0
    assert_eq!(p.selected_index(), 0);
    // @step And resume_picker.selected() returns Some(SessionInfo with id "s-1")
    assert_eq!(p.selected().expect("selected").id, "s-1");
}

/// Scenario: Down arrow advances selection and wraps around at the end
#[test]
fn down_arrow_advances_and_wraps() {
    // @step Given a ResumePicker populated with three sessions ["s-1", "s-2", "s-3"]
    let mut p = ResumePicker::new();
    p.set_sessions(vec![
        session_info("s-1", "first"),
        session_info("s-2", "second"),
        session_info("s-3", "third"),
    ]);
    // @step When the user presses Down
    p.handle_key(KeyCode::Down, KeyModifiers::NONE);
    // @step Then resume_picker.selected_index() equals 1
    assert_eq!(p.selected_index(), 1);
    // @step When the user presses Down
    p.handle_key(KeyCode::Down, KeyModifiers::NONE);
    // @step Then resume_picker.selected_index() equals 2
    assert_eq!(p.selected_index(), 2);
    // @step When the user presses Down
    p.handle_key(KeyCode::Down, KeyModifiers::NONE);
    // @step Then resume_picker.selected_index() equals 0
    assert_eq!(p.selected_index(), 0);
}

/// Scenario: Up arrow walks backward and wraps to the last row
#[test]
fn up_arrow_walks_backward_and_wraps() {
    // @step Given a ResumePicker populated with three sessions ["s-1", "s-2", "s-3"]
    let mut p = ResumePicker::new();
    p.set_sessions(vec![
        session_info("s-1", "first"),
        session_info("s-2", "second"),
        session_info("s-3", "third"),
    ]);
    // @step When the user presses Up
    p.handle_key(KeyCode::Up, KeyModifiers::NONE);
    // @step Then resume_picker.selected_index() equals 2
    assert_eq!(p.selected_index(), 2);
    // @step When the user presses Up
    p.handle_key(KeyCode::Up, KeyModifiers::NONE);
    // @step Then resume_picker.selected_index() equals 1
    assert_eq!(p.selected_index(), 1);
}

/// Scenario: Enter on a highlighted row emits Selected with the SessionId
#[test]
fn enter_emits_selected_with_session_id() {
    // @step Given a ResumePicker populated with three sessions ["s-1", "s-2", "s-3"]
    let mut p = ResumePicker::new();
    p.set_sessions(vec![
        session_info("s-1", "first"),
        session_info("s-2", "second"),
        session_info("s-3", "third"),
    ]);
    // @step When the user presses Down
    p.handle_key(KeyCode::Down, KeyModifiers::NONE);
    // @step And the user presses Enter
    let outcome = p.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    // @step Then handle_key returns ResumePickerOutcome::Selected(SessionId("s-2"))
    match outcome {
        ResumePickerOutcome::Selected(id) => assert_eq!(id, SessionId::new("s-2")),
        other => panic!("expected Selected(s-2), got {other:?}"),
    }
}

/// Scenario: Enter on an empty session list is ignored
#[test]
fn enter_on_empty_list_is_ignored() {
    // @step Given a fresh ResumePicker with zero sessions
    let mut p = ResumePicker::new();
    // @step When the user presses Enter
    let outcome = p.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    // @step Then handle_key returns ResumePickerOutcome::Ignored
    assert!(matches!(outcome, ResumePickerOutcome::Ignored));
}

/// Scenario: Esc on the popup returns Dismiss
#[test]
fn esc_returns_dismiss() {
    // @step Given a ResumePicker populated with one session ["s-1"]
    let mut p = ResumePicker::new();
    p.set_sessions(vec![session_info("s-1", "first")]);
    // @step When the user presses Esc
    let outcome = p.handle_key(KeyCode::Esc, KeyModifiers::NONE);
    // @step Then handle_key returns ResumePickerOutcome::Dismiss
    assert!(matches!(outcome, ResumePickerOutcome::Dismiss));
}

/// Scenario: Tab is ignored by the resume picker
#[test]
fn tab_is_ignored() {
    // @step Given a ResumePicker populated with one session ["s-1"]
    let mut p = ResumePicker::new();
    p.set_sessions(vec![session_info("s-1", "first")]);
    // @step When the user presses Tab
    let outcome = p.handle_key(KeyCode::Tab, KeyModifiers::NONE);
    // @step Then handle_key returns ResumePickerOutcome::Ignored
    assert!(matches!(outcome, ResumePickerOutcome::Ignored));
}

/// Scenario: Modifier-prefixed keys are propagated so AgentView can route Shift+arrow chords
#[test]
fn shift_arrow_is_propagated_as_ignored() {
    // @step Given a ResumePicker populated with two sessions ["s-1", "s-2"]
    let mut p = ResumePicker::new();
    p.set_sessions(vec![
        session_info("s-1", "first"),
        session_info("s-2", "second"),
    ]);
    // @step When the user presses Shift+Down
    let outcome = p.handle_key(KeyCode::Down, KeyModifiers::SHIFT);
    // @step Then handle_key returns ResumePickerOutcome::Ignored
    assert!(matches!(outcome, ResumePickerOutcome::Ignored));
    // @step And resume_picker.selected_index() is unchanged at 0
    assert_eq!(p.selected_index(), 0);
}

/// Scenario: Empty session list renders the "(no sessions to resume)" placeholder
#[test]
fn empty_list_renders_placeholder() {
    // @step Given a fresh ResumePicker with zero sessions
    let p = ResumePicker::new();
    // @step When the popup is rendered
    let painted = render_to_string(&p);
    // @step Then the rendered body contains the literal string "(no sessions to resume)"
    assert!(
        painted.contains("(no sessions to resume)"),
        "missing placeholder in rendered body:\n{painted}"
    );
}

/// Scenario: Populated session list renders one row per SessionInfo
#[test]
fn populated_list_renders_one_row_per_session() {
    // @step Given a ResumePicker populated with two sessions [SessionInfo("s-1", "first"), SessionInfo("s-2", "second")]
    let mut p = ResumePicker::new();
    p.set_sessions(vec![
        session_info("s-1", "first"),
        session_info("s-2", "second"),
    ]);
    // @step When the popup is rendered
    let painted = render_to_string(&p);
    // @step Then the rendered body contains a row referencing "s-1"
    assert!(painted.contains("s-1"), "missing s-1 in:\n{painted}");
    // @step And the rendered body contains a row referencing "s-2"
    assert!(painted.contains("s-2"), "missing s-2 in:\n{painted}");
    // @step And the rendered body contains the navigation hint "↑↓ Navigate │ Enter Attach │ Esc Close"
    assert!(
        painted.contains("Navigate") && painted.contains("Enter") && painted.contains("Esc"),
        "missing nav hint:\n{painted}"
    );
}
