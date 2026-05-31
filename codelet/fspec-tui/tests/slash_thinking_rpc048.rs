//! RPC-048 — `/thinking off|low|med|medium|high` inline-arg dispatch
//! wiring tests for the AgentView's `handle_input_submitted` path.
//!
//! Feature: spec/features/slash-command-thinking-inline.feature
//!
//! Drives the App with `Action::InputSubmitted("/thinking <arg>")`
//! against a `MockBackend` and asserts the new SetThinkingLevel and
//! InvalidThinkingLevel parser branches route through
//! `handle_thinking_level_selected` (backend round-trip) and a direct
//! scrollback push respectively — without opening the
//! ThinkingLevelDialog and without forwarding the text to
//! `backend.send_input`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{
    parse_slash_command, Action, App, FspecBackend, SlashCommandParse, THINKING_LEVEL_DIALOG_ID,
};
use codelet_rpc_types::{SessionId, ThinkingLevel};

mod common;
use common::MockBackend;

// ─────────────────────────────────────────────────────────────────────────
// Pure-parser table tests — exercise `parse_slash_command` directly so the
// branch table is asserted in isolation BEFORE the dispatch wiring runs.
// ─────────────────────────────────────────────────────────────────────────

/// Scenario Outline: parse_slash_command recognises /thinking <level> inline arg
#[test]
fn parse_slash_command_recognises_thinking_level_inline_args() {
    // @step Given the function parse_slash_command from app/slash_parser.rs
    // @step When it is called with text "<input>"
    // @step Then it returns SlashCommandParse::SetThinkingLevel(<level>)
    let cases = [
        ("/thinking off", ThinkingLevel::Off),
        ("/thinking low", ThinkingLevel::Low),
        ("/thinking med", ThinkingLevel::Medium),
        ("/thinking medium", ThinkingLevel::Medium),
        ("/thinking high", ThinkingLevel::High),
        ("/thinking HIGH", ThinkingLevel::High),
    ];
    for (input, expected) in cases {
        assert_eq!(
            parse_slash_command(input),
            SlashCommandParse::SetThinkingLevel(expected),
            "parse_slash_command({input:?}) must return SetThinkingLevel({expected:?})"
        );
    }
}

/// Scenario: parse_slash_command returns InvalidThinkingLevel for an unknown arg
#[test]
fn parse_slash_command_returns_invalid_thinking_level_for_unknown_arg() {
    // @step Given the function parse_slash_command from app/slash_parser.rs
    // @step When it is called with text "/thinking gibberish"
    // @step Then it returns SlashCommandParse::InvalidThinkingLevel("gibberish")
    assert_eq!(
        parse_slash_command("/thinking gibberish"),
        SlashCommandParse::InvalidThinkingLevel("gibberish".to_string())
    );
}

/// Scenario: Bare /thinking continues to open the ThinkingLevelDialog
#[test]
fn parse_slash_command_bare_thinking_opens_dialog() {
    // @step Given the function parse_slash_command from app/slash_parser.rs
    // @step When it is called with text "/thinking"
    // @step Then it returns SlashCommandParse::OpenThinkingDialog
    assert_eq!(
        parse_slash_command("/thinking"),
        SlashCommandParse::OpenThinkingDialog
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Dispatch wiring tests — drive the App with `Action::InputSubmitted("…")`
// against a `MockBackend` and assert the parser-driven branches reach the
// right backend method (or the right scrollback notice for invalid args).
// ─────────────────────────────────────────────────────────────────────────


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

/// Flatten every visible chunk's text in `id`'s scrollback into a single
/// String. Mirrors the `session_scrollback_text` helper in
/// `slash_clear_rpc046.rs` so the `[error] unknown thinking level: …`
/// assertion can scan the scrollback regardless of focus.
fn session_scrollback_text(app: &App, id: &SessionId) -> String {
    let chunks = app
        .agent_view_store()
        .session_context_for(id)
        .map(|c| c.scrollback.visible_window(1024))
        .unwrap_or_default();
    chunks
        .iter()
        .flat_map(|c| {
            c.lines
                .iter()
                .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect::<String>())
        })
        .collect::<Vec<String>>()
        .join("\n")
}

/// Scenario: Submitting "/thinking high" sets the level via the backend and does NOT open the dialog
#[tokio::test]
async fn submitting_slash_thinking_high_sets_level_and_does_not_open_dialog() {
    // @step Given an App with one open session SessionId("s-1") wired to a MockBackend
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await;
    // @step And AgentViewStore.thinking_level_for(SessionId("s-1")) = Some(ThinkingLevel::Off)
    app.dispatch(Action::ThinkingLevelLoaded(
        SessionId::new("s-1"),
        ThinkingLevel::Off,
    ));
    assert_eq!(
        app.agent_view_store()
            .thinking_level_for(&SessionId::new("s-1"))
            .copied(),
        Some(ThinkingLevel::Off)
    );
    // @step And the MockBackend's get_thinking_level returns ThinkingLevel::High
    mock.set_thinking_level(ThinkingLevel::High);
    let prior_send = mock.send_input_calls();
    let prior_set_thinking = mock.set_thinking_level_calls();
    // @step When the input is submitted with text "/thinking high"
    submit_input(&mut app, "/thinking high");
    drain_pending(&mut app).await;
    // @step Then within 1 second backend.set_thinking_level is called exactly once with (SessionId("s-1"), ThinkingLevel::High)
    assert_eq!(
        mock.set_thinking_level_calls(),
        prior_set_thinking + 1,
        "/thinking high must call backend.set_thinking_level exactly once"
    );
    let last = mock
        .last_set_thinking_level()
        .expect("last set_thinking_level");
    assert_eq!(last.0, SessionId::new("s-1"));
    assert_eq!(last.1, ThinkingLevel::High);
    // @step And no ThinkingLevelDialog is pushed onto the Compositor
    assert!(
        !app.compositor().contains(THINKING_LEVEL_DIALOG_ID),
        "/thinking high must NOT open the picker dialog"
    );
    // @step And the text is NOT forwarded to backend.send_input
    assert_eq!(
        mock.send_input_calls(),
        prior_send,
        "/thinking high must NOT forward to backend.send_input"
    );
    // @step And after the spawned task completes AgentViewStore.thinking_level_for(SessionId("s-1")) is Some(ThinkingLevel::High)
    assert_eq!(
        app.agent_view_store()
            .thinking_level_for(&SessionId::new("s-1"))
            .copied(),
        Some(ThinkingLevel::High),
        "thinking_level_for(s-1) must fold to High after the refresh"
    );
}

/// Scenario: Submitting "/thinking gibberish" emits an error notice and does NOT call the backend
#[tokio::test]
async fn submitting_slash_thinking_gibberish_emits_error_notice_and_skips_backend() {
    // @step Given an App with one open session SessionId("s-1") wired to a MockBackend
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await;
    let prior_send = mock.send_input_calls();
    let prior_set_thinking = mock.set_thinking_level_calls();
    // @step When the input is submitted with text "/thinking gibberish"
    submit_input(&mut app, "/thinking gibberish");
    drain_pending(&mut app).await;
    // @step Then SessionId("s-1") scrollback contains a chunk whose text equals "[error] unknown thinking level: gibberish"
    let text = session_scrollback_text(&app, &SessionId::new("s-1"));
    assert!(
        text.lines()
            .any(|l| l == "[error] unknown thinking level: gibberish"),
        "expected `[error] unknown thinking level: gibberish` in s-1 scrollback, got {text:?}"
    );
    // @step And backend.set_thinking_level is NOT called
    assert_eq!(
        mock.set_thinking_level_calls(),
        prior_set_thinking,
        "/thinking gibberish must NOT call backend.set_thinking_level"
    );
    // @step And no ThinkingLevelDialog is pushed onto the Compositor
    assert!(
        !app.compositor().contains(THINKING_LEVEL_DIALOG_ID),
        "/thinking gibberish must NOT open the picker dialog"
    );
    // @step And the text is NOT forwarded to backend.send_input
    assert_eq!(
        mock.send_input_calls(),
        prior_send,
        "/thinking gibberish must NOT forward to backend.send_input"
    );
}

/// Scenario: Submitting bare "/thinking" still opens the ThinkingLevelDialog (RPC-022 parity)
#[tokio::test]
async fn submitting_bare_slash_thinking_still_opens_the_dialog() {
    // @step Given an App with one open session SessionId("s-1") wired to a MockBackend
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await;
    // @step And no dialogs are pushed onto the Compositor
    assert!(!app.compositor().contains(THINKING_LEVEL_DIALOG_ID));
    let prior_send = mock.send_input_calls();
    let prior_set_thinking = mock.set_thinking_level_calls();
    // @step When the input is submitted with text "/thinking"
    submit_input(&mut app, "/thinking");
    drain_pending(&mut app).await;
    // @step Then a ThinkingLevelDialog with id "thinking-level-dialog" is pushed onto the Compositor at Priority::Foreground
    assert!(
        app.compositor().contains(THINKING_LEVEL_DIALOG_ID),
        "bare /thinking must open the picker dialog"
    );
    assert_eq!(
        app.compositor().topmost_priority(),
        Some(codelet_fspec_tui::Priority::Foreground)
    );
    // @step And the text is NOT forwarded to backend.send_input
    assert_eq!(
        mock.send_input_calls(),
        prior_send,
        "bare /thinking must NOT forward to backend.send_input"
    );
    // @step And backend.set_thinking_level is NOT called
    assert_eq!(
        mock.set_thinking_level_calls(),
        prior_set_thinking,
        "bare /thinking must NOT call backend.set_thinking_level"
    );
}
