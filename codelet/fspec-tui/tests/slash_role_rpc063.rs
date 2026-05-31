//! RPC-063 — App-level integration tests for the `/role` slash command
//! end-to-end (UI dialog) wiring.
//!
//! Feature: spec/features/role-slash-command-end-to-end-ui-dialog.feature
//!
//! Exercises the App::dispatch routing for both the palette pick
//! (`Action::SlashCommandSelected(SlashCommandAction::Role)`) and the
//! submit-line interception (`Action::InputSubmitted("/role")`) so we
//! cover both the dialog-open paths AND the direct-set / clear paths.
//!
//! Supersedes the obsolete RPC-022 "bare /role clears" tests — those
//! tests' scenarios are removed in this card (the behaviour changed).
//!
//! Dialog seed-from-store verification is indirect: a press of Enter
//! on the just-opened dialog round-trips through `backend.set_session_role`
//! with the seeded draft (no edits in between), so the captured arg on
//! `MockBackend.last_set_session_role()` equals the value the dialog
//! was seeded with. Component-level seed assertions live in
//! `role_dialog_rpc063.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::sync::Arc;

use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::{
    parse_slash_command, Action, App, FspecBackend, Priority, SlashCommandParse,
    ROLE_DIALOG_ID,
};
use codelet_rpc_types::SessionId;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use common::MockBackend;

fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

async fn drain_pending(app: &mut App) {
    while let Some(handle) = app.next_pending_task() {
        let _ = handle.await;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            let _ = handle.await;
        }
    }
}

fn submit_input(app: &mut App, text: &str) {
    app.dispatch(Action::InputSubmitted(text.to_string()));
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn role_dialog_count(app: &App) -> usize {
    app.compositor()
        .layer_ids()
        .iter()
        .filter(|id| id.as_str() == ROLE_DIALOG_ID)
        .count()
}

/// Scenario Outline: parse_slash_command routes /role variants to the new RoleDialog parse outcome
#[test]
fn parse_slash_command_routes_role_variants_to_open_role_dialog() {
    // @step Given the function parse_slash_command from app/slash_parser.rs
    // @step When it is called with text=<input>
    // @step Then it returns <expected_variant>

    // | /role | OpenRoleDialog |
    assert_eq!(
        parse_slash_command("/role"),
        SlashCommandParse::OpenRoleDialog
    );

    // | /role clear | ClearRole |
    assert_eq!(
        parse_slash_command("/role clear"),
        SlashCommandParse::ClearRole
    );

    // | /role CLEAR | ClearRole |
    assert_eq!(
        parse_slash_command("/role CLEAR"),
        SlashCommandParse::ClearRole
    );

    // | /role You are a security reviewer | SetRole("You are a security reviewer") |
    assert_eq!(
        parse_slash_command("/role You are a security reviewer"),
        SlashCommandParse::SetRole("You are a security reviewer".to_string())
    );

    // | /role  leading space ok | SetRole("leading space ok") |
    assert_eq!(
        parse_slash_command("/role  leading space ok"),
        SlashCommandParse::SetRole("leading space ok".to_string())
    );
}

/// Scenario: Palette pick of /role on a session with no role opens the dialog with empty draft
#[tokio::test]
async fn palette_pick_role_with_no_role_opens_dialog_with_empty_draft() {
    // @step Given an App with one open session SessionId("s-1") whose role_for(SessionId("s-1")) is None
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await;
    assert!(app
        .agent_view_store()
        .role_for(&SessionId::new("s-1"))
        .is_none());
    let prior_set_role = mock.set_session_role_calls();

    // @step When the user picks /role from the slash palette
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Role));
    drain_pending(&mut app).await;

    // @step Then a RoleDialog with id "role-dialog" is pushed onto the Compositor at Priority::Foreground
    assert!(app.compositor().contains(ROLE_DIALOG_ID));
    assert_eq!(app.compositor().topmost_priority(), Some(Priority::Foreground));
    // @step And the dialog's draft buffer is the empty string
    // Indirect verification: pressing Enter on a fresh dialog (no
    // edits) emits Action::SetSessionRole(sid, None) because an empty
    // draft is treated as a clear.
    let _ = app.handle_event(&key(KeyCode::Enter));
    drain_pending(&mut app).await;
    assert_eq!(
        mock.set_session_role_calls(),
        prior_set_role + 1,
        "Enter on empty draft must round-trip through backend.set_session_role"
    );
    let last = mock.last_set_session_role().expect("last set_session_role");
    assert_eq!(last.0, SessionId::new("s-1"));
    assert_eq!(last.1, None, "empty draft must save as None (clear)");
}

/// Scenario: Palette pick of /role on a session with an existing role pre-fills the dialog draft
#[tokio::test]
async fn palette_pick_role_with_existing_role_pre_fills_dialog_draft() {
    // @step Given an App with one open session SessionId("s-1") whose role_for(SessionId("s-1")) is Some("You are a security reviewer")
    let (mut app, mock) = fresh_app();
    // Seed the mock so the SessionCreated → spawn_get_session_role
    // round-trip lands the same role in the AgentViewStore (otherwise
    // the late SessionRoleLoaded(None) would wipe the manual set).
    mock.seed_session_role(
        SessionId::new("s-1"),
        Some("You are a security reviewer".to_string()),
    );
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await;
    assert_eq!(
        app.agent_view_store()
            .role_for(&SessionId::new("s-1"))
            .map(str::to_string),
        Some("You are a security reviewer".to_string())
    );
    let prior_set_role = mock.set_session_role_calls();

    // @step When the user picks /role from the slash palette
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Role));
    drain_pending(&mut app).await;

    // @step Then a RoleDialog with id "role-dialog" is pushed onto the Compositor at Priority::Foreground
    assert!(app.compositor().contains(ROLE_DIALOG_ID));
    assert_eq!(app.compositor().topmost_priority(), Some(Priority::Foreground));
    // @step And the dialog's draft buffer reads "You are a security reviewer"
    // Indirect verification: pressing Enter on the seeded dialog (no
    // edits) saves the seeded text verbatim.
    let _ = app.handle_event(&key(KeyCode::Enter));
    drain_pending(&mut app).await;
    assert_eq!(mock.set_session_role_calls(), prior_set_role + 1);
    let last = mock.last_set_session_role().expect("last set_session_role");
    assert_eq!(last.0, SessionId::new("s-1"));
    assert_eq!(last.1, Some("You are a security reviewer".to_string()));
}

/// Scenario: Palette pick of /role with no active session is a silent no-op
#[tokio::test]
async fn palette_pick_role_with_no_active_session_is_silent_noop() {
    // @step Given an App with NO open session
    let (mut app, mock) = fresh_app();
    let prior_set_role = mock.set_session_role_calls();
    let prior_chunks = app.navigator().agent.chunk_count(app.agent_view_store());

    // @step When the user picks /role from the slash palette
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Role));
    drain_pending(&mut app).await;

    // @step Then no RoleDialog is pushed onto the Compositor
    assert!(!app.compositor().contains(ROLE_DIALOG_ID));
    // @step And no tokio task is spawned that calls backend.set_session_role
    assert_eq!(mock.set_session_role_calls(), prior_set_role);
    // @step And no scrollback line is appended
    let chunks_now = app.navigator().agent.chunk_count(app.agent_view_store());
    assert_eq!(chunks_now, prior_chunks);
}

/// Scenario: Submitting bare "/role" opens the RoleDialog (no longer clears the role)
#[tokio::test]
async fn submitting_bare_slash_role_opens_the_role_dialog() {
    // @step Given an App with one open session SessionId("s-1") whose role_for(SessionId("s-1")) is Some("Reviewer A")
    let (mut app, mock) = fresh_app();
    mock.seed_session_role(SessionId::new("s-1"), Some("Reviewer A".to_string()));
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await;
    assert_eq!(
        app.agent_view_store()
            .role_for(&SessionId::new("s-1"))
            .map(str::to_string),
        Some("Reviewer A".to_string())
    );
    let prior_set_role = mock.set_session_role_calls();

    // @step When the input is submitted with text "/role"
    submit_input(&mut app, "/role");
    drain_pending(&mut app).await;

    // @step Then a RoleDialog with id "role-dialog" is pushed onto the Compositor at Priority::Foreground
    assert!(app.compositor().contains(ROLE_DIALOG_ID));
    assert_eq!(app.compositor().topmost_priority(), Some(Priority::Foreground));
    // @step And the dialog's draft buffer reads "Reviewer A"
    // @step And AgentViewStore.role_for(SessionId("s-1")) remains Some("Reviewer A")
    assert_eq!(
        app.agent_view_store()
            .role_for(&SessionId::new("s-1"))
            .map(str::to_string),
        Some("Reviewer A".to_string())
    );
    // @step And no tokio task is spawned that calls backend.set_session_role
    assert_eq!(mock.set_session_role_calls(), prior_set_role);
}

/// Scenario: Submitting "/role You are a code reviewer" sets the role directly without opening the dialog
#[tokio::test]
async fn submitting_role_with_text_sets_role_directly_no_dialog() {
    // @step Given an App with one open session SessionId("s-1") whose role_for(SessionId("s-1")) is None
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await;
    let prior_set_role = mock.set_session_role_calls();

    // @step When the input is submitted with text "/role You are a code reviewer"
    submit_input(&mut app, "/role You are a code reviewer");
    drain_pending(&mut app).await;

    // @step Then NO RoleDialog is pushed onto the Compositor
    assert!(!app.compositor().contains(ROLE_DIALOG_ID));
    // @step And AgentViewStore.role_for(SessionId("s-1")) becomes Some("You are a code reviewer")
    assert_eq!(
        app.agent_view_store()
            .role_for(&SessionId::new("s-1"))
            .map(str::to_string),
        Some("You are a code reviewer".to_string())
    );
    // @step And a tokio task is spawned that calls backend.set_session_role(SessionId("s-1"), Some("You are a code reviewer"))
    assert_eq!(mock.set_session_role_calls(), prior_set_role + 1);
    let last = mock.last_set_session_role().expect("last set_session_role");
    assert_eq!(last.0, SessionId::new("s-1"));
    assert_eq!(last.1, Some("You are a code reviewer".to_string()));
}

/// Scenario: Submitting "/role clear" clears the role directly without opening the dialog
#[tokio::test]
async fn submitting_role_clear_clears_role_directly_no_dialog() {
    // @step Given an App with one open session SessionId("s-1") whose role_for(SessionId("s-1")) is Some("Reviewer A")
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    app.agent_view_store_mut()
        .set_role(SessionId::new("s-1"), Some("Reviewer A".to_string()));
    drain_pending(&mut app).await;
    let prior_set_role = mock.set_session_role_calls();

    // @step When the input is submitted with text "/role clear"
    submit_input(&mut app, "/role clear");
    drain_pending(&mut app).await;

    // @step Then NO RoleDialog is pushed onto the Compositor
    assert!(!app.compositor().contains(ROLE_DIALOG_ID));
    // @step And AgentViewStore.role_for(SessionId("s-1")) becomes None
    assert!(app
        .agent_view_store()
        .role_for(&SessionId::new("s-1"))
        .is_none());
    // @step And a tokio task is spawned that calls backend.set_session_role(SessionId("s-1"), None)
    assert_eq!(mock.set_session_role_calls(), prior_set_role + 1);
    let last = mock.last_set_session_role().expect("last set_session_role");
    assert_eq!(last.0, SessionId::new("s-1"));
    assert_eq!(last.1, None);
}

/// Scenario: Opening the RoleDialog is idempotent when the dialog is already on the Compositor
#[tokio::test]
async fn opening_role_dialog_is_idempotent() {
    // @step Given an App with one open session SessionId("s-1") whose role_for(SessionId("s-1")) is Some("Reviewer A")
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    app.agent_view_store_mut()
        .set_role(SessionId::new("s-1"), Some("Reviewer A".to_string()));
    drain_pending(&mut app).await;

    // @step And the user has already picked /role once so a RoleDialog is mounted on the Compositor
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Role));
    drain_pending(&mut app).await;
    assert!(app.compositor().contains(ROLE_DIALOG_ID));
    assert_eq!(
        role_dialog_count(&app),
        1,
        "must have exactly one role-dialog mounted after first pick"
    );

    // @step When the user picks /role again from the slash palette
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Role));
    drain_pending(&mut app).await;

    // @step Then exactly one RoleDialog with id "role-dialog" is on the Compositor (no duplicate push)
    assert!(app.compositor().contains(ROLE_DIALOG_ID));
    assert_eq!(
        role_dialog_count(&app),
        1,
        "must remain exactly one role-dialog mounted after second pick"
    );
}
