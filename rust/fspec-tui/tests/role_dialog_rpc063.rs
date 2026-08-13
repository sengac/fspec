//! RPC-063 — RoleDialog component unit tests.
//!
//! Feature: spec/features/role-slash-command-end-to-end-ui-dialog.feature
//!
//! Drives the Priority::Foreground modal dialog through its public
//! Component surface: `priority()`, `id()`, `render()`, `handle_event()`,
//! and the test-only `take_pending_action()` / `draft()` accessors.
//! Mirrors the existing `ThinkingLevelDialog` component tests.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use codelet_fspec_tui::{
    Action, Component, Compositor, EventResult, Priority, RoleDialog, ROLE_DIALOG_ID,
};
use codelet_rpc_types::SessionId;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn key_with_mods(code: KeyCode, mods: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, mods))
}

/// Render `dialog` against an 80x24 TestBackend and return the buffer
/// as a single \n-delimited String for substring assertions.
fn render_to_string(dialog: &mut RoleDialog) -> String {
    let backend = TestBackend::new(80, 24);
    let mut term = Terminal::new(backend).expect("Terminal::new");
    term.draw(|frame| {
        Component::render(dialog, frame.area(), frame.buffer_mut());
    })
    .expect("draw");
    let buf: Buffer = term.backend().buffer().clone();
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

/// Scenario: RoleDialog renders at Priority::Foreground with the canonical id and title
#[test]
fn role_dialog_renders_at_priority_foreground_with_canonical_id_and_title() {
    // @step Given a fresh RoleDialog with session_id SessionId("s-1") and seed_role = None
    let mut dialog = RoleDialog::new(SessionId::new("s-1"), None);

    // @step When its priority() method is invoked
    let prio = dialog.priority();
    // @step Then the result is Priority::Foreground
    assert_eq!(prio, Priority::Foreground);
    // @step And its id() method returns "role-dialog"
    assert_eq!(dialog.id(), ROLE_DIALOG_ID);
    assert_eq!(dialog.id(), "role-dialog");

    // @step When the dialog is rendered onto an 80x24 TestBackend
    let painted = render_to_string(&mut dialog);
    // @step Then the rendered buffer contains the substring "Role"
    assert!(
        painted.contains("Role"),
        "expected `Role` title row, got:\n{painted}"
    );
    // @step And the rendered buffer contains the footer substring "Enter Save"
    assert!(
        painted.contains("Enter Save"),
        "footer must contain `Enter Save`, got:\n{painted}"
    );
    // @step And the rendered buffer contains the footer substring "Ctrl+D Clear"
    assert!(
        painted.contains("Ctrl+D Clear"),
        "footer must contain `Ctrl+D Clear`, got:\n{painted}"
    );
    // @step And the rendered buffer contains the footer substring "Esc Cancel"
    assert!(
        painted.contains("Esc Cancel"),
        "footer must contain `Esc Cancel`, got:\n{painted}"
    );
}

/// Scenario: RoleDialog seeded with no role opens with an empty editable buffer
#[test]
fn role_dialog_seeded_with_no_role_opens_with_empty_editable_buffer() {
    // @step Given a fresh RoleDialog with session_id SessionId("s-1") and seed_role = None
    let dialog = RoleDialog::new(SessionId::new("s-1"), None);

    // @step When the dialog's current draft is inspected
    let draft = dialog.draft();

    // @step Then the draft buffer is the empty string
    assert_eq!(draft, "");
}

/// Scenario: RoleDialog seeded with existing role pre-fills the editable buffer
#[test]
fn role_dialog_seeded_with_existing_role_pre_fills_editable_buffer() {
    // @step Given a fresh RoleDialog with session_id SessionId("s-1") and seed_role = Some("You are a security reviewer")
    let dialog = RoleDialog::new(
        SessionId::new("s-1"),
        Some("You are a security reviewer".to_string()),
    );

    // @step When the dialog's current draft is inspected
    let draft = dialog.draft();

    // @step Then the draft buffer reads "You are a security reviewer"
    assert_eq!(draft, "You are a security reviewer");
}

/// Scenario: Enter saves the draft as a non-empty role and removes the dialog from the Compositor
#[test]
fn enter_saves_non_empty_draft_and_removes_dialog_from_compositor() {
    // @step Given a fresh RoleDialog with session_id SessionId("s-1") and seed_role = Some("Reviewer")
    let mut dialog = RoleDialog::new(SessionId::new("s-1"), Some("Reviewer".to_string()));
    // @step And the dialog is mounted on a Compositor
    let mut compositor = Compositor::new();
    compositor.push(Box::new(RoleDialog::new(
        SessionId::new("s-1"),
        Some("Reviewer".to_string()),
    )));
    assert!(compositor.contains(ROLE_DIALOG_ID));

    // @step When the user types " B" so the draft reads "Reviewer B"
    let _ = dialog.handle_event(&key(KeyCode::Char(' ')));
    let _ = dialog.handle_event(&key(KeyCode::Char('B')));
    assert_eq!(dialog.draft(), "Reviewer B");

    // @step And the user presses Enter
    let result = dialog.handle_event(&key(KeyCode::Enter));

    // @step Then handle_event returns EventResult::Consumed with a callback
    let callback = match result {
        EventResult::Consumed(Some(cb)) => cb,
        _ => panic!("expected Consumed(Some(callback))"),
    };
    // @step And the pending action is Action::SetSessionRole(SessionId("s-1"), Some("Reviewer B"))
    let action = dialog
        .take_pending_action()
        .expect("pending action must be set");
    match action {
        Action::SetSessionRole(sid, role) => {
            assert_eq!(sid, SessionId::new("s-1"));
            assert_eq!(role, Some("Reviewer B".to_string()));
        }
        other => panic!("expected SetSessionRole, got {other:?}"),
    }
    // @step And after the callback runs the Compositor no longer contains "role-dialog"
    callback(&mut compositor);
    assert!(!compositor.contains(ROLE_DIALOG_ID));
}

/// Scenario: Enter on an empty draft clears the role (treated like Ctrl+D)
#[test]
fn enter_on_empty_draft_clears_the_role() {
    // @step Given a fresh RoleDialog with session_id SessionId("s-1") and seed_role = None
    let mut dialog = RoleDialog::new(SessionId::new("s-1"), None);
    // @step And the dialog is mounted on a Compositor
    let mut compositor = Compositor::new();
    compositor.push(Box::new(RoleDialog::new(SessionId::new("s-1"), None)));
    assert!(compositor.contains(ROLE_DIALOG_ID));

    // @step When the user presses Enter without typing
    let result = dialog.handle_event(&key(KeyCode::Enter));

    // @step Then handle_event returns EventResult::Consumed with a callback
    let callback = match result {
        EventResult::Consumed(Some(cb)) => cb,
        _ => panic!("expected Consumed(Some(callback))"),
    };
    // @step And the pending action is Action::SetSessionRole(SessionId("s-1"), None)
    let action = dialog
        .take_pending_action()
        .expect("pending action must be set");
    match action {
        Action::SetSessionRole(sid, role) => {
            assert_eq!(sid, SessionId::new("s-1"));
            assert_eq!(role, None);
        }
        other => panic!("expected SetSessionRole, got {other:?}"),
    }
    // @step And after the callback runs the Compositor no longer contains "role-dialog"
    callback(&mut compositor);
    assert!(!compositor.contains(ROLE_DIALOG_ID));
}

/// Scenario: Ctrl+D clears the role and removes the dialog from the Compositor
#[test]
fn ctrl_d_clears_the_role_and_removes_dialog() {
    // @step Given a fresh RoleDialog with session_id SessionId("s-1") and seed_role = Some("You are a reviewer")
    let mut dialog = RoleDialog::new(
        SessionId::new("s-1"),
        Some("You are a reviewer".to_string()),
    );
    // @step And the dialog is mounted on a Compositor
    let mut compositor = Compositor::new();
    compositor.push(Box::new(RoleDialog::new(
        SessionId::new("s-1"),
        Some("You are a reviewer".to_string()),
    )));
    assert!(compositor.contains(ROLE_DIALOG_ID));

    // @step When the user presses Ctrl+D
    let result = dialog.handle_event(&key_with_mods(KeyCode::Char('d'), KeyModifiers::CONTROL));

    // @step Then handle_event returns EventResult::Consumed with a callback
    let callback = match result {
        EventResult::Consumed(Some(cb)) => cb,
        _ => panic!("expected Consumed(Some(callback))"),
    };
    // @step And the pending action is Action::SetSessionRole(SessionId("s-1"), None)
    let action = dialog
        .take_pending_action()
        .expect("pending action must be set");
    match action {
        Action::SetSessionRole(sid, role) => {
            assert_eq!(sid, SessionId::new("s-1"));
            assert_eq!(role, None);
        }
        other => panic!("expected SetSessionRole, got {other:?}"),
    }
    // @step And after the callback runs the Compositor no longer contains "role-dialog"
    callback(&mut compositor);
    assert!(!compositor.contains(ROLE_DIALOG_ID));
}

/// Scenario: Esc cancels the dialog without emitting an Action
#[test]
fn esc_cancels_the_dialog_without_emitting_an_action() {
    // @step Given a fresh RoleDialog with session_id SessionId("s-1") and seed_role = Some("You are a reviewer")
    let mut dialog = RoleDialog::new(
        SessionId::new("s-1"),
        Some("You are a reviewer".to_string()),
    );
    // @step And the dialog is mounted on a Compositor
    let mut compositor = Compositor::new();
    compositor.push(Box::new(RoleDialog::new(
        SessionId::new("s-1"),
        Some("You are a reviewer".to_string()),
    )));
    assert!(compositor.contains(ROLE_DIALOG_ID));

    // @step When the user types " typo" so the draft reads "You are a reviewer typo"
    for ch in " typo".chars() {
        let _ = dialog.handle_event(&key(KeyCode::Char(ch)));
    }
    assert_eq!(dialog.draft(), "You are a reviewer typo");

    // @step And the user presses Esc
    let result = dialog.handle_event(&key(KeyCode::Esc));

    // @step Then handle_event returns EventResult::Consumed with a callback
    let callback = match result {
        EventResult::Consumed(Some(cb)) => cb,
        _ => panic!("expected Consumed(Some(callback))"),
    };
    // @step And no pending action is emitted
    assert!(
        dialog.take_pending_action().is_none(),
        "Esc must not emit any pending action"
    );
    // @step And after the callback runs the Compositor no longer contains "role-dialog"
    callback(&mut compositor);
    assert!(!compositor.contains(ROLE_DIALOG_ID));
}

/// Scenario: RoleDialog file stays under 300 lines
#[test]
fn role_dialog_rs_stays_under_300_lines() {
    // @step Given the file rust/fspec-tui/src/components/role_dialog.rs after RPC-063 lands
    let path = common::workspace_root()
        .join("fspec-tui")
        .join("src")
        .join("components")
        .join("role_dialog.rs");
    // @step When a test counts the line-count of the file
    let lines = common::read_to_string_or_panic(&path).lines().count();
    // @step Then the file has fewer than 300 lines
    assert!(lines < 300, "role_dialog.rs has {lines} lines (>= 300)");
}

/// Scenario: The dispatch helper file for RPC-063 stays under 300 lines
#[test]
fn dispatch_role_dialog_rs_stays_under_300_lines() {
    // @step Given the file rust/fspec-tui/src/app/dispatch_role_dialog.rs after RPC-063 lands
    let path = common::workspace_root()
        .join("fspec-tui")
        .join("src")
        .join("app")
        .join("dispatch_role_dialog.rs");
    // @step When a test counts the line-count of the file
    let lines = common::read_to_string_or_panic(&path).lines().count();
    // @step Then the file has fewer than 300 lines
    assert!(
        lines < 300,
        "dispatch_role_dialog.rs has {lines} lines (>= 300)"
    );
}
