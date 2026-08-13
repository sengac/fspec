//! Feature: spec/features/agentview-edit-diff-rendering.feature
//!
//! RPC-391 — colored Edit/Write diff rendering in the live agent view.
//! Drives the REAL store path: push a ToolCall chunk (Edit/Write) then the
//! matching ToolResult via the App, and inspect the wrapped `Line`/`Span`
//! styles of the tool-call card. Each Gherkin step carries a matching
//! `// @step` comment whose text mirrors the feature file exactly.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{Action, App, ChunkKind, FspecBackend};
use codelet_rpc_types::{SessionId, StreamChunk, ToolCallInfo, ToolResultInfo};
use ratatui::style::Color;
use ratatui::text::Line;

mod common;
use common::MockBackend;

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

fn tool_call(id: &str, name: &str, input: &str) -> StreamChunk {
    StreamChunk::tool_call(ToolCallInfo {
        id: id.to_string(),
        name: name.to_string(),
        input: input.to_string(),
    })
}

fn tool_result(tool_call_id: &str, content: &str) -> StreamChunk {
    StreamChunk::tool_result(ToolResultInfo {
        tool_call_id: tool_call_id.to_string(),
        content: content.to_string(),
        is_error: false,
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

/// Concatenated text of a `Line`.
fn line_text(line: &Line<'static>) -> String {
    line.spans.iter().map(|s| s.content.as_ref()).collect()
}

/// True if any span on `line` carries background `bg`.
fn line_has_bg(line: &Line<'static>, bg: Color) -> bool {
    line.spans.iter().any(|s| s.style.bg == Some(bg))
}

#[test]
fn edit_replacing_one_line_shows_old_on_red_and_new_on_green() {
    let mut app = app_with_session();
    // @step Given an Edit tool call whose old_string and new_string differ in one line is captured at tool-call time
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call(
            "tc-1",
            "Edit",
            r#"{"old_string":"line2","new_string":"CHANGED"}"#,
        ),
    ));

    // @step When the matching ToolResult arrives and the diff card is wrapped into lines
    app.dispatch(Action::ChunkReceived(sid("s-1"), tool_result("tc-1", "ok")));
    let lines = first_chunk_lines(&app);

    // @step Then the removed line span has a background of rgb 139,0,0 and white text
    let removed = lines
        .iter()
        .find(|l| line_text(l).contains("line2"))
        .expect("removed line present");
    assert!(line_has_bg(removed, DIFF_BG_REMOVED));
    assert!(removed
        .spans
        .iter()
        .any(|s| s.style.bg == Some(DIFF_BG_REMOVED) && s.style.fg == Some(Color::White)));

    // @step And the added line span has a background of rgb 0,100,0 and white text
    let added = lines
        .iter()
        .find(|l| line_text(l).contains("CHANGED"))
        .expect("added line present");
    assert!(line_has_bg(added, DIFF_BG_ADDED));
    assert!(added
        .spans
        .iter()
        .any(|s| s.style.bg == Some(DIFF_BG_ADDED) && s.style.fg == Some(Color::White)));
}

#[test]
fn write_of_a_new_three_line_file_shows_three_green_lines() {
    let mut app = app_with_session();
    // @step Given a Write tool call whose content has three lines is captured at tool-call time
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call("tc-1", "Write", r#"{"content":"alpha\nbeta\ngamma"}"#),
    ));

    // @step When the matching ToolResult arrives and the diff card is wrapped into lines
    app.dispatch(Action::ChunkReceived(sid("s-1"), tool_result("tc-1", "ok")));
    let lines = first_chunk_lines(&app);

    // @step Then three line spans each have a background of rgb 0,100,0 and white text
    let green = lines
        .iter()
        .filter(|l| line_has_bg(l, DIFF_BG_ADDED))
        .count();
    assert_eq!(green, 3, "expected 3 green lines; got {green}");

    // @step And no removed-line background appears
    assert!(!lines.iter().any(|l| line_has_bg(l, DIFF_BG_REMOVED)));
}

#[test]
fn bash_tool_result_renders_plain_white_with_no_diff_coloring() {
    let mut app = app_with_session();
    // @step Given a Bash tool call and its ToolResult with no captured pending diff
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call("tc-1", "Bash", r#"{"command":"echo hi"}"#),
    ));
    let body: String = (1..=20)
        .map(|i| format!("line-{i}"))
        .collect::<Vec<_>>()
        .join("\n");

    // @step When the tool card is wrapped into lines
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_result("tc-1", &body),
    ));
    let lines = first_chunk_lines(&app);

    // @step Then no span carries a red or green diff background
    assert!(!lines.iter().any(|l| line_has_bg(l, DIFF_BG_REMOVED)));
    assert!(!lines.iter().any(|l| line_has_bg(l, DIFF_BG_ADDED)));

    // @step And the card is collapsed by the existing eight-line tool-output rule
    assert!(lines
        .iter()
        .any(|l| line_text(l) == "... +12 lines (Enter to view full)"));
}

#[test]
fn edit_over_collapse_limit_shows_25_inline_while_modal_shows_full() {
    let mut app = app_with_session();
    // @step Given an Edit producing more than 25 diff display lines is captured at tool-call time
    let old: String = (1..=60).map(|i| format!("old{i}\n")).collect();
    let new: String = (1..=60).map(|i| format!("new{i}\n")).collect();
    let input = format!(
        r#"{{"old_string":{},"new_string":{}}}"#,
        serde_json::to_string(&old).unwrap(),
        serde_json::to_string(&new).unwrap()
    );
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call("tc-1", "Edit", &input),
    ));

    // @step When the matching ToolResult arrives and the diff card is wrapped into lines
    app.dispatch(Action::ChunkReceived(sid("s-1"), tool_result("tc-1", "ok")));
    let lines = first_chunk_lines(&app);

    // @step Then the inline body shows 25 display lines plus a '... +N lines' indicator
    let texts: Vec<String> = lines.iter().map(line_text).collect();
    assert!(
        texts.iter().any(|t| t.contains("(select turn to /expand)")),
        "inline collapse indicator missing; got {texts:?}"
    );

    // @step And the retained full diff exposed to the turn-content modal contains all display lines
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .expect("ctx");
    let seq = ctx.scrollback.visible_window(4096).first().unwrap().seq;
    let full = ctx.scrollback.full_text_for_seq(seq).expect("full text");
    assert!(
        full.contains("old1") && full.contains("new60"),
        "modal full diff must contain all lines"
    );
    assert!(
        !full.contains("(select turn to /expand)"),
        "modal full diff must NOT be collapsed"
    );
}

#[test]
fn edit_with_uncaptured_pending_input_falls_back_to_raw_text() {
    let mut app = app_with_session();
    // @step Given an Edit tool call whose input is malformed JSON so no pending diff is captured
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call("tc-1", "Edit", "not-json"),
    ));

    // @step When the matching ToolResult arrives and the tool card is wrapped into lines
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_result("tc-1", "RAW_RESULT_BODY"),
    ));
    let lines = first_chunk_lines(&app);

    // @step Then the raw ToolResult content is shown as plain text with no diff coloring
    assert!(lines
        .iter()
        .any(|l| line_text(l).contains("RAW_RESULT_BODY")));
    assert!(!lines.iter().any(|l| line_has_bg(l, DIFF_BG_REMOVED)));
    assert!(!lines.iter().any(|l| line_has_bg(l, DIFF_BG_ADDED)));

    // @step And no panic occurs
    // (reaching this line proves no panic occurred)
}

#[test]
fn context_diff_lines_render_with_gray_gutter_and_white_content() {
    let mut app = app_with_session();
    // @step Given an Edit diff card containing a context line of the form '  250   foo'
    let old = "ctx1\nctx2\nold\nctx3\nctx4";
    let new = "ctx1\nctx2\nnew\nctx3\nctx4";
    let input = format!(
        r#"{{"old_string":{},"new_string":{}}}"#,
        serde_json::to_string(old).unwrap(),
        serde_json::to_string(new).unwrap()
    );
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call("tc-1", "Edit", &input),
    ));

    // @step When the diff card is wrapped into lines
    app.dispatch(Action::ChunkReceived(sid("s-1"), tool_result("tc-1", "ok")));
    let lines = first_chunk_lines(&app);

    // @step Then the line-number gutter span is gray and the content span is white
    let ctx_line = lines
        .iter()
        .find(|l| line_text(l).contains("ctx1"))
        .expect("context line present");
    assert!(
        ctx_line
            .spans
            .iter()
            .any(|s| s.style.fg == Some(Color::Gray)),
        "gutter must be gray; got {:?}",
        ctx_line.spans
    );
    assert!(ctx_line
        .spans
        .iter()
        .any(|s| s.content.contains("ctx1") && s.style.bg.is_none()));
}

#[test]
fn marker_characters_are_stripped_before_display() {
    let mut app = app_with_session();
    // @step Given an Edit diff card with removed and added lines
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call("tc-1", "Edit", r#"{"old_string":"foo","new_string":"bar"}"#),
    ));

    // @step When the diff card is wrapped into lines
    app.dispatch(Action::ChunkReceived(sid("s-1"), tool_result("tc-1", "ok")));
    let lines = first_chunk_lines(&app);

    // @step Then no rendered span text contains the literal '[R]' or '[A]' marker
    for l in &lines {
        let t = line_text(l);
        assert!(!t.contains("[R]"), "literal [R] leaked: {t:?}");
        assert!(!t.contains("[A]"), "literal [A] leaked: {t:?}");
    }
}

#[test]
fn diff_cards_bypass_the_eight_line_tool_output_collapse() {
    let mut app = app_with_session();
    // @step Given an Edit diff card whose collapsed body has more than eight lines
    let old: String = (1..=12).map(|i| format!("old{i}\n")).collect();
    let new: String = (1..=12).map(|i| format!("new{i}\n")).collect();
    let input = format!(
        r#"{{"old_string":{},"new_string":{}}}"#,
        serde_json::to_string(&old).unwrap(),
        serde_json::to_string(&new).unwrap()
    );
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call("tc-1", "Edit", &input),
    ));

    // @step When the diff card is wrapped into lines
    app.dispatch(Action::ChunkReceived(sid("s-1"), tool_result("tc-1", "ok")));
    let lines = first_chunk_lines(&app);
    let texts: Vec<String> = lines.iter().map(line_text).collect();

    // @step Then no '... +N lines (Enter to view full)' indicator from the eight-line collapse appears
    assert!(
        !texts.iter().any(|t| t.contains("(Enter to view full)")),
        "8-line collapse indicator must NOT appear for diff cards; got {texts:?}"
    );

    // @step And all of the diff body lines are rendered
    assert!(texts.iter().any(|t| t.contains("old1")));
    assert!(texts.iter().any(|t| t.contains("new12")));

    // Sanity: the card is a diff ToolCall.
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .expect("ctx");
    let kind = ctx
        .scrollback
        .visible_window(4096)
        .first()
        .and_then(|c| c.source.as_ref())
        .map(|s| s.kind.clone())
        .unwrap();
    assert!(matches!(kind, ChunkKind::ToolCall { is_diff: true, .. }));
}
