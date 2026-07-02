//! RPC-401 — AgentView message line-spacing parity: every rendered chunk
//! (message) must emit exactly ONE trailing blank separator line after its
//! content, matching the TS `wrapMessageToLines` `addSeparator=true` default
//! (`src/tui/utils/conversationUtils.ts:117-127`).
//!
//! Feature: spec/features/agentview-message-line-spacing-parity-missing-per-message-separator-gutter.feature
//!
//! These tests drive the REAL store/render path (mirroring the RPC-399/400
//! harness): push chunks via the App dispatch, then inspect the wrapped
//! `RenderedChunk::lines`. They encode the NEW contract: a trailing empty
//! `Line::from("")` after EVERY chunk's content. They MUST FAIL (red phase)
//! against current `wrap_source`, which appends NO separator.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, StreamChunk, ToolCallInfo, ToolResultInfo};
use ratatui::backend::TestBackend;
use ratatui::text::Line;
use ratatui::Terminal;

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

/// Concatenated text of a `Line`.
fn line_text(line: &Line<'static>) -> String {
    line.spans.iter().map(|s| s.content.as_ref()).collect()
}

/// Wrapped lines of the chunk at `idx` (0-based) in s-1.
fn chunk_lines(app: &App, idx: usize) -> Vec<Line<'static>> {
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .expect("session context exists");
    ctx.scrollback
        .visible_window(4096)
        .get(idx)
        .map(|c| c.lines.clone())
        .unwrap_or_default()
}

/// All wrapped lines across every chunk in s-1, in order (the combined
/// scrollback the painter walks).
fn all_scrollback_lines(app: &App) -> Vec<Line<'static>> {
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .expect("session context exists");
    ctx.scrollback
        .visible_window(4096)
        .iter()
        .flat_map(|c| c.lines.clone())
        .collect()
}

/// Build a body of `n` numbered lines: "line-1".."line-n".
fn numbered_body(n: usize) -> String {
    (1..=n)
        .map(|i| format!("line-{i}"))
        .collect::<Vec<_>>()
        .join("\n")
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A single user message ends with one blank separator line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn single_user_message_ends_with_one_blank_separator_line() {
    let mut app = app_with_session();
    // @step Given a user message with text "hello"
    // @step When the message is wrapped for the scrollback
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::user_input("hello".to_string()),
    ));

    let lines = chunk_lines(&app, 0);

    // @step Then the wrapped output has 2 lines
    assert_eq!(
        lines.len(),
        2,
        "user message must wrap to content + one trailing blank; got {:?}",
        lines.iter().map(line_text).collect::<Vec<_>>()
    );

    // @step And the first line is "You: hello"
    assert_eq!(
        line_text(&lines[0]),
        "You: hello",
        "first line must be the prefixed content"
    );

    // @step And the last line is blank
    assert_eq!(
        line_text(&lines[lines.len() - 1]),
        "",
        "last line must be a blank separator"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Two consecutive assistant messages are separated by one blank line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn two_consecutive_assistant_messages_separated_by_one_blank_line() {
    let mut app = app_with_session();
    // @step Given an assistant message with text "first"
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::text("first".to_string()),
    ));
    app.dispatch(Action::ChunkReceived(sid("s-1"), StreamChunk::Done));

    // @step And an assistant message with text "second"
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::text("second".to_string()),
    ));
    app.dispatch(Action::ChunkReceived(sid("s-1"), StreamChunk::Done));

    // @step When both messages are wrapped and painted into the scrollback
    let lines = all_scrollback_lines(&app);
    let texts: Vec<String> = lines.iter().map(line_text).collect();

    // @step Then exactly one blank line appears between the two content blocks
    let first_idx = texts
        .iter()
        .position(|t| t.contains("first"))
        .expect("'first' content line present");
    let second_idx = texts
        .iter()
        .position(|t| t.contains("second"))
        .expect("'second' content line present");
    assert!(
        second_idx > first_idx,
        "second content must come after first; got {texts:?}"
    );
    let between: Vec<&String> = texts[first_idx + 1..second_idx].iter().collect();
    assert_eq!(
        between.len(),
        1,
        "exactly one row must sit between the two content blocks; got {between:?}"
    );
    assert_eq!(
        between[0], "",
        "the row between the two messages must be blank; got {between:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A collapsed tool-call chunk ends with a blank separator line
//           after its indicator
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn collapsed_tool_call_chunk_ends_with_blank_separator_after_indicator() {
    let mut app = app_with_session();
    // @step Given a settled tool-call chunk whose body exceeds the collapse threshold
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call_chunk("tc-1", "Bash", "{\"command\":\"echo\"}"),
    ));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_result_chunk("tc-1", &numbered_body(20), false),
    ));

    // @step When the tool-call chunk is wrapped for the scrollback
    let lines = chunk_lines(&app, 0);
    let texts: Vec<String> = lines.iter().map(line_text).collect();
    assert!(
        lines.len() >= 2,
        "need at least indicator + blank; got {texts:?}"
    );

    // @step Then the second to last line is the "... +N lines" indicator
    let second_to_last = &texts[texts.len() - 2];
    assert!(
        second_to_last.starts_with("... +") && second_to_last.contains("lines"),
        "second-to-last line must be the '... +N lines' indicator; got {second_to_last:?} in {texts:?}"
    );

    // @step And the last line is blank
    assert_eq!(
        texts[texts.len() - 1],
        "",
        "last line must be a blank separator; got {texts:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Total visual rows include one separator per message
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn total_visual_rows_include_one_separator_per_message() {
    let mut app = app_with_session();
    // @step Given three single-line messages in the scrollback
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::user_input("one".to_string()),
    ));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::user_input("two".to_string()),
    ));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::user_input("three".to_string()),
    ));

    // @step When the total visual rows are computed
    // `total_visual_rows` is pub(crate); from an integration test we sum the
    // wrapped line counts across every chunk (the same quantity, driven off
    // `RenderedChunk::lines`).
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .expect("session context exists");
    let total: usize = ctx
        .scrollback
        .visible_window(4096)
        .iter()
        .map(|c| c.lines.len())
        .sum();

    // @step Then the total visual rows equal 6
    assert_eq!(
        total, 6,
        "3 content rows + 3 separator rows must total 6; got {total}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A diff tool-call chunk keeps its diff rows and ends with a
//           blank separator line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn diff_tool_call_chunk_keeps_diff_rows_and_ends_with_blank_separator() {
    let mut app = app_with_session();
    // @step Given a diff tool-call chunk with typed diff rows
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

    // @step When the diff tool-call chunk is wrapped for the scrollback
    let lines = chunk_lines(&app, 0);
    let texts: Vec<String> = lines.iter().map(line_text).collect();

    // @step Then the typed diff rows are preserved
    assert!(
        texts.iter().any(|t| t.contains("line2")),
        "removed diff row 'line2' must be preserved; got {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t.contains("CHANGED")),
        "added diff row 'CHANGED' must be preserved; got {texts:?}"
    );

    // @step And the last line is blank
    assert_eq!(
        texts[texts.len() - 1],
        "",
        "last line must be a blank separator; got {texts:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: In Item mode arrow bars occupy the gutter rows without hiding
//           content
// ─────────────────────────────────────────────────────────────────────────

/// Render the App and return the rows of glyphs.
fn render_rows(app: &mut App, w: u16, h: u16) -> Vec<String> {
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("Terminal::new");
    term.draw(|frame| {
        app.render(frame.area(), frame.buffer_mut());
    })
    .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut rows = Vec::with_capacity(buf.area.height as usize);
    for y in 0..buf.area.height {
        let mut row = String::new();
        for x in 0..buf.area.width {
            row.push_str(buf[(x, y)].symbol());
        }
        rows.push(row);
    }
    rows
}

#[test]
fn item_mode_arrow_bars_occupy_gutter_rows_without_hiding_content() {
    use codelet_fspec_tui::ViewMode;

    let mut app = app_with_session();
    app.navigator_mut().active_view = ViewMode::Agent;

    // @step Given three single-line messages in the scrollback in Item mode
    // Seed three single-line source-backed messages via user input so each
    // chunk carries a trailing separator once RPC-401 lands.
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::user_input("turn-0".to_string()),
    ));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::user_input("turn-1".to_string()),
    ));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::user_input("turn-2".to_string()),
    ));

    // @step And the middle turn is selected
    {
        let ctx = app
            .agent_view_store_mut()
            .session_context_mut_for(&sid("s-1"))
            .expect("session context exists");
        ctx.scrollback.enter_item_mode(); // selects the last turn (idx 2)
        ctx.scrollback
            .navigate_turn(codelet_fspec_tui::views::agent::TurnDir::Up); // 2 -> 1 (middle)
        assert_eq!(ctx.scrollback.selected_index(), Some(1));
    }

    // @step When the scrollback is painted
    let rows = render_rows(&mut app, 120, 24);

    // The selected turn's content row must still be visible.
    let sel_y = rows
        .iter()
        .position(|r| r.contains("turn-1"))
        .expect("selected turn content row present");
    assert!(sel_y >= 1, "need a row above the selected turn");

    // With one blank separator gutter between every message, the ▼ bar row
    // above the selected turn must NOT be an adjacent turn's content row and
    // the ▲ bar row below must NOT be an adjacent turn's content row: the
    // bars occupy the blank gutters, so ALL THREE turns stay visible.
    //
    // @step Then the down arrow bar is painted on the blank separator row above the selected turn
    assert!(
        rows[sel_y - 1].contains('\u{25BC}'),
        "expected ▼ bar on the blank separator row above the selected turn; got {:?}",
        rows[sel_y - 1]
    );
    // The ▼ bar row must be a gutter, not the previous turn's content.
    assert!(
        !rows[sel_y - 1].contains("turn-0"),
        "the ▼ bar row must be a blank gutter, not turn-0's content; got {:?}",
        rows[sel_y - 1]
    );

    // @step And the up arrow bar is painted on the blank separator row below the selected turn
    // The selected chunk now owns a trailing blank separator row (its own
    // gutter). The ▲ bar is painted ON that gutter — the row directly below
    // the selected content (`sel_y + 1`) — so it occupies the blank
    // separator instead of overwriting the next turn's content.
    assert!(
        rows[sel_y + 1].contains('\u{25B2}'),
        "expected ▲ bar on the selected turn's blank separator row; got {:?}",
        rows[sel_y + 1]
    );
    // The ▲ bar row must be a gutter, not the next turn's content.
    assert!(
        !rows[sel_y + 1].contains("turn-2"),
        "the ▲ bar row must be a blank gutter, not turn-2's content; got {:?}",
        rows[sel_y + 1]
    );

    // @step And the selected turn's content row is still visible
    assert!(
        rows[sel_y].contains("turn-1"),
        "the selected turn's content row must remain visible; got {:?}",
        rows[sel_y]
    );
    // Both neighbouring turns must ALSO remain visible (the bars did not
    // overwrite them because they landed on gutter rows).
    assert!(
        rows.iter().any(|r| r.contains("turn-0")),
        "turn-0 must remain visible (bar landed on a gutter); got rows {rows:?}"
    );
    assert!(
        rows.iter().any(|r| r.contains("turn-2")),
        "turn-2 must remain visible (bar landed on a gutter); got rows {rows:?}"
    );
}
