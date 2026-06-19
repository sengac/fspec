//! RPC-022 — Slash-command parsing and wiring tests for /model,
//! /thinking, and /role.
//!
//! Feature: spec/features/rpc022-slash-command-wiring.feature
//!
//! Exercises the `parse_slash_command` table AND the input-submission
//! interception in `App::handle_input_submitted` that fans
//! /model/thinking/role through dispatch_model_thinking_dialogs helpers instead of
//! forwarding them to `backend.send_input`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::{
    parse_slash_command, Action, App, FspecBackend, Priority, SlashCommandParse, ViewMode,
    MODEL_SELECTOR_DIALOG_ID, THINKING_LEVEL_DIALOG_ID,
};
use codelet_rpc_types::{ModelEntry, ProviderInfo, SessionId, ThinkingLevel};

mod common;
use common::MockBackend;

fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

/// Drain pending tasks AND any follow-up actions emitted onto the
/// action bus.
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

/// Helper: dispatch the input-submission action — the same path the
/// MultiLineInput would take when the user presses Enter.
fn submit_input(app: &mut App, text: &str) {
    app.dispatch(Action::InputSubmitted(text.to_string()));
}

/// Scenario Outline: parse_slash_command recognises the four wired commands
#[test]
fn parse_slash_command_recognises_the_four_wired_commands() {
    // @step Given the function parse_slash_command from app/dispatch_model_thinking_dialogs.rs
    // @step When it is called with text=<input>
    // @step Then it returns <expected_variant>
    assert_eq!(
        parse_slash_command("/model"),
        SlashCommandParse::OpenModelDialog
    );
    assert_eq!(
        parse_slash_command("/thinking"),
        SlashCommandParse::OpenThinkingDialog
    );
    assert_eq!(
        parse_slash_command("/role"),
        SlashCommandParse::OpenRoleDialog
    );
    assert_eq!(
        parse_slash_command("/role clear"),
        SlashCommandParse::ClearRole
    );
    assert_eq!(
        parse_slash_command("/role You are a security reviewer"),
        SlashCommandParse::SetRole("You are a security reviewer".to_string())
    );
    assert_eq!(
        parse_slash_command("/role  leading space ok"),
        SlashCommandParse::SetRole("leading space ok".to_string())
    );
    assert_eq!(
        parse_slash_command("hello world"),
        SlashCommandParse::NotASlashCommand
    );
    assert_eq!(
        parse_slash_command("/unknown anything"),
        SlashCommandParse::NotASlashCommand
    );
}

/// Scenario: Submitting "/model" opens the full-screen ModelSelector
/// mode-view and spawns list_providers (RPC-337: replaces the retired
/// RPC-022 Compositor modal)
#[tokio::test]
async fn submitting_slash_model_opens_dialog_and_spawns_list_providers() {
    // @step Given an App with one open session SessionId("s-1") and no dialogs pushed
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await;
    assert!(!app.compositor().contains(MODEL_SELECTOR_DIALOG_ID));
    // @step And the backend's list_providers returns [ProviderInfo{ key: "openai", ... }]
    mock.seed_providers(vec![ProviderInfo {
        key: "openai".to_string(),
        display_name: "OpenAI".to_string(),
        models: vec![ModelEntry {
            id: "gpt-5.1-codex".to_string(),
            display_name: "gpt-5.1-codex".to_string(),
            context_window: 200_000,
            supports_reasoning: true,
            supports_vision: false,
            is_custom: false,
        }],
        profile_name: None,
        is_unreachable: false,
    }]);
    let prior_send = mock.send_input_calls();
    let prior_lp = mock.list_providers_calls();
    // @step When the input is submitted with text "/model"
    submit_input(&mut app, "/model");
    // @step Then the text is NOT forwarded to backend.send_input
    drain_pending(&mut app).await;
    assert_eq!(
        mock.send_input_calls(),
        prior_send,
        "/model must not be forwarded to backend.send_input"
    );
    // @step And the Navigator flips to ViewMode::ModelSelector (no Compositor modal)
    assert_eq!(app.active_view(), ViewMode::ModelSelector);
    assert!(!app.compositor().contains(MODEL_SELECTOR_DIALOG_ID));
    // @step And a tokio task is spawned that calls backend.list_providers()
    assert!(
        mock.list_providers_calls() > prior_lp,
        "list_providers must be called"
    );
    // @step And the model selector view contains 1 provider after the task resolves
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    let backend = TestBackend::new(100, 30);
    let mut term = Terminal::new(backend).expect("Terminal::new");
    term.draw(|frame| {
        app.render(frame.area(), frame.buffer_mut());
    })
    .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut painted = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            painted.push_str(buf[(x, y)].symbol());
        }
        painted.push('\n');
    }
    assert!(
        painted.contains("OpenAI"),
        "model selector must show 1 provider after list_providers resolves; got:\n{painted}"
    );
}

/// Scenario: Submitting "/thinking" opens the ThinkingLevelDialog seeded with the cached level
#[tokio::test]
async fn submitting_slash_thinking_opens_dialog_seeded_with_cached_level() {
    // @step Given an App with one open session SessionId("s-1") and no dialogs pushed
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await;
    // @step And AgentViewStore.thinking_level_for(SessionId("s-1")) = Some(ThinkingLevel::Medium)
    app.dispatch(Action::ThinkingLevelLoaded(
        SessionId::new("s-1"),
        ThinkingLevel::Medium,
    ));
    assert_eq!(
        app.agent_view_store()
            .thinking_level_for(&SessionId::new("s-1"))
            .copied(),
        Some(ThinkingLevel::Medium)
    );
    let prior_send = mock.send_input_calls();
    // @step When the input is submitted with text "/thinking"
    submit_input(&mut app, "/thinking");
    // @step Then the text is NOT forwarded to backend.send_input
    drain_pending(&mut app).await;
    assert_eq!(mock.send_input_calls(), prior_send);
    // @step And a ThinkingLevelDialog is pushed onto the Compositor at Priority::Foreground
    assert!(app.compositor().contains(THINKING_LEVEL_DIALOG_ID));
    assert_eq!(
        app.compositor().topmost_priority(),
        Some(Priority::Foreground)
    );
    // @step And the dialog's initial selected_level is ThinkingLevel::Medium
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    let backend = TestBackend::new(80, 24);
    let mut term = Terminal::new(backend).expect("Terminal::new");
    term.draw(|frame| {
        app.compositor_mut()
            .render(frame.area(), frame.buffer_mut());
    })
    .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut painted = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            painted.push_str(buf[(x, y)].symbol());
        }
        painted.push('\n');
    }
    let medium_row = painted
        .lines()
        .find(|l| l.contains("Medium"))
        .expect("Medium row not found in rendered dialog");
    assert!(
        medium_row.contains('▸'),
        "Medium must be highlighted (▸), got {medium_row:?}"
    );
}

/// Scenario: Submitting "/role You are a reviewer" sets the role and shows the RoleBanner
#[tokio::test]
async fn submitting_slash_role_text_sets_role_and_shows_banner() {
    // @step Given an App with one open session SessionId("s-1") and no dialogs pushed
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await;
    // @step And AgentViewStore.role_for(SessionId("s-1")) is None
    assert!(app
        .agent_view_store()
        .role_for(&SessionId::new("s-1"))
        .is_none());
    let prior_send = mock.send_input_calls();
    let prior_role = mock.set_session_role_calls();
    // @step When the input is submitted with text "/role You are a reviewer"
    submit_input(&mut app, "/role You are a reviewer");
    // @step Then the text is NOT forwarded to backend.send_input
    drain_pending(&mut app).await;
    assert_eq!(mock.send_input_calls(), prior_send);
    // @step And Action::SetSessionRole(SessionId("s-1"), Some("You are a reviewer")) is dispatched
    // @step And AgentViewStore.role_for(SessionId("s-1")) becomes Some("You are a reviewer")
    assert_eq!(
        app.agent_view_store().role_for(&SessionId::new("s-1")),
        Some("You are a reviewer")
    );
    // @step And a tokio task is spawned that calls backend.set_session_role(SessionId("s-1"), Some("You are a reviewer".to_string()))
    assert_eq!(mock.set_session_role_calls(), prior_role + 1);
    let last = mock.last_set_session_role().expect("last set_session_role");
    assert_eq!(last.0, SessionId::new("s-1"));
    assert_eq!(last.1, Some("You are a reviewer".to_string()));
}

/// Scenario: Submitting "/role clear" clears the role and hides the RoleBanner
#[tokio::test]
async fn submitting_slash_role_clear_clears_role() {
    // @step Given an App with one open session SessionId("s-1") whose role is Some("Reviewer A")
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    app.agent_view_store_mut()
        .set_role(SessionId::new("s-1"), Some("Reviewer A".to_string()));
    drain_pending(&mut app).await;
    let prior_role = mock.set_session_role_calls();
    // @step When the input is submitted with text "/role clear"
    submit_input(&mut app, "/role clear");
    drain_pending(&mut app).await;
    // @step Then Action::SetSessionRole(SessionId("s-1"), None) is dispatched
    // @step And AgentViewStore.role_for(SessionId("s-1")) becomes None
    assert!(app
        .agent_view_store()
        .role_for(&SessionId::new("s-1"))
        .is_none());
    // @step And a tokio task is spawned that calls backend.set_session_role(SessionId("s-1"), None)
    assert_eq!(mock.set_session_role_calls(), prior_role + 1);
    let last = mock.last_set_session_role().expect("last set_session_role");
    assert_eq!(last.0, SessionId::new("s-1"));
    assert_eq!(last.1, None);
}

/// Scenario: Submitting bare "/role" opens the RoleDialog (RPC-063 supersedes the old "treated as a clear" semantics)
#[tokio::test]
async fn submitting_bare_slash_role_opens_the_role_dialog() {
    use codelet_fspec_tui::ROLE_DIALOG_ID;
    // @step Given an App with one open session SessionId("s-1") whose role is Some("Reviewer A")
    let (mut app, _mock) = fresh_app();
    _mock.seed_session_role(SessionId::new("s-1"), Some("Reviewer A".to_string()));
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await;
    assert_eq!(
        app.agent_view_store()
            .role_for(&SessionId::new("s-1"))
            .map(str::to_string),
        Some("Reviewer A".to_string())
    );
    // @step When the input is submitted with text "/role"
    submit_input(&mut app, "/role");
    drain_pending(&mut app).await;
    // @step Then RPC-063 routes through SlashCommandParse::OpenRoleDialog
    //        and a RoleDialog is pushed onto the Compositor seeded with
    //        the existing role
    // @step And AgentViewStore.role_for(SessionId("s-1")) remains Some("Reviewer A")
    assert!(app.compositor().contains(ROLE_DIALOG_ID));
    assert_eq!(
        app.agent_view_store()
            .role_for(&SessionId::new("s-1"))
            .map(str::to_string),
        Some("Reviewer A".to_string()),
        "bare /role must NOT clear the role any more (RPC-063)"
    );
}

/// Scenario: Submitting plain text falls through to backend.send_input unchanged
#[tokio::test]
async fn submitting_plain_text_falls_through_to_backend_send_input() {
    // @step Given an App with one open session SessionId("s-1") and no dialogs pushed
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await;
    let prior_send = mock.send_input_calls();
    let prior_role = mock.set_session_role_calls();
    // @step When the input is submitted with text "hello world"
    submit_input(&mut app, "hello world");
    drain_pending(&mut app).await;
    // @step Then a tokio task is spawned that calls backend.send_input(SessionId("s-1"), "hello world")
    assert_eq!(mock.send_input_calls(), prior_send + 1);
    let last = mock.last_send_input().expect("last send_input");
    assert_eq!(last.0, SessionId::new("s-1"));
    assert_eq!(last.1, "hello world");
    // @step And no dialog is pushed onto the Compositor
    assert!(!app.compositor().contains(MODEL_SELECTOR_DIALOG_ID));
    assert!(!app.compositor().contains(THINKING_LEVEL_DIALOG_ID));
    // @step And no Action::SetSessionRole is dispatched
    assert_eq!(mock.set_session_role_calls(), prior_role);
}

/// Scenario: Slash commands are NOT appended to the per-session history
#[tokio::test]
async fn slash_commands_are_not_appended_to_per_session_history() {
    // @step Given an App with one open session SessionId("s-1")
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await;
    let prior_history = mock.persistence_add_history_calls();
    // @step When the input is submitted with text "/model"
    submit_input(&mut app, "/model");
    drain_pending(&mut app).await;
    // @step Then no tokio task is spawned that calls backend.persistence_add_history
    assert_eq!(
        mock.persistence_add_history_calls(),
        prior_history,
        "/model must not call persistence_add_history"
    );
    // @step When the input is submitted with text "hello"
    submit_input(&mut app, "hello");
    drain_pending(&mut app).await;
    // @step Then exactly one tokio task is spawned that calls backend.persistence_add_history(SessionId("s-1"), "hello")
    assert_eq!(
        mock.persistence_add_history_calls(),
        prior_history + 1,
        "plain `hello` must call persistence_add_history exactly once"
    );
    let last = mock
        .last_persistence_add_history()
        .expect("last persistence_add_history");
    assert_eq!(last.0, SessionId::new("s-1"));
    assert_eq!(last.1, "hello");
}

/// Scenario: Slash popup selection of /model also opens the full-screen
/// ModelSelector mode-view (RPC-337: replaces the retired modal)
#[tokio::test]
async fn slash_popup_selection_of_model_opens_the_dialog() {
    // @step Given an App with one open session SessionId("s-1") and the slash popup open with selected command Model
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await;
    let prior_lp = mock.list_providers_calls();
    // @step When the user presses Enter inside the popup
    // (The popup emits SlashCommandSelected(Model) on Enter.)
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Model));
    // @step Then Action::SlashCommandSelected(SlashCommandAction::Model) is dispatched
    // (Already dispatched above.)
    drain_pending(&mut app).await;
    // @step And the Navigator flips to ViewMode::ModelSelector (no Compositor modal)
    assert_eq!(app.active_view(), ViewMode::ModelSelector);
    assert!(!app.compositor().contains(MODEL_SELECTOR_DIALOG_ID));
    // @step And a tokio task is spawned that calls backend.list_providers()
    assert!(
        mock.list_providers_calls() > prior_lp,
        "list_providers must be called"
    );
}

/// Scenario: Slash popup selection of /role opens the RoleDialog (RPC-063 supersedes the old "treated as a clear" semantics)
#[tokio::test]
async fn slash_popup_selection_of_role_opens_the_role_dialog() {
    use codelet_fspec_tui::ROLE_DIALOG_ID;
    // @step Given an App with one open session SessionId("s-1") whose role is Some("Reviewer A") and the slash popup open with selected command Role
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
    let prior_role = mock.set_session_role_calls();
    let prior_chunks = app.navigator().agent.chunk_count(app.agent_view_store());
    // @step When the user presses Enter inside the popup
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Role));
    // @step Then Action::SlashCommandSelected(SlashCommandAction::Role) is dispatched
    drain_pending(&mut app).await;
    // @step And RPC-063 routes through handle_open_role_dialog and pushes
    //        the RoleDialog onto the Compositor
    assert!(app.compositor().contains(ROLE_DIALOG_ID));
    // @step And AgentViewStore.role_for(SessionId("s-1")) remains Some("Reviewer A")
    assert_eq!(
        app.agent_view_store()
            .role_for(&SessionId::new("s-1"))
            .map(str::to_string),
        Some("Reviewer A".to_string()),
        "popup-picker `/role` must NOT clear the role any more (RPC-063)"
    );
    // @step And NO tokio task is spawned that calls backend.set_session_role (the dialog defers persistence)
    assert_eq!(
        mock.set_session_role_calls(),
        prior_role,
        "popup-picker `/role` must NOT call backend.set_session_role any more (RPC-063 routes through the dialog)"
    );
    // @step And no scrollback line containing the substring "[notice] /role" is appended
    let chunk_delta = app.navigator().agent.chunk_count(app.agent_view_store()) - prior_chunks;
    assert_eq!(
        chunk_delta, 0,
        "popup-picker `/role` must NOT append a `[notice] /role` scrollback line — rule [5]"
    );
}
