//! RPC-093 — Thinking streaming accumulation parity:
//! consecutive StreamChunk::Thinking deltas merge into one yellow
//! [Thinking] block; ToolCall finalises it; UserInput/Interrupted
//! turn-boundaries (and Done/Error) clear the slot without mutating.
//!
//! Feature: spec/features/agentview-thinking-streaming-parity.feature
//!
//! Authoritative TS reference:
//!   src/tui/utils/thinkingBlockManager.ts  (appendThinking + findActiveThinkingBlock + finalizeThinkingBlock)
//!   src/tui/utils/chunkProcessor.ts:463-466 (Thinking branch)
//!   src/tui/utils/chunkProcessor.ts:469     (ToolCall finalize call site)
//!
//! These tests assert post-RPC-093 acceptance. They MUST fail
//! (red phase) against the current per-delta-push stub in
//! codelet/fspec-tui/src/store/agent_view/session_context.rs:80-86.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{Action, App, ChunkKind, FspecBackend};
use codelet_rpc_types::{SessionId, StreamChunk, ToolCallInfo, ToolResultInfo};

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

/// Flatten visible chunks → one String per Line (spans concatenated).
fn session_lines(app: &App, id: &SessionId) -> Vec<String> {
    app.agent_view_store()
        .session_context_for(id)
        .map(|c| c.scrollback.visible_window(1024))
        .unwrap_or_default()
        .into_iter()
        .flat_map(|c| {
            c.lines.into_iter().map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
        })
        .collect()
}

fn session_chunk_count(app: &App, id: &SessionId) -> usize {
    app.agent_view_store()
        .session_context_for(id)
        .map(|c| c.scrollback.chunk_count())
        .unwrap_or(0)
}

fn session_in_flight_assistant(app: &App, id: &SessionId) -> Option<usize> {
    app.agent_view_store()
        .session_context_for(id)
        .and_then(|c| c.in_flight_assistant)
}

/// **RPC-093**: read the new `in_flight_thinking` slot.
fn session_in_flight_thinking(app: &App, id: &SessionId) -> Option<usize> {
    app.agent_view_store()
        .session_context_for(id)
        .and_then(|c| c.in_flight_thinking)
}

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

fn nth_chunk_kind(app: &App, id: &SessionId, n: usize) -> ChunkKind {
    let ctx = app
        .agent_view_store()
        .session_context_for(id)
        .expect("session context exists");
    let chunks = ctx.scrollback.visible_window(1024);
    chunks
        .get(n)
        .and_then(|c| c.source.as_ref())
        .map(|s| s.kind.clone())
        .expect("ChunkSource present")
}

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
        .unwrap_or(false)
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

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Consecutive Thinking deltas accumulate into a single
//           in-flight thinking chunk
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn consecutive_thinking_deltas_accumulate_into_single_in_flight_thinking_chunk() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When the chunks subscriber forwards StreamChunk::Thinking { thinking: "The user" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::thinking("The user".to_string()),
    ));

    // @step And the chunks subscriber forwards StreamChunk::Thinking { thinking: " is asking" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::thinking(" is asking".to_string()),
    ));

    // @step And the chunks subscriber forwards StreamChunk::Thinking { thinking: " about cards" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::thinking(" about cards".to_string()),
    ));

    // @step Then the s-1 scrollback contains exactly one rendered chunk
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 1);

    // @step And that chunk's source.text equals "The user is asking about cards" without any "[Thinking]" prefix baked in
    let stored = nth_chunk_source_text(&app, &sid("s-1"), 0);
    assert_eq!(stored, "The user is asking about cards");
    assert!(
        !stored.contains("[Thinking]"),
        "stored thinking text must not bake the [Thinking] prefix; got {stored:?}"
    );

    // @step And the SessionContext in_flight_thinking slot is Some(<that chunk's index>)
    assert_eq!(session_in_flight_thinking(&app, &sid("s-1")), Some(0));

    // @step And the chunk's source.kind is ChunkKind::Thinking
    let kind = nth_chunk_kind(&app, &sid("s-1"), 0);
    assert!(
        matches!(kind, ChunkKind::Thinking),
        "expected Thinking kind, got {kind:?}"
    );

    // @step And the chunk's is_streaming flag is true
    assert!(
        nth_chunk_is_streaming(&app, &sid("s-1"), 0),
        "thinking chunk should be is_streaming=true while accumulating"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The "[Thinking]\n" prefix is applied by the renderer only
//           on lineIndex==0 of the first wrapped line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn thinking_prefix_applied_by_renderer_only_on_line_index_zero() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When the chunks subscriber forwards StreamChunk::Thinking { thinking: "first line\nsecond line" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::thinking("first line\nsecond line".to_string()),
    ));

    // @step Then the wrapped lines produced for that chunk are exactly the table
    let visible = session_lines(&app, &sid("s-1"));
    assert_eq!(
        visible,
        vec![
            "[Thinking]".to_string(),
            "first line".to_string(),
            "second line".to_string(),
        ],
    );

    // @step And the stored chunk.source.text is exactly "first line\nsecond line" (no prefix baked in)
    let stored = nth_chunk_source_text(&app, &sid("s-1"), 0);
    assert_eq!(stored, "first line\nsecond line");
    assert!(!stored.contains("[Thinking]"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: ToolCall finalises the in-flight thinking chunk and pushes
//           a tool-call card after it
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn tool_call_finalises_in_flight_thinking_and_pushes_card() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();
    // @step And the chunks subscriber has forwarded StreamChunk::Thinking { thinking: "Let me check the files" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::thinking("Let me check the files".to_string()),
    ));

    // @step When the chunks subscriber forwards ToolCall { tc-1, Read, {"file_path":"/etc/hosts"} } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk("tc-1", "Read", "{\"file_path\":\"/etc/hosts\"}"),
    ));

    // @step Then the s-1 scrollback contains exactly two rendered chunks in order
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 2);
    let kind0 = nth_chunk_kind(&app, &sid("s-1"), 0);
    let kind1 = nth_chunk_kind(&app, &sid("s-1"), 1);
    assert!(matches!(kind0, ChunkKind::Thinking));
    assert!(matches!(kind1, ChunkKind::ToolCall { .. }));

    let visible = session_lines(&app, &sid("s-1"));
    // The thinking block renders as "[Thinking]" header + body line(s),
    // then the tool-call card renders the parity header. RPC-388: Read has
    // no command/action_type key, so it falls into the default branch →
    // "{ file_path: '/etc/hosts' }".
    assert_eq!(visible[0], "[Thinking]");
    assert!(visible
        .iter()
        .any(|l| l == "\u{25CF} Read({ file_path: '/etc/hosts' })"));

    // @step And the Thinking chunk at index 0 has is_streaming false
    assert!(
        !nth_chunk_is_streaming(&app, &sid("s-1"), 0),
        "ToolCall must finalise the in-flight thinking chunk"
    );

    // @step And the SessionContext in_flight_thinking slot is None
    assert_eq!(session_in_flight_thinking(&app, &sid("s-1")), None);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A second Thinking delta after a ToolCall starts a fresh
//           thinking chunk
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn second_thinking_after_tool_call_starts_fresh_chunk() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();
    // @step And the chunks subscriber has forwarded Thinking { "first thought" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::thinking("first thought".to_string()),
    ));
    // @step And the chunks subscriber has forwarded ToolCall { tc-1, Read, ... } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk("tc-1", "Read", "{\"file_path\":\"/etc/hosts\"}"),
    ));

    // @step When the chunks subscriber forwards Thinking { "second thought" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::thinking("second thought".to_string()),
    ));

    // @step Then the s-1 scrollback contains exactly three rendered chunks in order
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 3);
    assert!(matches!(
        nth_chunk_kind(&app, &sid("s-1"), 0),
        ChunkKind::Thinking
    ));
    assert!(matches!(
        nth_chunk_kind(&app, &sid("s-1"), 1),
        ChunkKind::ToolCall { .. }
    ));
    assert!(matches!(
        nth_chunk_kind(&app, &sid("s-1"), 2),
        ChunkKind::Thinking
    ));
    assert_eq!(nth_chunk_source_text(&app, &sid("s-1"), 0), "first thought");
    assert_eq!(
        nth_chunk_source_text(&app, &sid("s-1"), 2),
        "second thought"
    );

    // @step And the chunk at index 0 has is_streaming false
    assert!(!nth_chunk_is_streaming(&app, &sid("s-1"), 0));
    // @step And the chunk at index 2 has is_streaming true
    assert!(nth_chunk_is_streaming(&app, &sid("s-1"), 2));
    // @step And the SessionContext in_flight_thinking slot is Some(2)
    assert_eq!(session_in_flight_thinking(&app, &sid("s-1")), Some(2));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: UserInput is a turn boundary that clears the in-flight
//           thinking slot
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn user_input_clears_in_flight_thinking_without_mutating() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();
    // @step And the chunks subscriber has forwarded Thinking { "first thought" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::thinking("first thought".to_string()),
    ));
    // @step And the chunks subscriber has forwarded UserInput { "carry on" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::user_input("carry on".to_string()),
    ));

    // @step When the chunks subscriber forwards Thinking { "second thought" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::thinking("second thought".to_string()),
    ));

    // @step Then the s-1 scrollback contains exactly three rendered chunks in order
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 3);
    assert!(matches!(
        nth_chunk_kind(&app, &sid("s-1"), 0),
        ChunkKind::Thinking
    ));
    assert!(matches!(
        nth_chunk_kind(&app, &sid("s-1"), 1),
        ChunkKind::UserInput
    ));
    assert!(matches!(
        nth_chunk_kind(&app, &sid("s-1"), 2),
        ChunkKind::Thinking
    ));
    assert_eq!(nth_chunk_source_text(&app, &sid("s-1"), 0), "first thought");
    assert_eq!(
        nth_chunk_source_text(&app, &sid("s-1"), 2),
        "second thought"
    );

    // @step And the chunk at index 0 has is_streaming true (left untouched by UserInput)
    assert!(
        nth_chunk_is_streaming(&app, &sid("s-1"), 0),
        "UserInput must NOT mutate the existing thinking chunk's is_streaming"
    );
    // @step And the chunk at index 2 has is_streaming true
    assert!(nth_chunk_is_streaming(&app, &sid("s-1"), 2));
    // @step And the SessionContext in_flight_thinking slot is Some(2)
    assert_eq!(session_in_flight_thinking(&app, &sid("s-1")), Some(2));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A Thinking delta arriving while in_flight_assistant is Some
//           inserts the thinking chunk BEFORE the assistant chunk
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn thinking_inserts_before_in_flight_assistant() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();
    // @step And the chunks subscriber has forwarded Text { "Reading" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::text("Reading".to_string()),
    ));
    // @step And the SessionContext in_flight_assistant slot is Some(0)
    assert_eq!(session_in_flight_assistant(&app, &sid("s-1")), Some(0));

    // @step When the chunks subscriber forwards Thinking { "Hmm" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::thinking("Hmm".to_string()),
    ));

    // @step Then the s-1 scrollback contains exactly two rendered chunks in order
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 2);
    assert!(matches!(
        nth_chunk_kind(&app, &sid("s-1"), 0),
        ChunkKind::Thinking
    ));
    assert!(matches!(
        nth_chunk_kind(&app, &sid("s-1"), 1),
        ChunkKind::AssistantText
    ));
    assert_eq!(nth_chunk_source_text(&app, &sid("s-1"), 0), "Hmm");
    assert_eq!(nth_chunk_source_text(&app, &sid("s-1"), 1), "Reading");

    // @step And the SessionContext in_flight_thinking slot is Some(0)
    assert_eq!(session_in_flight_thinking(&app, &sid("s-1")), Some(0));
    // @step And the SessionContext in_flight_assistant slot is Some(1)
    assert_eq!(
        session_in_flight_assistant(&app, &sid("s-1")),
        Some(1),
        "in_flight_assistant must be bumped by 1 when a thinking chunk is spliced before it"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Done clears the in_flight_thinking slot without mutating
//           the existing thinking chunk
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn done_clears_in_flight_thinking_without_mutating_chunk() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();
    // @step And the chunks subscriber has forwarded Thinking { "settled thought" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::thinking("settled thought".to_string()),
    ));

    // @step When the chunks subscriber forwards Done for s-1
    app.dispatch(Action::ChunkReceived(sid("s-1"), StreamChunk::Done));

    // @step Then the s-1 scrollback contains exactly one rendered chunk
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 1);
    // @step And that chunk's source.text equals "settled thought"
    assert_eq!(
        nth_chunk_source_text(&app, &sid("s-1"), 0),
        "settled thought"
    );
    // @step And that chunk's is_streaming flag is true (Done does not mutate the thinking chunk)
    assert!(
        nth_chunk_is_streaming(&app, &sid("s-1"), 0),
        "Done must NOT mutate is_streaming on the thinking chunk"
    );
    // @step And the SessionContext in_flight_thinking slot is None
    assert_eq!(session_in_flight_thinking(&app, &sid("s-1")), None);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A second Thinking delta after Done starts a fresh thinking
//           chunk (turn boundary)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn second_thinking_after_done_starts_fresh_chunk() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();
    // @step And the chunks subscriber has forwarded Thinking { "old thought" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::thinking("old thought".to_string()),
    ));
    // @step And the chunks subscriber has forwarded Done for s-1
    app.dispatch(Action::ChunkReceived(sid("s-1"), StreamChunk::Done));

    // @step When the chunks subscriber forwards Thinking { "new thought" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::thinking("new thought".to_string()),
    ));

    // @step Then the s-1 scrollback contains exactly two rendered chunks in order
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 2);
    assert!(matches!(
        nth_chunk_kind(&app, &sid("s-1"), 0),
        ChunkKind::Thinking
    ));
    assert!(matches!(
        nth_chunk_kind(&app, &sid("s-1"), 1),
        ChunkKind::Thinking
    ));
    assert_eq!(nth_chunk_source_text(&app, &sid("s-1"), 0), "old thought");
    assert_eq!(nth_chunk_source_text(&app, &sid("s-1"), 1), "new thought");
    // @step And the SessionContext in_flight_thinking slot is Some(1)
    assert_eq!(session_in_flight_thinking(&app, &sid("s-1")), Some(1));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Error clears the in_flight_thinking slot without mutating
//           the existing thinking chunk
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn error_clears_in_flight_thinking_without_mutating_chunk() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();
    // @step And the chunks subscriber has forwarded Thinking { "mid thought" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::thinking("mid thought".to_string()),
    ));

    // @step When the chunks subscriber forwards Error { "rate limit exceeded" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::error("rate limit exceeded".to_string()),
    ));

    // @step Then the s-1 scrollback contains exactly two rendered chunks in order
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 2);
    assert!(matches!(
        nth_chunk_kind(&app, &sid("s-1"), 0),
        ChunkKind::Thinking
    ));
    assert!(matches!(
        nth_chunk_kind(&app, &sid("s-1"), 1),
        ChunkKind::Error
    ));
    assert_eq!(nth_chunk_source_text(&app, &sid("s-1"), 0), "mid thought");
    assert_eq!(
        nth_chunk_source_text(&app, &sid("s-1"), 1),
        "API Error: rate limit exceeded"
    );

    // @step And the chunk at index 0 has is_streaming true (Error does not mutate it)
    assert!(
        nth_chunk_is_streaming(&app, &sid("s-1"), 0),
        "Error must NOT mutate is_streaming on the thinking chunk"
    );
    // @step And the SessionContext in_flight_thinking slot is None
    assert_eq!(session_in_flight_thinking(&app, &sid("s-1")), None);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Interrupted clears the in_flight_thinking slot without
//           mutating the existing thinking chunk
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn interrupted_clears_in_flight_thinking_without_mutating_chunk() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();
    // @step And the chunks subscriber has forwarded Thinking { "interrupted mid-flight" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::thinking("interrupted mid-flight".to_string()),
    ));

    // @step When the chunks subscriber forwards Interrupted {} for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::interrupted(Vec::new()),
    ));

    // @step Then the s-1 scrollback contains exactly two rendered chunks in order
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 2);
    assert!(matches!(
        nth_chunk_kind(&app, &sid("s-1"), 0),
        ChunkKind::Thinking
    ));
    assert!(matches!(
        nth_chunk_kind(&app, &sid("s-1"), 1),
        ChunkKind::Interrupted
    ));
    assert_eq!(
        nth_chunk_source_text(&app, &sid("s-1"), 0),
        "interrupted mid-flight"
    );

    // @step And the chunk at index 0 has is_streaming true (Interrupted does not mutate it)
    assert!(
        nth_chunk_is_streaming(&app, &sid("s-1"), 0),
        "Interrupted must NOT mutate is_streaming on the thinking chunk"
    );
    // @step And the SessionContext in_flight_thinking slot is None
    assert_eq!(session_in_flight_thinking(&app, &sid("s-1")), None);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Screenshot transcript — four deltas of one logical thought
//           produce one scrollback row
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn screenshot_transcript_four_deltas_produce_one_scrollback_row() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();

    let deltas = [
        "The user is asking about ",
        "how well a \"card\" was done. This is likely referring to a ",
        "work unit in fspec. Let me check the board to see what ",
        "work units are in progress or recently completed.",
    ];

    // @step When the chunks subscriber forwards the following Thinking deltas in order for s-1
    for d in deltas.iter() {
        app.dispatch(Action::ChunkReceived(
            sid("s-1"),
            StreamChunk::thinking((*d).to_string()),
        ));
    }

    // @step Then the s-1 scrollback contains exactly one rendered chunk
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 1);

    // @step And the rendered first wrapped line of that chunk is "[Thinking]"
    let visible = session_lines(&app, &sid("s-1"));
    assert_eq!(visible[0], "[Thinking]");

    // @step And that chunk's source.text concatenates all four deltas with no "[Thinking]" prefix baked in
    let stored = nth_chunk_source_text(&app, &sid("s-1"), 0);
    let expected: String = deltas.iter().copied().collect();
    assert_eq!(stored, expected);
    assert!(
        !stored.contains("[Thinking]"),
        "stored source.text must not bake [Thinking]; got {stored:?}"
    );

    // @step And the SessionContext in_flight_thinking slot is Some(0)
    assert_eq!(session_in_flight_thinking(&app, &sid("s-1")), Some(0));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Full round-trip — user asks, agent thinks, calls tool,
//           thinks again, answers, finishes
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn full_round_trip_thinking_renders_five_chunks_in_order() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When the chunks subscriber forwards the chunks in order for s-1
    let chunks = vec![
        StreamChunk::user_input("how is RPC-093 going?".to_string()),
        StreamChunk::thinking("Let me ".to_string()),
        StreamChunk::thinking("check the card".to_string()),
        tool_call_chunk("tc-1", "Fspec", "{\"command\":\"show-work-unit\"}"),
        tool_result_chunk("tc-1", "ok", false),
        StreamChunk::thinking("It is in ".to_string()),
        StreamChunk::thinking("specifying".to_string()),
        StreamChunk::text("RPC-093 is in specifying.".to_string()),
        StreamChunk::Done,
    ];
    for c in chunks {
        app.dispatch(Action::ChunkReceived(sid("s-1"), c));
    }

    // @step Then the s-1 scrollback contains exactly five rendered chunks in order
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 5);
    assert!(matches!(
        nth_chunk_kind(&app, &sid("s-1"), 0),
        ChunkKind::UserInput
    ));
    assert!(matches!(
        nth_chunk_kind(&app, &sid("s-1"), 1),
        ChunkKind::Thinking
    ));
    assert!(matches!(
        nth_chunk_kind(&app, &sid("s-1"), 2),
        ChunkKind::ToolCall { .. }
    ));
    assert!(matches!(
        nth_chunk_kind(&app, &sid("s-1"), 3),
        ChunkKind::Thinking
    ));
    assert!(matches!(
        nth_chunk_kind(&app, &sid("s-1"), 4),
        ChunkKind::AssistantText
    ));

    assert_eq!(
        nth_chunk_source_text(&app, &sid("s-1"), 0),
        "how is RPC-093 going?"
    );
    assert_eq!(
        nth_chunk_source_text(&app, &sid("s-1"), 1),
        "Let me check the card"
    );
    // tool-call chunk text already includes header + result body
    let tool_text = nth_chunk_source_text(&app, &sid("s-1"), 2);
    assert!(tool_text.contains("Fspec(show-work-unit)"));
    assert!(tool_text.contains("ok"));
    assert_eq!(
        nth_chunk_source_text(&app, &sid("s-1"), 3),
        "It is in specifying"
    );
    assert_eq!(
        nth_chunk_source_text(&app, &sid("s-1"), 4),
        "RPC-093 is in specifying."
    );

    // @step And the chunk at index 1 has is_streaming false (finalised by the ToolCall at index 2)
    assert!(!nth_chunk_is_streaming(&app, &sid("s-1"), 1));
    // @step And the chunk at index 3 has is_streaming true (Done does not mutate it)
    assert!(nth_chunk_is_streaming(&app, &sid("s-1"), 3));
    // @step And the chunk at index 4 has is_streaming false (Done finalised the assistant text)
    assert!(!nth_chunk_is_streaming(&app, &sid("s-1"), 4));

    // @step And the SessionContext in_flight_thinking slot is None
    assert_eq!(session_in_flight_thinking(&app, &sid("s-1")), None);
    // @step And the SessionContext in_flight_assistant slot is None
    assert_eq!(session_in_flight_assistant(&app, &sid("s-1")), None);

    // @step And no chunk has "[Thinking]" baked into its stored source.text
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .unwrap();
    for (i, c) in ctx.scrollback.visible_window(1024).iter().enumerate() {
        let stored = c
            .source
            .as_ref()
            .map(|s| s.text.clone())
            .unwrap_or_default();
        assert!(
            !stored.contains("[Thinking]"),
            "chunk {i} must not bake [Thinking]; got {stored:?}"
        );
    }
}
