//! RPC-387 — Supervisor message rendering in the subordinate view.
//!
//! Feature: spec/features/supervisor-message-rendering.feature
//!
//! The backend `format_incoming_message` emits a SPACE-separated envelope
//! `"[SUPERVISOR: <role> | Session: <sid>] <body>"`. The Rust TUI parser
//! `parse_supervisor_envelope` (private to `session_context.rs`) must extract
//! the body for BOTH the space form (real backend) and the newline form
//! (replay/legacy), and fall back to the `supervisor` role when no header is
//! present.
//!
//! `parse_supervisor_envelope` is a private fn, so these tests exercise it
//! end-to-end through the PUBLIC render path: a `StreamChunk::IncomingMessage`
//! is dispatched and the resulting scrollback line `"[W] <role>> <body>"`
//! (magenta) is asserted. This mirrors how the real subordinate view renders
//! supervisor messages.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, StreamChunk};
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
/// the FIRST chunk in `id`'s scrollback.
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
// Scenario: Render a space-separated incoming supervisor message in the
//           scrollback (real backend wire format).
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn incoming_message_space_separated_backend_envelope_renders_magenta_bracket_w_line() {
    // @step Given a subordinate session with an empty scrollback
    let mut app = app_with_session();

    // @step When a StreamChunk::IncomingMessage carrying "[SUPERVISOR: reviewer | Session: s-2] please check this" is recorded
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::incoming_message(
            "[SUPERVISOR: reviewer | Session: s-2] please check this".to_string(),
        ),
    ));

    // @step Then the scrollback shows a single rendered chunk with text "[W] reviewer> please check this"
    let lines = session_scrollback_text_lines(&app, &sid("s-1"));
    assert_eq!(lines, vec!["[W] reviewer> please check this".to_string()]);

    // @step And the rendered chunk is colored magenta
    assert_eq!(first_span_fg(&app, &sid("s-1")), Some(Color::Magenta));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Parse a space-separated supervisor envelope from the real
//           backend. Asserted end-to-end through the render path: the body
//           must survive into the scrollback line.
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn parse_space_separated_supervisor_envelope_yields_role_and_body() {
    // @step Given a supervisor envelope "[SUPERVISOR: reviewer | Session: s-2] please check this" separated by a space
    let mut app = app_with_session();
    let envelope = "[SUPERVISOR: reviewer | Session: s-2] please check this".to_string();

    // @step When the envelope is parsed
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::incoming_message(envelope),
    ));
    let lines = session_scrollback_text_lines(&app, &sid("s-1"));

    // @step Then the extracted role is "reviewer"
    assert!(
        lines.first().map(|l| l.starts_with("[W] reviewer> ")) == Some(true),
        "expected role 'reviewer' in rendered line; got {lines:?}"
    );

    // @step And the extracted body is "please check this"
    assert_eq!(lines, vec!["[W] reviewer> please check this".to_string()]);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Parse a newline-separated supervisor envelope from the replay
//           path.
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn parse_newline_separated_supervisor_envelope_yields_role_and_body() {
    // @step Given a supervisor envelope "[SUPERVISOR: reviewer | Session: s-2]\nplease check this" separated by a newline
    let mut app = app_with_session();
    let envelope = "[SUPERVISOR: reviewer | Session: s-2]\nplease check this".to_string();

    // @step When the envelope is parsed
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::incoming_message(envelope),
    ));
    let lines = session_scrollback_text_lines(&app, &sid("s-1"));

    // @step Then the extracted role is "reviewer"
    assert!(
        lines.first().map(|l| l.starts_with("[W] reviewer> ")) == Some(true),
        "expected role 'reviewer' in rendered line; got {lines:?}"
    );

    // @step And the extracted body is "please check this"
    assert_eq!(lines, vec!["[W] reviewer> please check this".to_string()]);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Fall back to the default role when no header is present.
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn parse_supervisor_envelope_without_header_falls_back_to_supervisor_role() {
    // @step Given a raw message "raw body without header" with no envelope header
    let mut app = app_with_session();
    let envelope = "raw body without header".to_string();

    // @step When the envelope is parsed
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::incoming_message(envelope),
    ));
    let lines = session_scrollback_text_lines(&app, &sid("s-1"));

    // @step Then the extracted role is "supervisor"
    assert!(
        lines.first().map(|l| l.starts_with("[W] supervisor> ")) == Some(true),
        "expected fallback role 'supervisor' in rendered line; got {lines:?}"
    );

    // @step And the extracted body is "raw body without header"
    assert_eq!(
        lines,
        vec!["[W] supervisor> raw body without header".to_string()]
    );
}
