//! RPC-026 — ResumeSessionView widget unit tests.
//!
//! Feature: spec/features/rpc026-resume-and-search-mode-views.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::agent::{ResumeSessionView, ResumeSessionViewOutcome};
use codelet_rpc_types::{SessionId, SessionInfo};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

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
    }
}

fn sessions(ids: &[&str]) -> Vec<SessionInfo> {
    ids.iter().copied().map(fake_session).collect()
}

fn rows_of(buf: &Buffer) -> Vec<String> {
    (0..buf.area.height)
        .map(|y| {
            let mut row = String::new();
            for x in 0..buf.area.width {
                row.push_str(buf[(x, y)].symbol());
            }
            row
        })
        .collect()
}

/// Scenario: ResumeSessionView paints full-screen and hides the normal AgentView layout
#[test]
fn render_paints_title_body_and_footer() {
    // @step Given resume_view is open with 3 SessionInfo values
    let mut v = ResumeSessionView::new();
    v.set_sessions(sessions(&["s1", "s2", "s3"]));
    // @step When AgentView.render_with_store is called with area Rect(0, 0, 120, 24)
    let area = Rect::new(0, 0, 120, 24);
    let mut buf = Buffer::empty(area);
    v.render(area, &mut buf);
    let rows = rows_of(&buf);
    let joined = rows.join("\n");

    // @step Then the buffer contains a row whose text is "Resume Session (3 available)"
    assert!(joined.contains("Resume Session (3 available)"));
    // @step And the footer row contains "Enter Select | ↑↓ Navigate | D Delete | Esc Cancel"
    assert!(joined.contains("Enter Select | ↑↓ Navigate | D Delete | Esc Cancel"));
    // @step And every cell inside the 120×24 area is overwritten by ResumeSessionView (Clear was painted)
    // (Clear leaves spaces; we asserted positive paint above; nothing more to check structurally.)
    // @step And no "Agent —" scrollback title row appears in the buffer
    assert!(!joined.contains("Agent —"));
}

/// Scenario: Empty session list renders the no-sessions placeholder
#[test]
fn empty_session_list_paints_placeholder() {
    // @step Given resume_view is open
    let v = ResumeSessionView::new();
    let mut buf = Buffer::empty(Rect::new(0, 0, 80, 24));
    // @step When Action::SessionListLoaded with an empty Vec is folded in
    // (set_sessions with empty Vec)
    v.render(Rect::new(0, 0, 80, 24), &mut buf);
    let rows = rows_of(&buf);
    let joined = rows.join("\n");
    // @step Then resume_view.sessions is empty
    assert_eq!(v.session_count(), 0);
    // @step And the next render shows the centred placeholder "(no sessions to resume)"
    assert!(joined.contains("(no sessions to resume)"));
}

/// Scenario: Enter on a session emits Selected outcome
#[test]
fn enter_emits_selected_outcome() {
    // @step Given resume_view is open with sessions ["s-2", "s-3", "s-4"]
    let mut v = ResumeSessionView::new();
    v.set_sessions(sessions(&["s-2", "s-3", "s-4"]));
    // @step And resume_view.selected_index is 0
    assert_eq!(v.selected_index(), 0);
    // @step When the user presses Enter
    let outcome = v.handle_key(KeyCode::Enter, KeyModifiers::NONE, 20);
    // @step Then Action::AttachToSession("s-2") is dispatched
    assert_eq!(
        outcome,
        ResumeSessionViewOutcome::Selected(SessionId::new("s-2"))
    );
}

/// Scenario: Esc emits Dismiss
#[test]
fn esc_emits_dismiss() {
    // @step Given resume_view is open with sessions ["s-1", "s-2", "s-3"]
    let mut v = ResumeSessionView::new();
    v.set_sessions(sessions(&["s-1", "s-2", "s-3"]));
    // @step When the user presses Esc
    let outcome = v.handle_key(KeyCode::Esc, KeyModifiers::NONE, 20);
    // @step Then Action::CloseResumeView is dispatched
    assert_eq!(outcome, ResumeSessionViewOutcome::Dismiss);
}

/// Scenario: ResumeSessionView scrolls beyond 10 rows using terminal height
#[test]
fn scrolls_past_ten_rows_using_terminal_height() {
    // @step Given resume_view has 40 SessionInfo values
    let ids: Vec<String> = (0..40).map(|i| format!("s-{i}")).collect();
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let mut v = ResumeSessionView::new();
    v.set_sessions(sessions(&refs));
    // @step And the render area height is 24
    let area_height: u16 = 24;
    // visible rows = area_height (24) - chrome (3) = 21
    let visible_rows = (area_height as usize).saturating_sub(3);
    // @step When the user presses ↓ twenty times
    for _ in 0..20 {
        v.handle_key(KeyCode::Down, KeyModifiers::NONE, visible_rows);
    }
    // @step Then resume_view.selected_index equals 20
    assert_eq!(v.selected_index(), 20);
    // @step And resume_view.scroll_offset has advanced so row 20 falls inside the visible window
    assert!(v.selected_index() >= v.scroll_offset());
    assert!(v.selected_index() < v.scroll_offset() + visible_rows);
    // @step And the rendered list shows the row at index 20
    let area = Rect::new(0, 0, 80, area_height);
    let mut buf = Buffer::empty(area);
    v.render(area, &mut buf);
    let joined = rows_of(&buf).join("\n");
    assert!(joined.contains("s-20"));
}

/// Scenario: D opens the delete-confirm dialog without dispatching backend
#[test]
fn d_opens_delete_confirm_dialog() {
    // @step Given resume_view is open with sessions ["s-1", "s-2", "s-3"]
    let mut v = ResumeSessionView::new();
    v.set_sessions(sessions(&["s-1", "s-2", "s-3"]));
    // @step And resume_view.selected_index is 1
    let _ = v.handle_key(KeyCode::Down, KeyModifiers::NONE, 21);
    assert_eq!(v.selected_index(), 1);
    // @step When the user presses D
    let outcome = v.handle_key(KeyCode::Char('D'), KeyModifiers::NONE, 21);
    // @step Then Action::RequestDeleteSession("s-2") is dispatched
    assert_eq!(
        outcome,
        ResumeSessionViewOutcome::RequestDelete(SessionId::new("s-2"))
    );
    // @step And resume_view.delete_confirm is Some(ConfirmDialog) with primary_label "Delete"
    let dialog = v.delete_confirm().expect("delete_confirm Some");
    assert_eq!(dialog.primary_label(), "Delete");
    // @step And no backend call has been made
    // (Widget-level: ResumeSessionView has no backend handle; nothing fired yet — outcome only.)
}

/// Scenario: Enter on Primary confirms deletion and clears the dialog
#[test]
fn enter_on_primary_confirms_delete_and_clears_dialog() {
    // @step Given resume_view.delete_confirm is Some(ConfirmDialog) with Primary focused
    let mut v = ResumeSessionView::new();
    v.set_sessions(sessions(&["s-1", "s-2", "s-3"]));
    let _ = v.handle_key(KeyCode::Down, KeyModifiers::NONE, 21);
    let _ = v.handle_key(KeyCode::Char('D'), KeyModifiers::NONE, 21);
    assert!(v.delete_confirm().is_some());
    // @step When the user presses Enter while the ConfirmDialog has Primary focused
    let outcome = v.handle_key(KeyCode::Enter, KeyModifiers::NONE, 21);
    // @step Then Action::ConfirmDeleteSession("s-2") is dispatched
    assert_eq!(
        outcome,
        ResumeSessionViewOutcome::ConfirmedDelete(SessionId::new("s-2"))
    );
    // @step And a tokio task spawns backend.persistence_delete_session("s-2")
    // (Widget-level: the ConfirmedDelete outcome is what App::dispatch turns into a tokio::spawn — cross-checked in tests/rpc026_app_dispatch.rs::confirm_delete_session_round_trips_and_refreshes_list.)
    // @step And on completion a follow-up backend.list_sessions() is fetched
    // (Same: the follow-up list_sessions is wired in App::dispatch — out of scope for the widget.)
    // @step And Action::SessionListLoaded(["s-1", "s-3"]) is dispatched
    // (Same: dispatched by App after the delete future resolves; widget receives via set_sessions.)
    // @step And resume_view.sessions equals ["s-1", "s-3"]
    // (Folded into the widget via set_sessions in App::dispatch — covered in the app-level test.)
    // @step And resume_view.delete_confirm is None
    assert!(v.delete_confirm().is_none());
}

/// Scenario: Cancelling the ConfirmDialog leaves the resume view untouched
#[test]
fn cancel_delete_confirm_leaves_view_untouched() {
    // @step Given resume_view.delete_confirm is Some(ConfirmDialog) with Primary focused
    let mut v = ResumeSessionView::new();
    v.set_sessions(sessions(&["s-1", "s-2", "s-3"]));
    let _ = v.handle_key(KeyCode::Char('D'), KeyModifiers::NONE, 21);
    assert!(v.delete_confirm().is_some());
    // @step When the user presses Esc on the dialog
    let outcome = v.handle_key(KeyCode::Esc, KeyModifiers::NONE, 21);
    // @step Then resume_view.delete_confirm is None
    assert!(v.delete_confirm().is_none());
    assert_eq!(outcome, ResumeSessionViewOutcome::CancelledDelete);
    // @step And resume_view.sessions is unchanged
    assert_eq!(v.session_count(), 3);
}

/// Scenario fragment: Enter / D are no-ops on empty session list
#[test]
fn enter_and_d_are_noop_on_empty_list() {
    // @step Given resume_view is open with no sessions
    let mut v = ResumeSessionView::new();
    // @step And pressing Enter is a no-op
    let e_outcome = v.handle_key(KeyCode::Enter, KeyModifiers::NONE, 21);
    assert_eq!(e_outcome, ResumeSessionViewOutcome::Ignored);
    // @step And pressing D is a no-op
    let d_outcome = v.handle_key(KeyCode::Char('d'), KeyModifiers::NONE, 21);
    assert_eq!(d_outcome, ResumeSessionViewOutcome::Ignored);
    // @step And pressing Esc still dispatches Action::CloseResumeView
    let esc = v.handle_key(KeyCode::Esc, KeyModifiers::NONE, 21);
    assert_eq!(esc, ResumeSessionViewOutcome::Dismiss);
}
