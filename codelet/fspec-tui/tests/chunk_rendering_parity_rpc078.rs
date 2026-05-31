//! RPC-078 — AgentView scrollback exact TS Ink parity.
//!
//! Feature: spec/features/agentview-scrollback-no-duplicate-userinput-and-wrap.feature
//!
//! These tests assert the CORRECT TS Ink reference prefixes:
//!   UserInput       → "You: <text>"      GREEN
//!   Text            → "● <text>"         WHITE
//!   Thinking        → "[Thinking]\n..."  YELLOW
//!   Error           → "API Error: ..."   WHITE
//!   Done            → (no line, strip "..." from previous Text)
//!   Interrupted     → "⚠ Interrupted"    WHITE
//!   UserNotification→ verbatim message   inherited
//!   IncomingMessage → "[W] <role>> <body>" MAGENTA
//!
//! And the no-duplicate-user-input invariant:
//!   handle_input_submitted MUST NOT push a synchronous "You: ..." line
//!   when the session is backed by a real session manager — the
//!   UserInput chunk broadcast path is the single source of truth.
//!
//! Authoritative reference: spec/attachments/RPC-078/chunk-variant-matrix.md
//!                          spec/attachments/RPC-078/before-after-architecture.md

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{NotificationSeverity, SessionId, StreamChunk};
use ratatui::style::Color;
use ratatui::text::Line;

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

fn app_with_session() -> App {
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app
}

/// Flatten every visible chunk in `id`'s scrollback into a Vec of
/// per-chunk Vec<Line<'static>> so tests can inspect spans + styles.
fn session_scrollback_chunks(app: &App, id: &SessionId) -> Vec<Vec<Line<'static>>> {
    app.agent_view_store()
        .session_context_for(id)
        .map(|c| c.scrollback.visible_window(1024))
        .unwrap_or_default()
        .into_iter()
        .map(|c| c.lines)
        .collect()
}

/// Concatenate every span's content across every Line of every chunk
/// into a flat Vec<String> — one entry per Line.
fn session_scrollback_text_lines(app: &App, id: &SessionId) -> Vec<String> {
    session_scrollback_chunks(app, id)
        .into_iter()
        .flat_map(|lines| {
            lines.into_iter().map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
        })
        .collect()
}

/// Return the foreground color of the FIRST span of the FIRST line of
/// the FIRST chunk in `id`'s scrollback. Used by per-variant color
/// assertions.
fn first_span_fg(app: &App, id: &SessionId) -> Option<Color> {
    app.agent_view_store()
        .session_context_for(id)?
        .scrollback
        .visible_window(1024)
        .first()?
        .lines
        .first()?
        .spans
        .first()?
        .style
        .fg
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: UserInput chunk renders as green You: prefixed line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn user_input_chunk_renders_as_green_you_prefixed_line() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When the chunks subscriber forwards StreamChunk::UserInput { text: "hi" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::user_input("hi".to_string()),
    ));

    // @step Then the s-1 scrollback contains exactly one rendered chunk whose visible text equals "You: hi"
    let lines = session_scrollback_text_lines(&app, &sid("s-1"));
    assert_eq!(lines, vec!["You: hi".to_string()]);

    // @step Then the rendered chunk's spans carry foreground color GREEN
    assert_eq!(first_span_fg(&app, &sid("s-1")), Some(Color::Green));

    // @step Then no rendered chunk contains the substring "user> "
    for line in &lines {
        assert!(
            !line.contains("user> "),
            "scrollback must not use wrong-prefix 'user> '; got {line:?}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Text assistant chunk renders as white circle-bullet prefixed line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn text_assistant_chunk_renders_as_white_circle_bullet_prefixed_line() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When the chunks subscriber forwards StreamChunk::Text { text: "hello back" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::text("hello back".to_string()),
    ));

    // @step Then the s-1 scrollback contains exactly one rendered chunk whose visible text equals "● hello back"
    let lines = session_scrollback_text_lines(&app, &sid("s-1"));
    assert_eq!(lines, vec!["● hello back".to_string()]);

    // @step When the chunks subscriber forwards StreamChunk::Done for s-1
    app.dispatch(Action::ChunkReceived(sid("s-1"), StreamChunk::Done));

    // @step Then no rendered chunk contains the substring "assistant> " or "[done]"
    let after = session_scrollback_text_lines(&app, &sid("s-1"));
    for line in &after {
        assert!(
            !line.contains("assistant> "),
            "scrollback must not use wrong-prefix 'assistant> '; got {line:?}"
        );
        assert!(
            !line.contains("[done]"),
            "scrollback must not emit '[done]' literal; got {line:?}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Error chunk renders as API Error prefixed status line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn error_chunk_renders_as_api_error_prefixed_status_line() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When the chunks subscriber forwards StreamChunk::Error { error: "rate limit exceeded" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::error("rate limit exceeded".to_string()),
    ));

    // @step Then the s-1 scrollback contains exactly one rendered chunk whose visible text equals "API Error: rate limit exceeded"
    let lines = session_scrollback_text_lines(&app, &sid("s-1"));
    assert_eq!(lines, vec!["API Error: rate limit exceeded".to_string()]);

    // @step Then no rendered chunk contains the substring "[error]"
    for line in &lines {
        assert!(
            !line.contains("[error]"),
            "scrollback must not emit '[error]' literal; got {line:?}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Done chunk produces no scrollback line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn done_chunk_produces_no_scrollback_line() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When the chunks subscriber forwards StreamChunk::Done for s-1
    app.dispatch(Action::ChunkReceived(sid("s-1"), StreamChunk::Done));

    // @step Then the s-1 scrollback contains zero rendered chunks
    let chunks = session_scrollback_chunks(&app, &sid("s-1"));
    assert_eq!(
        chunks.len(),
        0,
        "Done must not emit any scrollback line; got {chunks:?}"
    );

    // @step Then no rendered chunk contains the substring "[done]"
    let lines = session_scrollback_text_lines(&app, &sid("s-1"));
    for line in &lines {
        assert!(
            !line.contains("[done]"),
            "scrollback must not emit '[done]' literal; got {line:?}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Interrupted chunk renders as warning-prefixed status line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn interrupted_chunk_renders_as_warning_prefixed_status_line() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When the chunks subscriber forwards StreamChunk::Interrupted { queued_inputs: vec!["a", "b"] } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::interrupted(vec!["a".to_string(), "b".to_string()]),
    ));

    // @step Then the s-1 scrollback contains exactly one rendered chunk whose visible text equals "⚠ Interrupted"
    let lines = session_scrollback_text_lines(&app, &sid("s-1"));
    assert_eq!(lines, vec!["⚠ Interrupted".to_string()]);

    // @step Then no rendered chunk contains the substring "[interrupted]" or "queued"
    for line in &lines {
        assert!(
            !line.contains("[interrupted]"),
            "scrollback must not emit '[interrupted]' literal; got {line:?}"
        );
        assert!(
            !line.contains("queued"),
            "scrollback must not surface raw 'queued' debug text; got {line:?}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: UserNotification chunk renders the message verbatim with no extra prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn user_notification_chunk_renders_message_verbatim_with_no_extra_prefix() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When the chunks subscriber forwards StreamChunk::UserNotification { message: "⟳ Reconnecting..." } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::user_notification(
            "⟳ Reconnecting...".to_string(),
            NotificationSeverity::Info,
        ),
    ));

    // @step Then the s-1 scrollback contains exactly one rendered chunk whose visible text equals "⟳ Reconnecting..."
    let lines = session_scrollback_text_lines(&app, &sid("s-1"));
    assert_eq!(lines, vec!["⟳ Reconnecting...".to_string()]);

    // @step Then no rendered chunk contains the substring "[notice]"
    for line in &lines {
        assert!(
            !line.contains("[notice]"),
            "scrollback must not emit '[notice]' literal; got {line:?}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: IncomingMessage chunk parses SUPERVISOR prefix and renders as magenta bracket-W line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn incoming_message_chunk_parses_supervisor_prefix_and_renders_as_magenta_bracket_w_line() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When the chunks subscriber forwards StreamChunk::IncomingMessage { text: "[SUPERVISOR: reviewer | Session: s-2]\nplease check this" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::incoming_message(
            "[SUPERVISOR: reviewer | Session: s-2]\nplease check this".to_string(),
        ),
    ));

    // @step Then the s-1 scrollback contains exactly one rendered chunk whose visible text equals "[W] reviewer> please check this"
    let lines = session_scrollback_text_lines(&app, &sid("s-1"));
    assert_eq!(lines, vec!["[W] reviewer> please check this".to_string()]);

    // @step Then the rendered chunk's spans carry foreground color MAGENTA
    assert_eq!(first_span_fg(&app, &sid("s-1")), Some(Color::Magenta));

    // @step Then no rendered chunk contains the substring "supervisor> "
    for line in &lines {
        assert!(
            !line.contains("supervisor> "),
            "scrollback must not use wrong-prefix 'supervisor> '; got {line:?}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Thinking chunk renders as yellow Thinking-prefixed block
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn thinking_chunk_renders_as_yellow_thinking_prefixed_block() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When the chunks subscriber forwards StreamChunk::Thinking { thinking: "considering the options" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::thinking("considering the options".to_string()),
    ));

    // @step Then the s-1 scrollback contains a rendered chunk whose first visible line equals "[Thinking]"
    let chunks = session_scrollback_chunks(&app, &sid("s-1"));
    assert_eq!(chunks.len(), 1, "expected exactly one chunk");
    let first_line_text: String = chunks[0]
        .first()
        .expect("at least one Line")
        .spans
        .iter()
        .map(|s| s.content.as_ref())
        .collect();
    assert_eq!(first_line_text, "[Thinking]".to_string());

    // @step Then the rendered chunk's spans carry foreground color YELLOW
    assert_eq!(first_span_fg(&app, &sid("s-1")), Some(Color::Yellow));

    // @step Then no rendered chunk contains the substring "(thinking)"
    let lines = session_scrollback_text_lines(&app, &sid("s-1"));
    for line in &lines {
        assert!(
            !line.contains("(thinking)"),
            "scrollback must not use wrong-prefix '(thinking)'; got {line:?}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: User input is not duplicated when the session manager
// broadcasts UserInput back
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn user_input_is_not_duplicated_when_session_manager_broadcasts_userinput_back() {
    // @step Given an AgentView with an open session s-1 backed by a real session manager
    // (MockBackend models a "real" session manager — its send_input is a no-op so
    //  the broadcast must come from explicit Action::ChunkReceived in this test;
    //  the production path replays UserInput via background_session.rs:1089)
    let mut app = app_with_session();

    // @step When the App dispatches Action::InputSubmitted("is this card done?")
    app.dispatch(Action::InputSubmitted("is this card done?".to_string()));

    // @step Then the s-1 scrollback contains exactly one rendered chunk whose visible text equals "You: is this card done?"
    // (After the RPC-078 fix the synchronous push at handle_input_submitted is
    //  removed, so for sessions backed by a session manager the only way the
    //  user line lands is via the ChunkReceived broadcast below.)
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::user_input("is this card done?".to_string()),
    ));
    let lines = session_scrollback_text_lines(&app, &sid("s-1"));
    let target = "You: is this card done?";
    let count = lines.iter().filter(|l| *l == target).count();
    assert_eq!(
        count, 1,
        "expected EXACTLY one '{target}' line; got {lines:?}"
    );

    // @step When the chunks subscriber forwards Action::ChunkReceived(s-1, StreamChunk::UserInput { text: "is this card done?" })
    // (already done above — this scenario asserts the count stays at 1 after
    //  the chunk lands once.)

    // @step Then the count of "You: is this card done?" lines is exactly 1
    let after = session_scrollback_text_lines(&app, &sid("s-1"));
    let after_count = after.iter().filter(|l| *l == target).count();
    assert_eq!(
        after_count, 1,
        "user input must not be duplicated; got {after:?}"
    );
}
