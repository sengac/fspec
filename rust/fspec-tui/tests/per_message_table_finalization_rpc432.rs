//! RPC-432 — per-message markdown table finalization.
//!
//! Feature: spec/features/markdown-tables-per-assistant-message-finalization.feature
//!
//! Every non-empty in-flight AssistantText chunk MUST be run through
//! format_markdown_tables at finalization, regardless of which chunk
//! finalizes it (ToolCall, Error, Interrupted, UserInput, or Done).
//!
//! These tests MUST fail (red phase) against the current implementation,
//! where format_markdown_tables only runs in `handle_done` (turn Done)
//! and NOT in `flush_in_flight_drop_empty` (ToolCall / Error /
//! Interrupted / UserInput flushes).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{Action, App, ChunkKind, FspecBackend};
use codelet_rpc_types::{SessionId, StreamChunk, ToolCallInfo};

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

fn tool_call_chunk(id: &str, name: &str, input: &str) -> StreamChunk {
    StreamChunk::tool_call(ToolCallInfo {
        id: id.to_string(),
        name: name.to_string(),
        input: input.to_string(),
    })
}

/// Get the nth chunk's `source.text` (the stored, possibly-formatted text).
fn nth_chunk_source_text(app: &App, id: &SessionId, n: usize) -> String {
    let ctx = app
        .agent_view_store()
        .session_context_for(id)
        .expect("session context exists");
    let chunks = ctx.scrollback.visible_window(1024);
    chunks
        .get(n)
        .and_then(|c| c.source.as_ref())
        .map(|s| s.text.clone())
        .unwrap_or_default()
}

/// Get the nth chunk's `is_streaming` flag.
fn nth_chunk_is_streaming(app: &App, id: &SessionId, n: usize) -> bool {
    let ctx = app
        .agent_view_store()
        .session_context_for(id)
        .expect("session context exists");
    let chunks = ctx.scrollback.visible_window(1024);
    chunks
        .get(n)
        .and_then(|c| c.source.as_ref())
        .map(|s| s.is_streaming)
        .unwrap_or(true)
}

/// Get the nth chunk's `ChunkKind`.
fn nth_chunk_kind(app: &App, id: &SessionId, n: usize) -> Option<ChunkKind> {
    let ctx = app
        .agent_view_store()
        .session_context_for(id)
        .expect("session context exists");
    let chunks = ctx.scrollback.visible_window(1024);
    chunks
        .get(n)
        .and_then(|c| c.source.as_ref())
        .map(|s| s.kind.clone())
}

/// Count chunks in the session's scrollback.
fn session_chunk_count(app: &App, id: &SessionId) -> usize {
    app.agent_view_store()
        .session_context_for(id)
        .map(|c| c.scrollback.chunk_count())
        .unwrap_or(0)
}

/// The two tables used by the feature's scenarios.
const TABLE_H1: &str = "| h1 | h2 |\n|---|---|\n| a | b |";
const TABLE_AB: &str = "| a | b |\n|---|---|\n| 1 | 2 |";

/// Assert that `text` contains a box-drawing grid (top border ┌,
/// header separator ├, bottom border └) — i.e. it was formatted by
/// format_markdown_tables.
fn assert_contains_box_grid(text: &str, label: &str) {
    assert!(
        text.contains('┌') && text.contains('└') && text.contains('├'),
        "{label}: expected a box-drawing grid (┌/├/└), got {text:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: ToolCall finalizes the intermediate assistant message with a
//           box-drawing grid
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_toolcall_finalizes_intermediate_with_box_drawing_grid() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When the chunks subscriber forwards StreamChunk::Text { text: "| h1 | h2 |\n|---|---|\n| a | b |" } then StreamChunk::ToolCall { tool_call: { id: "tc-1", name: "Bash", input: "{\"command\":\"ls\"}" } } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::text(TABLE_H1.to_string()),
    ));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk("tc-1", "Bash", "{\"command\":\"ls\"}"),
    ));

    // @step Then the s-1 scrollback's first chunk source.text contains a box-drawing grid with top border "┌" and bottom border "└" instead of raw pipe rows
    let chunk0 = nth_chunk_source_text(&app, &sid("s-1"), 0);
    assert_contains_box_grid(&chunk0, "chunk 0 after ToolCall flush");
    assert!(
        !chunk0.contains("| h1 | h2 |"),
        "raw pipe rows must be replaced by the grid: {chunk0:?}"
    );

    // @step And the first chunk's is_streaming flag is false
    assert!(
        !nth_chunk_is_streaming(&app, &sid("s-1"), 0),
        "chunk 0 is_streaming must be false after ToolCall flush"
    );

    // @step And the s-1 scrollback ends with a ToolCall chunk for "tc-1" after the formatted assistant chunk
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 2);
    match nth_chunk_kind(&app, &sid("s-1"), 1) {
        Some(ChunkKind::ToolCall { tool_call_id, .. }) => assert_eq!(tool_call_id, "tc-1"),
        other => panic!("expected ToolCall kind at index 1, got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Error finalizes the in-flight assistant message with a
//           box-drawing grid
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_error_finalizes_with_box_drawing_grid() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When the chunks subscriber forwards StreamChunk::Text { text: "| a | b |\n|---|---|\n| 1 | 2 |" } then StreamChunk::Error { error: "rate limit exceeded" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::text(TABLE_AB.to_string()),
    ));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::Error {
            error: "rate limit exceeded".to_string(),
        },
    ));

    // @step Then the in-flight assistant chunk's source.text contains a box-drawing grid with top border "┌" and bottom border "└" instead of raw pipe rows
    let chunk0 = nth_chunk_source_text(&app, &sid("s-1"), 0);
    assert_contains_box_grid(&chunk0, "chunk 0 after Error flush");

    // @step And the in-flight assistant chunk's is_streaming flag is false
    assert!(
        !nth_chunk_is_streaming(&app, &sid("s-1"), 0),
        "chunk 0 is_streaming must be false after Error flush"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Interrupted finalizes the in-flight assistant message with a
//           box-drawing grid
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_interrupted_finalizes_with_box_drawing_grid() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When the chunks subscriber forwards StreamChunk::Text { text: "| a | b |\n|---|---|\n| 1 | 2 |" } then StreamChunk::Interrupted for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::text(TABLE_AB.to_string()),
    ));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::Interrupted {
            queued_inputs: vec![],
        },
    ));

    // @step Then the in-flight assistant chunk's source.text contains a box-drawing grid with top border "┌" and bottom border "└" instead of raw pipe rows
    let chunk0 = nth_chunk_source_text(&app, &sid("s-1"), 0);
    assert_contains_box_grid(&chunk0, "chunk 0 after Interrupted flush");

    // @step And the in-flight assistant chunk's is_streaming flag is false
    assert!(
        !nth_chunk_is_streaming(&app, &sid("s-1"), 0),
        "chunk 0 is_streaming must be false after Interrupted flush"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: UserInput finalizes the in-flight assistant message with a
//           box-drawing grid
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_userinput_finalizes_with_box_drawing_grid() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When the chunks subscriber forwards StreamChunk::Text { text: "| a | b |\n|---|---|\n| 1 | 2 |" } then StreamChunk::UserInput { text: "next question" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::text(TABLE_AB.to_string()),
    ));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::UserInput {
            text: "next question".to_string(),
        },
    ));

    // @step Then the in-flight assistant chunk's source.text contains a box-drawing grid with top border "┌" and bottom border "└" instead of raw pipe rows
    let chunk0 = nth_chunk_source_text(&app, &sid("s-1"), 0);
    assert_contains_box_grid(&chunk0, "chunk 0 after UserInput flush");

    // @step And the in-flight assistant chunk's is_streaming flag is false
    assert!(
        !nth_chunk_is_streaming(&app, &sid("s-1"), 0),
        "chunk 0 is_streaming must be false after UserInput flush"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Done still formats the turn-final assistant message exactly
//           once (regression guard for RPC-091/RPC-370 behavior)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_done_still_formats_turn_final_message() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When the chunks subscriber forwards StreamChunk::Text { text: "| a | b |\n|---|---|\n| 1 | 2 |" } then StreamChunk::Done for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::text(TABLE_AB.to_string()),
    ));
    app.dispatch(Action::ChunkReceived(sid("s-1"), StreamChunk::Done));

    // @step Then the final chunk's source.text contains a box-drawing grid with top border "┌" and bottom border "└" instead of raw pipe rows
    let chunk0 = nth_chunk_source_text(&app, &sid("s-1"), 0);
    assert_contains_box_grid(&chunk0, "chunk 0 after Done");

    // @step And the final chunk's is_streaming flag is false
    assert!(
        !nth_chunk_is_streaming(&app, &sid("s-1"), 0),
        "chunk 0 is_streaming must be false after Done"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Done after a ToolCall leaves the already-formatted message
//           unchanged (idempotency)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_done_after_toolcall_leaves_formatted_unchanged() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When the chunks subscriber forwards StreamChunk::Text { text: "| h1 | h2 |\n|---|---|\n| a | b |" } then StreamChunk::ToolCall { tool_call: { id: "tc-1", name: "Bash", input: "{\"command\":\"ls\"}" } } then StreamChunk::Text { text: "done listing" } then StreamChunk::Done for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::text(TABLE_H1.to_string()),
    ));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk("tc-1", "Bash", "{\"command\":\"ls\"}"),
    ));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::text("done listing".to_string()),
    ));
    app.dispatch(Action::ChunkReceived(sid("s-1"), StreamChunk::Done));

    // @step Then the first chunk (intermediate assistant message) still contains a box-drawing grid with top border "┌" and bottom border "└"
    let chunk0 = nth_chunk_source_text(&app, &sid("s-1"), 0);
    assert_contains_box_grid(&chunk0, "chunk 0 after Done (was ToolCall-flushed)");

    // @step And the second assistant chunk (after the tool call) contains the plain text "done listing"
    // Scrollback layout: 0 = assistant (grid), 1 = tool-call card,
    // 2 = second assistant message.
    let chunk2 = nth_chunk_source_text(&app, &sid("s-1"), 2);
    assert_eq!(
        chunk2, "done listing",
        "second assistant chunk must be the plain follow-up text"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: An intermediate assistant message without a table is
//           unchanged at flush
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_intermediate_without_table_unchanged_at_flush() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When the chunks subscriber forwards StreamChunk::Text { text: "Let me check the board" } then StreamChunk::ToolCall { tool_call: { id: "tc-1", name: "Bash", input: "{\"command\":\"ls\"}" } } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::text("Let me check the board".to_string()),
    ));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk("tc-1", "Bash", "{\"command\":\"ls\"}"),
    ));

    // @step Then the first chunk's source.text equals "Let me check the board" byte-for-byte
    let chunk0 = nth_chunk_source_text(&app, &sid("s-1"), 0);
    assert_eq!(
        chunk0, "Let me check the board",
        "non-table prose must be unchanged at flush"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Every assistant message in a multi-message turn renders its
//           own grid
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_every_message_in_multi_message_turn_renders_grid() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When the chunks subscriber forwards StreamChunk::Text { text: "| a | b |\n|---|---|\n| 1 | 2 |" } then StreamChunk::ToolCall { tool_call: { id: "tc-1", name: "Bash", input: "{\"command\":\"ls\"}" } } then StreamChunk::Text { text: "| x | y |\n|---|---|\n| 3 | 4 |" } then StreamChunk::Done for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::text(TABLE_AB.to_string()),
    ));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk("tc-1", "Bash", "{\"command\":\"ls\"}"),
    ));
    // After the ToolCall flush, the next Text delta starts a fresh
    // in-flight assistant bubble (RPC-091 semantics).
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::text("| x | y |\n|---|---|\n| 3 | 4 |".to_string()),
    ));
    app.dispatch(Action::ChunkReceived(sid("s-1"), StreamChunk::Done));

    // @step Then both assistant chunks contain box-drawing grids with top border "┌" and bottom border "└"
    // Scrollback layout: 0 = first assistant (ToolCall-flushed),
    // 1 = tool-call card, 2 = second assistant (Done-finalized).
    let chunk0 = nth_chunk_source_text(&app, &sid("s-1"), 0);
    assert_contains_box_grid(&chunk0, "chunk 0 (first assistant message)");
    let chunk2 = nth_chunk_source_text(&app, &sid("s-1"), 2);
    assert_contains_box_grid(&chunk2, "chunk 2 (second assistant message)");
}
