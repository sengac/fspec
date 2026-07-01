//! Feature: spec/features/tool-progress-carries-tool-call-id.feature
//!
//! BUG-149 — Live tool output not folded into TUI card: ToolProgress emitted
//! with empty tool_call_id.
//!
//! These tests drive the REAL store render path (like
//! tool_call_output_collapse_rpc389.rs and chunkprocessor_parity_rpc091.rs):
//! dispatch Action::ChunkReceived with StreamChunk::tool_call / tool_progress
//! and assert the stored/rendered card body.
//!
//! These cover the MATCH side of the contract (fspec-tui handle_tool_progress
//! folds by EXACT tool_call_id). The TUI match-side is already correct, so
//! these lock the contract (they are expected to PASS). They guard against a
//! regression where an empty id would be folded into a card, or where progress
//! for another card would bleed into this one.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, StreamChunk, ToolCallInfo, ToolProgressInfo};

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

fn tool_progress_chunk(tool_call_id: &str, name: &str, chunk: &str) -> StreamChunk {
    StreamChunk::tool_progress(ToolProgressInfo {
        tool_call_id: tool_call_id.to_string(),
        tool_name: name.to_string(),
        output_chunk: chunk.to_string(),
        is_stderr: false,
    })
}

/// Return the stored `source.text` of the chunk whose ToolCall tool_call_id
/// matches `card_id`. Returns None if there is no such card.
fn card_body_text(app: &App, id: &SessionId, card_id: &str) -> Option<String> {
    let ctx = app
        .agent_view_store()
        .session_context_for(id)
        .expect("session context exists");
    ctx.scrollback
        .visible_window(4096)
        .into_iter()
        .find(|c| match c.source.as_ref().map(|s| &s.kind) {
            Some(codelet_fspec_tui::ChunkKind::ToolCall { tool_call_id, .. }) => {
                tool_call_id == card_id
            }
            _ => false,
        })
        .and_then(|c| c.source.as_ref().map(|s| s.text.clone()))
}

/// Return the `is_streaming` flag of the card whose id matches `card_id`.
fn card_is_streaming(app: &App, id: &SessionId, card_id: &str) -> Option<bool> {
    let ctx = app
        .agent_view_store()
        .session_context_for(id)
        .expect("session context exists");
    ctx.scrollback
        .visible_window(4096)
        .into_iter()
        .find(|c| match c.source.as_ref().map(|s| &s.kind) {
            Some(codelet_fspec_tui::ChunkKind::ToolCall { tool_call_id, .. }) => {
                tool_call_id == card_id
            }
            _ => false,
        })
        .and_then(|c| c.source.as_ref().map(|s| s.is_streaming))
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Progress with a matching tool_call_id folds into the streaming card
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn progress_with_matching_id_folds_into_streaming_card() {
    // @step Given the agent view scrollback contains a ToolCall card with tool_call_id "tc-1"
    let mut app = app_with_session();
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk("tc-1", "Bash", "{\"command\":\"npm test\"}"),
    ));

    // @step When a ToolProgress with tool_call_id "tc-1" arrives while the command is running
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_progress_chunk("tc-1", "Bash", "streamed-output-line\n"),
    ));

    // @step Then the card "tc-1" body shows the streamed output
    let body = card_body_text(&app, &sid("s-1"), "tc-1").expect("card tc-1 exists");
    assert!(
        body.contains("streamed-output-line"),
        "card tc-1 must absorb matching progress; got {body:?}"
    );

    // @step And the card "tc-1" is still marked as streaming
    assert_eq!(
        card_is_streaming(&app, &sid("s-1"), "tc-1"),
        Some(true),
        "card tc-1 must remain streaming after live progress"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Progress with an empty tool_call_id matches no card and is dropped
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn progress_with_empty_id_matches_no_card_and_is_dropped() {
    // @step Given the agent view scrollback contains a ToolCall card with tool_call_id "tc-1"
    let mut app = app_with_session();
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk("tc-1", "Bash", "{\"command\":\"npm test\"}"),
    ));

    // @step When a ToolProgress with an empty tool_call_id arrives
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_progress_chunk("", "Bash", "orphan-output\n"),
    ));

    // @step Then the card "tc-1" body does not show that output
    let body = card_body_text(&app, &sid("s-1"), "tc-1").expect("card tc-1 exists");
    assert!(
        !body.contains("orphan-output"),
        "empty-id progress must not fold into card tc-1; got {body:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Progress for another session's card does not alter this card
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn progress_for_another_card_does_not_alter_this_card() {
    // @step Given a ToolCall card with tool_call_id "tc-1" exists
    let mut app = app_with_session();
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk("tc-1", "Bash", "{\"command\":\"one\"}"),
    ));

    // @step And a separate ToolCall card with tool_call_id "tc-2" exists
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk("tc-2", "Bash", "{\"command\":\"two\"}"),
    ));

    let before = card_body_text(&app, &sid("s-1"), "tc-1").expect("card tc-1 exists");

    // @step When a ToolProgress with tool_call_id "tc-2" arrives
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_progress_chunk("tc-2", "Bash", "belongs-to-tc-2\n"),
    ));

    // @step Then the card "tc-1" body is unchanged
    let after = card_body_text(&app, &sid("s-1"), "tc-1").expect("card tc-1 exists");
    assert_eq!(
        before, after,
        "card tc-1 body must be unchanged by progress addressed to tc-2"
    );
    assert!(
        !after.contains("belongs-to-tc-2"),
        "card tc-1 must not absorb tc-2's output; got {after:?}"
    );
}
