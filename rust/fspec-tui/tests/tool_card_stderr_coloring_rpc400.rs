//! RPC-400 — stderr lines in tool cards must render red and strip the
//! `⚠stderr⚠` sentinel (TypeScript parity).
//!
//! Feature: spec/features/tool-card-stderr-line-coloring.feature
//!
//! These tests drive the REAL store/render path (mirroring the RPC-389/391/399
//! harness): push a ToolCall chunk, then ToolProgress (streaming) and/or
//! ToolResult (settle) via the App, and assert the wrapped `Line`/`Span`
//! styles of the tool-call card — inspecting `span.style.fg == Color::Red`
//! and asserting the marker text never survives to a rendered span.
//!
//! They MUST fail (red phase) against current code, which:
//!   * drops `is_stderr` in `handle_tool_progress` (no marker added), and
//!   * never strips the marker nor colors per-line stderr in `wrap_source` /
//!     `style_modal_lines`.
//!
//! The marker value `⚠stderr⚠` is fixed by the feature and matches
//! rust/tools `bash_output.rs` `STDERR_MARKER`; fspec-tui does not depend
//! on codelet-tools, so this test pins the literal locally.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::store::agent_view::diff_decode::style_modal_lines;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, StreamChunk, ToolCallInfo, ToolProgressInfo, ToolResultInfo};
use ratatui::style::Color;
use ratatui::text::Line;

mod common;
use common::MockBackend;

/// The stderr sentinel marker (parity with codelet-tools `STDERR_MARKER`).
const STDERR_MARKER: &str = "⚠stderr⚠";

const DIFF_BG_REMOVED: Color = Color::Rgb(139, 0, 0);
const DIFF_BG_ADDED: Color = Color::Rgb(0, 100, 0);

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

/// All wrapped `Line`s of the FIRST chunk (the tool-call card) in s-1.
fn first_chunk_lines(app: &App) -> Vec<Line<'static>> {
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .expect("session context exists");
    ctx.scrollback
        .visible_window(4096)
        .first()
        .map(|c| c.lines.clone())
        .unwrap_or_default()
}

/// The stable `seq` of the first chunk (tool-call card).
fn first_chunk_seq(app: &App) -> u64 {
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .expect("session context exists");
    ctx.scrollback.visible_window(4096).first().unwrap().seq
}

/// Concatenated text of a `Line`.
fn line_text(line: &Line<'static>) -> String {
    line.spans.iter().map(|s| s.content.as_ref()).collect()
}

/// True if EVERY span carrying the given text is red (fg == Color::Red).
fn line_is_red(line: &Line<'static>) -> bool {
    !line.spans.is_empty() && line.spans.iter().all(|s| s.style.fg == Some(Color::Red))
}

/// Concatenated text of ONE modal row (`Vec<Span>`).
fn row_text(row: &[ratatui::text::Span<'static>]) -> String {
    row.iter().map(|s| s.content.as_ref()).collect()
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Settled stderr line renders red with the marker stripped
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn settled_stderr_line_renders_red_with_marker_stripped() {
    let mut app = app_with_session();
    // @step Given a settled tool card whose command succeeded
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk("tc-1", "Bash", "{\"command\":\"cargo build\"}"),
    ));
    // @step And a body line "⚠stderr⚠warning: unused import"
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_result_chunk("tc-1", "⚠stderr⚠warning: unused import", false),
    ));

    // @step When the tool card body is rendered in the scrollback
    let lines = first_chunk_lines(&app);

    // @step Then the line displays as "warning: unused import"
    let line = lines
        .iter()
        .find(|l| line_text(l).contains("warning: unused import"))
        .expect("stderr body line present");
    assert_eq!(
        line_text(line),
        "warning: unused import",
        "marker must be stripped; got {:?}",
        line_text(line)
    );

    // @step And the line is styled red
    assert!(
        line_is_red(line),
        "stderr line must be red; got {:?}",
        line.spans
    );

    // @step And no "⚠stderr⚠" marker text is visible
    assert!(
        !lines.iter().any(|l| line_text(l).contains(STDERR_MARKER)),
        "marker text must not reach screen; got {:?}",
        lines.iter().map(line_text).collect::<Vec<_>>()
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A successful command shows only its stderr lines red
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn successful_command_shows_only_stderr_lines_red() {
    let mut app = app_with_session();
    // @step Given a settled tool card whose command succeeded
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk("tc-1", "Bash", "{\"command\":\"cargo build\"}"),
    ));
    // @step And a body with line "Compiling main.rs" then line "⚠stderr⚠warning: unused var"
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_result_chunk(
            "tc-1",
            "Compiling main.rs\n⚠stderr⚠warning: unused var",
            false,
        ),
    ));

    // @step When the tool card body is rendered in the scrollback
    let lines = first_chunk_lines(&app);

    // @step Then the line "Compiling main.rs" is styled in the normal body color
    let stdout_line = lines
        .iter()
        .find(|l| line_text(l).contains("Compiling main.rs"))
        .expect("stdout body line present");
    assert!(
        stdout_line
            .spans
            .iter()
            .all(|s| s.style.fg != Some(Color::Red)),
        "stdout line must NOT be red; got {:?}",
        stdout_line.spans
    );

    // @step And the line "warning: unused var" is styled red
    let stderr_line = lines
        .iter()
        .find(|l| line_text(l).contains("warning: unused var"))
        .expect("stderr body line present");
    assert_eq!(line_text(stderr_line), "warning: unused var");
    assert!(
        line_is_red(stderr_line),
        "stderr line must be red; got {:?}",
        stderr_line.spans
    );

    // @step And no "⚠stderr⚠" marker text is visible
    assert!(
        !lines.iter().any(|l| line_text(l).contains(STDERR_MARKER)),
        "marker text must not reach screen; got {:?}",
        lines.iter().map(line_text).collect::<Vec<_>>()
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A failed command renders the whole body red with no marker visible
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn failed_command_renders_whole_body_red_with_no_marker() {
    let mut app = app_with_session();
    // @step Given a settled tool card whose command failed
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk("tc-1", "Bash", "{\"command\":\"cargo build\"}"),
    ));
    // @step And a body mixing stdout with line "⚠stderr⚠error: cannot find value"
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_result_chunk(
            "tc-1",
            "Compiling main.rs\n⚠stderr⚠error: cannot find value",
            true,
        ),
    ));

    // @step When the tool card body is rendered in the scrollback
    let lines = first_chunk_lines(&app);

    // @step Then every body line is styled red
    // The header line (first `● Bash(...)`) is not a body line; assert the
    // stdout + stderr body lines are both red.
    let body: Vec<&Line<'static>> = lines
        .iter()
        .filter(|l| {
            let t = line_text(l);
            t.contains("Compiling main.rs") || t.contains("error: cannot find value")
        })
        .collect();
    assert_eq!(body.len(), 2, "expected 2 body lines; got {body:?}");
    for l in &body {
        assert!(
            line_is_red(l),
            "every body line must be red; got {:?}",
            l.spans
        );
    }

    // @step And no "⚠stderr⚠" marker text is visible
    assert!(
        !lines.iter().any(|l| line_text(l).contains(STDERR_MARKER)),
        "marker text must not reach screen; got {:?}",
        lines.iter().map(line_text).collect::<Vec<_>>()
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Live stderr progress is prefixed with the marker so it renders red
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn live_stderr_progress_prefixed_with_marker_and_red() {
    let mut app = app_with_session();
    // @step Given a streaming tool card
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk("tc-1", "Bash", "{\"command\":\"cargo run\"}"),
    ));

    // @step When a ToolProgress chunk "error: boom\nmore" arrives with is_stderr true
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_progress_chunk("tc-1", "Bash", "error: boom\nmore", true),
    ));

    // @step Then the card body gains the lines "⚠stderr⚠error: boom" and "⚠stderr⚠more"
    // The marker is inserted into `source.text` by `handle_tool_progress`.
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .expect("session context exists");
    let source_text = ctx
        .scrollback
        .visible_window(4096)
        .first()
        .and_then(|c| c.source.as_ref())
        .map(|s| s.text.clone())
        .unwrap_or_default();
    assert!(
        source_text.contains("⚠stderr⚠error: boom"),
        "source text must carry marker-prefixed line; got {source_text:?}"
    );
    assert!(
        source_text.contains("⚠stderr⚠more"),
        "source text must carry marker-prefixed line; got {source_text:?}"
    );

    // @step And both lines are styled red when rendered
    let lines = first_chunk_lines(&app);
    let boom = lines
        .iter()
        .find(|l| line_text(l).contains("error: boom"))
        .expect("stderr line 'error: boom' present");
    let more = lines
        .iter()
        .find(|l| line_text(l) == "more")
        .expect("stderr line 'more' present");
    assert!(
        line_is_red(boom),
        "'error: boom' must be red; got {:?}",
        boom.spans
    );
    assert!(
        line_is_red(more),
        "'more' must be red; got {:?}",
        more.spans
    );
    assert!(
        !lines.iter().any(|l| line_text(l).contains(STDERR_MARKER)),
        "marker text must not reach screen; got {:?}",
        lines.iter().map(line_text).collect::<Vec<_>>()
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Live non-stderr progress is folded verbatim in the normal color
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn live_non_stderr_progress_folded_verbatim_normal_color() {
    let mut app = app_with_session();
    // @step Given a streaming tool card
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk("tc-1", "Bash", "{\"command\":\"cargo run\"}"),
    ));

    // @step When a ToolProgress chunk "Listening on :3000" arrives with is_stderr false
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_progress_chunk("tc-1", "Bash", "Listening on :3000", false),
    ));

    // @step Then the card body gains the line "Listening on :3000" with no marker added
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .expect("session context exists");
    let source_text = ctx
        .scrollback
        .visible_window(4096)
        .first()
        .and_then(|c| c.source.as_ref())
        .map(|s| s.text.clone())
        .unwrap_or_default();
    assert!(
        source_text.contains("Listening on :3000"),
        "source text must carry the line; got {source_text:?}"
    );
    assert!(
        !source_text.contains(STDERR_MARKER),
        "no marker must be added for is_stderr=false; got {source_text:?}"
    );

    // @step And the line is styled in the normal body color when rendered
    let lines = first_chunk_lines(&app);
    let line = lines
        .iter()
        .find(|l| line_text(l).contains("Listening on :3000"))
        .expect("progress line present");
    assert!(
        line.spans.iter().all(|s| s.style.fg != Some(Color::Red)),
        "non-stderr line must NOT be red; got {:?}",
        line.spans
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The content modal shows stderr lines red with the marker stripped
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn content_modal_shows_stderr_red_with_marker_stripped() {
    let mut app = app_with_session();
    // @step Given a settled tool card with a body line "⚠stderr⚠error: cannot find value"
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk("tc-1", "Bash", "{\"command\":\"cargo build\"}"),
    ));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_result_chunk("tc-1", "⚠stderr⚠error: cannot find value", false),
    ));

    // @step When the TurnContentModal is opened for that card
    let seq = first_chunk_seq(&app);
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .expect("session context exists");
    let full = ctx
        .scrollback
        .full_text_for_seq(seq)
        .expect("full text exists");
    // Render each hard line through the modal styling entry point
    // (`turn_modal::styled_rows` calls `style_modal_lines` per hard line).
    let mut rows: Vec<Vec<ratatui::text::Span<'static>>> = Vec::new();
    for hard in full.split('\n') {
        rows.extend(style_modal_lines(hard, 80, false));
    }

    // @step Then the line displays as "error: cannot find value"
    let row = rows
        .iter()
        .find(|r| row_text(r).contains("error: cannot find value"))
        .expect("stderr modal row present");
    assert_eq!(
        row_text(row),
        "error: cannot find value",
        "marker must be stripped in modal; got {:?}",
        row_text(row)
    );

    // @step And the line is styled red
    assert!(
        !row.is_empty() && row.iter().all(|s| s.style.fg == Some(Color::Red)),
        "modal stderr row must be red; got {row:?}"
    );

    // @step And no "⚠stderr⚠" marker text is visible
    assert!(
        !rows.iter().any(|r| row_text(r).contains(STDERR_MARKER)),
        "marker text must not reach the modal screen; got {:?}",
        rows.iter().map(|r| row_text(r)).collect::<Vec<_>>()
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Diff cards bypass stderr detection entirely
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn diff_cards_bypass_stderr_detection() {
    let mut app = app_with_session();
    // @step Given a settled tool card whose body is an Edit/Write diff
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk(
            "tc-1",
            "Edit",
            r#"{"old_string":"line2","new_string":"CHANGED"}"#,
        ),
    ));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_result_chunk("tc-1", "ok", false),
    ));

    // @step When the tool card body is rendered in the scrollback
    let lines = first_chunk_lines(&app);

    // @step Then stderr detection is not applied to the diff rows
    // (no marker exists in a diff card, and the diff styling must be intact)
    assert!(
        !lines.iter().any(|l| line_text(l).contains(STDERR_MARKER)),
        "diff card must carry no marker; got {:?}",
        lines.iter().map(line_text).collect::<Vec<_>>()
    );

    // @step And the diff rows keep their existing removed/added/context styling
    let removed = lines
        .iter()
        .find(|l| line_text(l).contains("line2"))
        .expect("removed diff line present");
    assert!(
        removed
            .spans
            .iter()
            .any(|s| s.style.bg == Some(DIFF_BG_REMOVED)),
        "removed diff row must keep red-diff background; got {:?}",
        removed.spans
    );
    let added = lines
        .iter()
        .find(|l| line_text(l).contains("CHANGED"))
        .expect("added diff line present");
    assert!(
        added
            .spans
            .iter()
            .any(|s| s.style.bg == Some(DIFF_BG_ADDED)),
        "added diff row must keep green-diff background; got {:?}",
        added.spans
    );
}
