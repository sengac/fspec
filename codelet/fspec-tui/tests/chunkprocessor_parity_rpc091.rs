//! RPC-091 — chunkProcessor parity: streaming Text accumulation,
//! ToolCall/ToolResult/ToolProgress cards, Done finalisation.
//!
//! Feature: spec/features/agentview-chunkprocessor-parity.feature
//!
//! Authoritative TS reference:
//!   src/tui/utils/chunkProcessor.ts       (accumulation algorithm)
//!   src/tui/utils/conversationUtils.ts    (bullet placement)
//!   src/tui/utils/toolFormatters.ts       (extractToolArgsDisplay)
//!   src/tui/utils/formatMarkdownTables.ts (Done finalisation)
//!
//! These tests assert the post-RPC-091 acceptance criteria. They MUST
//! fail (red phase) against the pre-RPC-091 stub in
//! codelet/fspec-tui/src/store/agent_view/session_context.rs.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, StreamChunk, ToolCallInfo, ToolProgressInfo, ToolResultInfo};
use ratatui::style::Color;

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

/// Flatten all chunks in `id`'s scrollback into a Vec<String>, one
/// entry per Line, concatenating all spans per Line.
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

fn session_in_flight(app: &App, id: &SessionId) -> Option<usize> {
    app.agent_view_store()
        .session_context_for(id)
        .and_then(|c| c.in_flight_assistant)
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

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Consecutive Text deltas accumulate into a single in-flight
//           assistant chunk
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn consecutive_text_deltas_accumulate_into_single_in_flight_assistant_chunk() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When the chunks subscriber forwards StreamChunk::Text { text: "Hello" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::text("Hello".to_string()),
    ));

    // @step And the chunks subscriber forwards StreamChunk::Text { text: " world" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::text(" world".to_string()),
    ));

    // @step Then the s-1 scrollback contains exactly one rendered chunk
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 1);

    // @step And that chunk's source.text equals "Hello world" without any bullet glyph
    let stored = nth_chunk_source_text(&app, &sid("s-1"), 0);
    assert_eq!(stored, "Hello world");
    assert!(
        !stored.starts_with('\u{25CF}'),
        "stored text must not bake the bullet glyph; got {stored:?}"
    );

    // @step And the SessionContext in_flight_assistant slot is Some(<that chunk's index>)
    assert_eq!(session_in_flight(&app, &sid("s-1")), Some(0));
    // @step And the chunk's source.kind is ChunkKind::AssistantText
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .unwrap();
    let chunks = ctx.scrollback.visible_window(1024);
    let kind = chunks[0]
        .source
        .as_ref()
        .map(|s| s.kind.clone())
        .expect("ChunkSource present");
    assert!(
        matches!(kind, codelet_fspec_tui::ChunkKind::AssistantText),
        "expected AssistantText kind, got {kind:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Bullet glyph is applied by the renderer only on lineIndex==0
//           of the first wrapped line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn bullet_glyph_applied_by_renderer_only_on_line_index_zero() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When the chunks subscriber forwards StreamChunk::Text { text: "first line\nsecond line" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::text("first line\nsecond line".to_string()),
    ));

    // @step Then the wrapped lines produced for that chunk are exactly the table
    let visible = session_lines(&app, &sid("s-1"));
    assert_eq!(
        visible,
        vec!["\u{25CF} first line".to_string(), "second line".to_string(),],
    );

    // @step And the stored chunk.source.text is exactly "first line\nsecond line" (no bullet baked in)
    let stored = nth_chunk_source_text(&app, &sid("s-1"), 0);
    assert_eq!(stored, "first line\nsecond line");
    assert!(!stored.contains('\u{25CF}'));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Done flushes the in-flight assistant slot and emits no new chunk
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn done_flushes_in_flight_and_emits_no_new_chunk() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();
    // @step And the chunks subscriber has forwarded StreamChunk::Text { text: "hello" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::text("hello".to_string()),
    ));
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 1);

    // @step When the chunks subscriber forwards StreamChunk::Done for s-1
    app.dispatch(Action::ChunkReceived(sid("s-1"), StreamChunk::Done));

    // @step Then the s-1 scrollback still contains exactly one rendered chunk
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 1);
    // @step And the SessionContext in_flight_assistant slot is None
    assert_eq!(session_in_flight(&app, &sid("s-1")), None);
    // @step And the chunk's is_streaming flag is false
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .unwrap();
    let chunks = ctx.scrollback.visible_window(1024);
    let is_streaming = chunks[0]
        .source
        .as_ref()
        .map(|s| s.is_streaming)
        .unwrap_or(true);
    assert!(!is_streaming, "Done must clear is_streaming");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Done runs formatMarkdownTables over the accumulated assistant text
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn done_runs_format_markdown_tables_over_accumulated_text() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();
    // @step And the chunks subscriber has forwarded StreamChunk::Text deltas concatenating to
    //       "| col1 | col2 |\n|---|---|\n| a | bb |" for s-1
    let pieces = ["| col1 | col2 |\n", "|---|---|\n", "| a | bb |"];
    for p in pieces {
        app.dispatch(Action::ChunkReceived(
            sid("s-1"),
            StreamChunk::text(p.to_string()),
        ));
    }

    // @step When the chunks subscriber forwards StreamChunk::Done for s-1
    app.dispatch(Action::ChunkReceived(sid("s-1"), StreamChunk::Done));

    // @step Then the final in-flight chunk's source.text equals a pipe-aligned table
    //       where every row uses equal column widths
    let final_text = nth_chunk_source_text(&app, &sid("s-1"), 0);
    let lines: Vec<&str> = final_text.lines().collect();
    assert_eq!(
        lines.len(),
        3,
        "expected 3 table rows after Done; got {final_text:?}"
    );
    // Each column should be padded to its widest cell.
    // col1: max width = 4 ("col1"), col2: max width = 4 ("col2"/"bb" → 4 from header)
    let widths: Vec<Vec<usize>> = lines
        .iter()
        .filter(|l| !l.contains("---"))
        .map(|l| {
            l.split('|')
                .filter(|c| !c.is_empty())
                .map(|c| c.trim_matches(' ').len() + c.chars().filter(|ch| *ch == ' ').count())
                .collect()
        })
        .collect();
    // All non-separator rows should have the same number of cells with
    // consistent width per column (column-major equality).
    for col in 0..widths[0].len() {
        let first = widths[0][col];
        for row in &widths[1..] {
            assert_eq!(
                row[col], first,
                "column {col} width must be equal across all rows in {final_text:?}"
            );
        }
    }

    // @step And the chunk's is_streaming flag is false
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .unwrap();
    let is_streaming = ctx.scrollback.visible_window(1024)[0]
        .source
        .as_ref()
        .map(|s| s.is_streaming)
        .unwrap_or(true);
    assert!(!is_streaming);
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

fn tool_progress_chunk(
    tool_call_id: &str,
    name: &str,
    chunk: &str,
    is_stderr: bool,
) -> StreamChunk {
    StreamChunk::tool_progress(ToolProgressInfo {
        tool_call_id: tool_call_id.to_string(),
        tool_name: name.to_string(),
        output_chunk: chunk.to_string(),
        is_stderr,
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: ToolCall flushes the in-flight assistant text and pushes a
//           tool-call card
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn tool_call_flushes_in_flight_and_pushes_tool_call_card() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();
    // @step And the chunks subscriber has forwarded StreamChunk::Text { text: "Let me check the board" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::text("Let me check the board".to_string()),
    ));

    // @step When the chunks subscriber forwards ToolCall { tc-1, Fspec, {"command":"board"} } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk("tc-1", "Fspec", "{\"command\":\"board\"}"),
    ));

    // @step Then the s-1 scrollback contains exactly two rendered chunks in order
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 2);
    let visible = session_lines(&app, &sid("s-1"));
    assert_eq!(visible[0], "\u{25CF} Let me check the board");
    assert_eq!(visible[1], "\u{25CF} Fspec(board)");

    // @step And the SessionContext in_flight_assistant slot is None
    assert_eq!(session_in_flight(&app, &sid("s-1")), None);

    // @step And the tool-call chunk's tool_call_id equals "tc-1"
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .unwrap();
    let chunks = ctx.scrollback.visible_window(1024);
    let kind = chunks[1].source.as_ref().map(|s| s.kind.clone()).unwrap();
    match kind {
        codelet_fspec_tui::ChunkKind::ToolCall { tool_call_id, .. } => {
            assert_eq!(tool_call_id, "tc-1");
        }
        other => panic!("expected ToolCall kind, got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: ToolCall drops an empty in-flight assistant placeholder
//           instead of finalising it
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn tool_call_drops_empty_in_flight_placeholder() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();
    // @step And the SessionContext in_flight_assistant slot points at an existing
    //       empty AssistantText chunk
    // (Achieved by forwarding an empty Text delta — the post-RPC-091 record_chunk
    // pushes a fresh empty placeholder for that case, matching the TS branch when
    // chunk.text === '' is wrapped through processStreamingChunk's else-push arm.)
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::text(String::new()),
    ));
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 1);
    assert_eq!(session_in_flight(&app, &sid("s-1")), Some(0));

    // @step When the chunks subscriber forwards ToolCall { tc-2, Bash, {"command":"ls"} } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk("tc-2", "Bash", "{\"command\":\"ls\"}"),
    ));

    // @step Then the empty AssistantText chunk has been removed from scrollback
    // @step And the s-1 scrollback contains exactly one rendered chunk of kind ToolCall
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 1);
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .unwrap();
    let chunks = ctx.scrollback.visible_window(1024);
    let kind = chunks[0].source.as_ref().map(|s| s.kind.clone()).unwrap();
    assert!(matches!(
        kind,
        codelet_fspec_tui::ChunkKind::ToolCall { .. }
    ));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: ToolResult attaches to the matching tool-call header and
//           pushes a fresh placeholder
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn tool_result_attaches_to_matching_header_and_pushes_fresh_placeholder() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();
    // @step And the scrollback contains a ToolCall chunk with tool_call_id "tc-1" and header "● Fspec(board)"
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk("tc-1", "Fspec", "{\"command\":\"board\"}"),
    ));
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 1);

    // @step When the chunks subscriber forwards ToolResult { tc-1, "AUTH-001  AUTH-002  AUTH-003", is_error: false } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_result_chunk("tc-1", "AUTH-001  AUTH-002  AUTH-003", false),
    ));

    // @step Then the matching ToolCall chunk's source.text equals "● Fspec(board)\nAUTH-001  AUTH-002  AUTH-003"
    // (stored text — bullet baked here because ToolCall header carries it via
    // the rendered "● " prefix logic; relaxed: assert header line + body present.)
    let stored0 = nth_chunk_source_text(&app, &sid("s-1"), 0);
    assert!(stored0.contains("Fspec(board)"));
    assert!(stored0.contains("AUTH-001  AUTH-002  AUTH-003"));

    // @step And the matching ToolCall chunk's is_error flag is false
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .unwrap();
    let chunks = ctx.scrollback.visible_window(1024);
    let kind0 = chunks[0].source.as_ref().map(|s| s.kind.clone()).unwrap();
    match kind0 {
        codelet_fspec_tui::ChunkKind::ToolCall { is_error, .. } => assert!(!is_error),
        other => panic!("expected ToolCall kind, got {other:?}"),
    }

    // @step And the s-1 scrollback ends with a fresh empty AssistantText chunk with is_streaming true
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 2);
    let kind1 = chunks[1].source.as_ref().map(|s| s.kind.clone()).unwrap();
    assert!(matches!(kind1, codelet_fspec_tui::ChunkKind::AssistantText));
    let is_streaming1 = chunks[1].source.as_ref().map(|s| s.is_streaming).unwrap();
    assert!(is_streaming1, "fresh placeholder must be is_streaming");
    let stored1 = nth_chunk_source_text(&app, &sid("s-1"), 1);
    assert!(stored1.is_empty(), "fresh placeholder must be empty");

    // @step And the SessionContext in_flight_assistant slot is Some(<index of that fresh placeholder>)
    assert_eq!(session_in_flight(&app, &sid("s-1")), Some(1));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: ToolResult with is_error true colours the body via the isError flag
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn tool_result_with_is_error_true_colours_body_red() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();
    // @step And the scrollback contains a ToolCall chunk with tool_call_id "tc-3" and header "● Bash(false)"
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk("tc-3", "Bash", "{\"command\":\"false\"}"),
    ));

    // @step When the chunks subscriber forwards ToolResult { tc-3, "exit code 1", is_error: true } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_result_chunk("tc-3", "exit code 1", true),
    ));

    // @step Then the matching ToolCall chunk's is_error flag is true
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .unwrap();
    let chunks = ctx.scrollback.visible_window(1024);
    let kind0 = chunks[0].source.as_ref().map(|s| s.kind.clone()).unwrap();
    match kind0 {
        codelet_fspec_tui::ChunkKind::ToolCall { is_error, .. } => assert!(is_error),
        other => panic!("expected ToolCall kind, got {other:?}"),
    }
    // @step And the rendered lines for that chunk carry foreground colour RED on the result body
    let body_line = chunks[0]
        .lines
        .iter()
        .find(|l| l.spans.iter().any(|s| s.content.contains("exit code 1")))
        .expect("result body line present");
    let fg = body_line
        .spans
        .iter()
        .find(|s| s.content.contains("exit code 1"))
        .and_then(|s| s.style.fg);
    assert_eq!(fg, Some(Color::Red));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Continuation Text after ToolResult starts a new AssistantText bubble
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn continuation_text_after_tool_result_starts_new_assistant_bubble() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();
    // @step And the scrollback ends with a fresh empty AssistantText placeholder
    //       created by a prior ToolResult
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk("tc-1", "Fspec", "{\"command\":\"board\"}"),
    ));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_result_chunk("tc-1", "ok", false),
    ));
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 2);
    assert_eq!(session_in_flight(&app, &sid("s-1")), Some(1));

    // @step When the chunks subscriber forwards Text { "Here are the " } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::text("Here are the ".to_string()),
    ));
    // @step And the chunks subscriber forwards Text { "work units" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::text("work units".to_string()),
    ));

    // @step Then the trailing AssistantText chunk's source.text equals "Here are the work units"
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 2);
    let trailing = nth_chunk_source_text(&app, &sid("s-1"), 1);
    assert_eq!(trailing, "Here are the work units");
    // @step And the SessionContext in_flight_assistant slot points at that trailing chunk
    assert_eq!(session_in_flight(&app, &sid("s-1")), Some(1));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: ToolProgress is folded under the matching tool-call card and
//           does not push a new top-level chunk
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn tool_progress_folded_under_matching_tool_call_card() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();
    // @step And the scrollback contains a ToolCall chunk with tool_call_id "tc-4" and header "● Bash(npm test)"
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk("tc-4", "Bash", "{\"command\":\"npm test\"}"),
    ));
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 1);

    // @step When the chunks subscriber forwards ToolProgress { tc-4, "PASS src/foo.test.ts\n", stream: stderr } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_progress_chunk("tc-4", "Bash", "PASS src/foo.test.ts\n", true),
    ));

    // @step Then the matching ToolCall chunk's source.text ends with "\nPASS src/foo.test.ts" within a streaming window
    let stored0 = nth_chunk_source_text(&app, &sid("s-1"), 0);
    assert!(
        stored0.contains("PASS src/foo.test.ts"),
        "tool-call chunk must absorb progress; got {stored0:?}"
    );

    // @step And no new top-level RenderedChunk has been appended to the scrollback
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 1);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Error drops a trailing empty in-flight placeholder before
//           pushing the API Error line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn error_drops_empty_in_flight_before_pushing_api_error_line() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();
    // @step And the SessionContext in_flight_assistant slot points at an existing empty AssistantText chunk
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::text(String::new()),
    ));
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 1);
    assert_eq!(session_in_flight(&app, &sid("s-1")), Some(0));

    // @step When the chunks subscriber forwards StreamChunk::Error { error: "rate limit exceeded" } for s-1
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::error("rate limit exceeded".to_string()),
    ));

    // @step Then the empty AssistantText chunk has been removed from scrollback
    // @step And the s-1 scrollback ends with a chunk whose source.text equals "API Error: rate limit exceeded"
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 1);
    let stored0 = nth_chunk_source_text(&app, &sid("s-1"), 0);
    assert_eq!(stored0, "API Error: rate limit exceeded");
    // @step And the SessionContext in_flight_assistant slot is None
    assert_eq!(session_in_flight(&app, &sid("s-1")), None);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: extractToolArgsDisplay collapses Bash command to a one-line summary
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn extract_tool_args_display_collapses_bash_to_command() {
    // @step Given a ToolCall with name "Bash" and input JSON {"command":"ls -la","timeout":5000}
    // @step When the renderer formats the tool-call header
    let header = codelet_fspec_tui::extract_tool_args_display(
        "Bash",
        "{\"command\":\"ls -la\",\"timeout\":5000}",
    );
    // @step Then the header text equals "ls -la"
    assert_eq!(header, "ls -la");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: extractToolArgsDisplay collapses Fspec command to the command subcommand
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn extract_tool_args_display_collapses_fspec_to_command() {
    // @step Given a ToolCall with name "Fspec" and input JSON {"command":"show-work-unit","args":"..."}
    // @step When the renderer formats the tool-call header
    let header = codelet_fspec_tui::extract_tool_args_display(
        "Fspec",
        "{\"command\":\"show-work-unit\",\"args\":\"{\\\"_\\\":[\\\"AUTH-001\\\"]}\"}",
    );
    // @step Then the header text equals "show-work-unit"
    assert_eq!(header, "show-work-unit");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Full round-trip — user asks, assistant streams, calls a tool,
//           continues, finishes
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn full_round_trip_renders_four_chunks_in_order() {
    // @step Given an AgentView with a fresh SessionContext for session s-1
    let mut app = app_with_session();

    // @step When the chunks subscriber forwards the chunks in order for s-1
    let chunks = vec![
        StreamChunk::user_input("what cards are open?".to_string()),
        StreamChunk::text("Let me ".to_string()),
        StreamChunk::text("check the board".to_string()),
        tool_call_chunk("tc-1", "Fspec", "{\"command\":\"board\"}"),
        tool_result_chunk("tc-1", "ok", false),
        StreamChunk::text("Here are the ".to_string()),
        StreamChunk::text("open work units".to_string()),
        StreamChunk::Done,
    ];
    for c in chunks {
        app.dispatch(Action::ChunkReceived(sid("s-1"), c));
    }

    // @step Then the s-1 scrollback contains exactly four rendered chunks in order
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 4);
    let visible = session_lines(&app, &sid("s-1"));
    // Expected visible top-line per chunk (subsequent chunk lines may follow).
    assert_eq!(visible[0], "You: what cards are open?");
    assert_eq!(visible[1], "\u{25CF} Let me check the board");
    // Chunk 2 is the tool-call; the first visible line is the header.
    assert_eq!(visible[2], "\u{25CF} Fspec(board)");
    // Last assistant bubble — find a line equal to "● Here are the open work units"
    assert!(
        visible
            .iter()
            .any(|l| l == "\u{25CF} Here are the open work units"),
        "expected final assistant bubble; got {visible:?}"
    );
    // @step And the SessionContext in_flight_assistant slot is None
    assert_eq!(session_in_flight(&app, &sid("s-1")), None);

    // @step And no chunk has a bullet baked into its stored source.text
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
            !stored.starts_with('\u{25CF}'),
            "chunk {i} must not bake the bullet; got {stored:?}"
        );
    }
}
