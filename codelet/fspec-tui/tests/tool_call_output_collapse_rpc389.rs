//! RPC-389 — tool-call output collapse + streaming window parity.
//!
//! Feature: spec/features/tool-call-output-collapse.feature
//!
//! Authoritative TS reference:
//!   src/tui/components/AgentView.tsx:533-605
//!     (formatCollapsedOutput COLLAPSED_LINES=8 + createStreamingWindow
//!      STREAMING_WINDOW_SIZE=10)
//!
//! These tests drive the REAL render path: push a ToolCall chunk, then a
//! ToolProgress (streaming) and/or ToolResult (settled) via the store, and
//! assert the rendered (wrapped) `lines` of the tool-call card. They MUST
//! fail (red phase) against the current verbatim `wrap_source` which dumps
//! every body line with no cap.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{Action, App, ChunkKind, FspecBackend};
use codelet_rpc_types::{SessionId, StreamChunk, ToolCallInfo, ToolProgressInfo, ToolResultInfo};

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn app_with_session() -> App {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app
}

fn tool_call_chunk(id: &str, name: &str, input: &str) -> StreamChunk {
    StreamChunk::tool_call(ToolCallInfo {
        id: id.to_string(),
        name: name.to_string(),
        input: input.to_string(),
    })
}

fn tool_result_chunk(tool_call_id: &str, content: &str, is_error: bool) -> StreamChunk {
    StreamChunk::tool_result(ToolResultInfo {
        tool_call_id: tool_call_id.to_string(),
        content: content.to_string(),
        is_error,
    })
}

fn tool_progress_chunk(tool_call_id: &str, name: &str, chunk: &str) -> StreamChunk {
    StreamChunk::tool_progress(ToolProgressInfo {
        tool_call_id: tool_call_id.to_string(),
        tool_name: name.to_string(),
        output_chunk: chunk.to_string(),
        is_stderr: false,
    })
}

/// All rendered lines (one String per `Line`, spans concatenated) of the
/// FIRST chunk (the tool-call card) in s-1's scrollback.
fn first_chunk_lines(app: &App, id: &SessionId) -> Vec<String> {
    let ctx = app
        .agent_view_store()
        .session_context_for(id)
        .expect("session context exists");
    let chunks = ctx.scrollback.visible_window(4096);
    chunks
        .first()
        .map(|c| {
            c.lines
                .iter()
                .map(|l| {
                    l.spans
                        .iter()
                        .map(|s| s.content.as_ref())
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

/// The stable `seq` of the first chunk (tool-call card).
fn first_chunk_seq(app: &App, id: &SessionId) -> u64 {
    let ctx = app
        .agent_view_store()
        .session_context_for(id)
        .expect("session context exists");
    ctx.scrollback.visible_window(4096).first().unwrap().seq
}

/// Build a body of `n` numbered lines: "line-1".."line-n".
fn numbered_body(n: usize) -> String {
    (1..=n)
        .map(|i| format!("line-{i}"))
        .collect::<Vec<_>>()
        .join("\n")
}

const INDICATOR_PREFIX: &str = "... +";

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Settled tool card with a short body shows the full body
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn settled_short_body_shows_full_body() {
    let mut app = app_with_session();
    // @step Given a settled tool-call card whose body has 5 lines
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk("tc-1", "Bash", "{\"command\":\"echo\"}"),
    ));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_result_chunk("tc-1", &numbered_body(5), false),
    ));

    // @step When the tool-call card is rendered into scrollback lines
    let lines = first_chunk_lines(&app, &sid("s-1"));

    // @step Then the rendered lines show all 5 body lines
    for i in 1..=5 {
        assert!(
            lines.iter().any(|l| l.contains(&format!("line-{i}"))),
            "expected body line-{i}; got {lines:?}"
        );
    }
    // @step And no "... +N lines" indicator line is shown
    assert!(
        !lines.iter().any(|l| l.contains(INDICATOR_PREFIX)),
        "no indicator expected; got {lines:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Settled tool card with a long body collapses to the first 8 lines
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn settled_long_body_collapses_to_first_8() {
    let mut app = app_with_session();
    // @step Given a settled tool-call card whose body has 20 lines
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk("tc-1", "Bash", "{\"command\":\"echo\"}"),
    ));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_result_chunk("tc-1", &numbered_body(20), false),
    ));

    // @step When the tool-call card is rendered into scrollback lines
    let lines = first_chunk_lines(&app, &sid("s-1"));

    // @step Then the rendered lines show the first 8 body lines
    for i in 1..=8 {
        assert!(
            lines.iter().any(|l| l.contains(&format!("line-{i}"))),
            "expected body line-{i}; got {lines:?}"
        );
    }
    assert!(
        !lines.iter().any(|l| l.contains("line-9")),
        "line-9 must be hidden; got {lines:?}"
    );
    // @step And the next rendered line is "... +12 lines (Enter to view full)"
    assert!(
        lines
            .iter()
            .any(|l| l == "... +12 lines (Enter to view full)"),
        "expected indicator; got {lines:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Streaming tool card shows only the last 10 lines
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn streaming_body_shows_last_10() {
    let mut app = app_with_session();
    // @step Given a streaming tool-call card whose body has 25 lines
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk("tc-1", "Bash", "{\"command\":\"build\"}"),
    ));
    for i in 1..=25 {
        app.dispatch(Action::ChunkReceived(
            sid("s-1"),
            tool_progress_chunk("tc-1", "Bash", &format!("line-{i}\n")),
        ));
    }

    // @step When the tool-call card is rendered into scrollback lines
    let lines = first_chunk_lines(&app, &sid("s-1"));

    // @step Then the rendered lines show only the last 10 body lines
    for i in 16..=25 {
        assert!(
            lines.iter().any(|l| l.contains(&format!("line-{i}"))),
            "expected tail body line-{i}; got {lines:?}"
        );
    }
    assert!(
        !lines.iter().any(|l| l.contains("line-15")),
        "line-15 must be outside the tail window; got {lines:?}"
    );
    // @step And no "... +N lines" indicator line is shown
    assert!(
        !lines.iter().any(|l| l.contains(INDICATOR_PREFIX)),
        "streaming window shows no indicator; got {lines:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A finished stream switches from the tail window to the collapsed view
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn finished_stream_switches_to_collapsed_view() {
    let mut app = app_with_session();
    // @step Given a streaming tool-call card whose body has 25 lines
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk("tc-1", "Bash", "{\"command\":\"build\"}"),
    ));
    for i in 1..=25 {
        app.dispatch(Action::ChunkReceived(
            sid("s-1"),
            tool_progress_chunk("tc-1", "Bash", &format!("line-{i}\n")),
        ));
    }

    // @step When the tool-call card finishes streaming
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_result_chunk("tc-1", "", false),
    ));

    // @step And the tool-call card is rendered into scrollback lines
    let lines = first_chunk_lines(&app, &sid("s-1"));

    // @step Then the rendered lines show the first 8 body lines
    for i in 1..=8 {
        assert!(
            lines.iter().any(|l| l.contains(&format!("line-{i}"))),
            "expected body line-{i}; got {lines:?}"
        );
    }
    assert!(
        !lines.iter().any(|l| l.contains("line-9")),
        "line-9 must be hidden after settle; got {lines:?}"
    );
    // @step And the next rendered line is "... +17 lines (Enter to view full)"
    assert!(
        lines
            .iter()
            .any(|l| l == "... +17 lines (Enter to view full)"),
        "expected indicator; got {lines:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The full body is preserved for the content modal
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn full_body_preserved_for_modal() {
    let mut app = app_with_session();
    // @step Given a settled tool-call card whose body has 20 lines
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk("tc-1", "Bash", "{\"command\":\"echo\"}"),
    ));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_result_chunk("tc-1", &numbered_body(20), false),
    ));

    // @step When the full text for the card's turn is requested for the TurnContentModal
    let seq = first_chunk_seq(&app, &sid("s-1"));
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .expect("session context exists");
    let full = ctx
        .scrollback
        .full_text_for_seq(seq)
        .expect("full text exists");

    // @step Then the returned text contains all 20 body lines
    for i in 1..=20 {
        assert!(
            full.contains(&format!("line-{i}")),
            "modal full text must contain line-{i}; got {full:?}"
        );
    }
    // Sanity: the card is a ToolCall.
    let kind = ctx
        .scrollback
        .visible_window(4096)
        .first()
        .and_then(|c| c.source.as_ref())
        .map(|s| s.kind.clone())
        .unwrap();
    assert!(matches!(kind, ChunkKind::ToolCall { .. }));
}
