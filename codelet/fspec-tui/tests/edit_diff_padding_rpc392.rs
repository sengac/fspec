//! Feature: spec/features/agentview-edit-diff-padding.feature
//!
//! RPC-392 — full-width background padding for colored Edit/Write diff
//! lines. Drives the REAL store + render paths:
//!   * Scrollback: push an Edit ToolCall + matching ToolResult through the
//!     App, then assert the wrapped diff `Line`s carry spans padded to the
//!     wrap width with the correct background.
//!   * Modal: build a `TurnContentModal` over a marker-encoded body and
//!     render it, asserting diff rows are painted full-width while a plain
//!     row is not.
//!
//! Each Gherkin step carries a matching `// @step` comment whose text
//! mirrors the feature file exactly.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::store::agent_view::chunk_wrap::wrap_source;
use codelet_fspec_tui::store::agent_view::diff_decode::{
    decode_diff_line_padded, DIFF_BG_ADDED as DECODE_BG_ADDED, DIFF_BG_REMOVED as DECODE_BG_REMOVED,
};
use codelet_fspec_tui::views::agent::rendered_chunk::ChunkSource;
use codelet_fspec_tui::views::agent::turn_modal::TurnContentModal;
use codelet_fspec_tui::{Action, App, ChunkKind, FspecBackend};
use codelet_rpc_types::{SessionId, StreamChunk, ToolCallInfo, ToolResultInfo};
use ratatui::backend::TestBackend;
use ratatui::buffer::Cell;
use ratatui::style::Color;
use ratatui::text::{Line, Span};
use ratatui::Terminal;

mod common;
use common::MockBackend;

const DIFF_BG_REMOVED: Color = Color::Rgb(139, 0, 0);
const DIFF_BG_ADDED: Color = Color::Rgb(0, 100, 0);

/// Display-width of a span's content (the `chars().count()` proxy).
fn span_width(span: &Span<'static>) -> usize {
    span.content.chars().count()
}

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

fn line_text(line: &Line<'static>) -> String {
    line.spans.iter().map(|s| s.content.as_ref()).collect()
}

fn line_width(line: &Line<'static>) -> usize {
    line.spans.iter().map(|s| s.content.chars().count()).sum()
}

fn line_has_bg(line: &Line<'static>, bg: Color) -> bool {
    line.spans.iter().any(|s| s.style.bg == Some(bg))
}

/// Build a diff `ChunkSource` directly (parity with what `chunk_processor`
/// produces for an Edit) so we can wrap it at an arbitrary width.
fn diff_source(old: &str, new: &str) -> ChunkSource {
    use codelet_fspec_tui::store::agent_view::pending_tool_diff::{
        capture_pending_diff, produce_diff_strings,
    };
    let input = format!(
        r#"{{"old_string":{},"new_string":{}}}"#,
        serde_json::to_string(old).unwrap(),
        serde_json::to_string(new).unwrap()
    );
    let pending = capture_pending_diff("Edit", &input).expect("captured");
    let (collapsed, full) = produce_diff_strings(&pending);
    ChunkSource {
        text: format!("Edit(file)\n{collapsed}"),
        color: Color::White,
        kind: ChunkKind::ToolCall {
            tool_call_id: "tc-1".to_string(),
            is_error: false,
            is_diff: true,
        },
        is_streaming: false,
        full_text: Some(format!("Edit(file)\n{full}")),
    }
}

#[test]
fn scrollback_diff_branch_emits_full_width_bars() {
    // @step Given a diff tool-call whose body has a removed line and an added line
    let mut app = app_with_session();
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call(
            "tc-1",
            "Edit",
            r#"{"old_string":"line2","new_string":"CHANGED"}"#,
        ),
    ));
    app.dispatch(Action::ChunkReceived(sid("s-1"), tool_result("tc-1", "ok")));

    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .expect("ctx");
    let source = ctx
        .scrollback
        .visible_window(4096)
        .first()
        .and_then(|c| c.source.clone())
        .expect("diff source present");

    // @step When the source is wrapped at a known width
    let width: u16 = 50;
    let lines = wrap_source(&source, width);

    // @step Then the removed line carries a span whose content display-width equals the wrap width with the removed background
    let removed = lines
        .iter()
        .find(|l| line_text(l).contains("line2"))
        .expect("removed line present");
    assert!(line_has_bg(removed, DIFF_BG_REMOVED));
    assert_eq!(line_width(removed), width as usize);

    // @step And the added line carries a span whose content display-width equals the wrap width with the added background
    let added = lines
        .iter()
        .find(|l| line_text(l).contains("CHANGED"))
        .expect("added line present");
    assert!(line_has_bg(added, DIFF_BG_ADDED));
    assert_eq!(line_width(added), width as usize);
}

#[test]
fn scrollback_diff_branch_uses_real_source_helper() {
    // Sanity that diff_source mirrors the real chunk for the modal test.
    let source = diff_source("line2", "CHANGED");
    let lines = wrap_source(&source, 50);
    assert!(lines.iter().any(|l| line_has_bg(l, DIFF_BG_REMOVED)));
    assert!(lines.iter().any(|l| line_has_bg(l, DIFF_BG_ADDED)));
}

// ── decode-level scenarios (drive the same width-aware decode the
//    scrollback + modal call sites use) ───────────────────────────────

#[test]
fn removed_line_is_padded_to_a_full_width_red_bar() {
    // @step Given a decoded removed diff line shorter than the render width
    let line = "  2 [R]- line2";
    let width = 40;
    // @step When it is decoded with that render width
    let spans = decode_diff_line_padded(line, width);
    // @step Then the resulting span content display-width equals the render width
    assert_eq!(spans.len(), 1);
    assert_eq!(span_width(&spans[0]), width);
    // @step And the span background is rgb 139,0,0 and the foreground is white
    assert_eq!(spans[0].style.bg, Some(DECODE_BG_REMOVED));
    assert_eq!(spans[0].style.bg, Some(DIFF_BG_REMOVED));
    assert_eq!(spans[0].style.fg, Some(Color::White));
    // @step And the span content contains no removed marker
    assert!(!spans[0].content.contains("[R]"));
}

#[test]
fn added_line_is_padded_to_a_full_width_green_bar() {
    // @step Given a decoded added diff line shorter than the render width
    let line = "  3 [A]+ CHANGED";
    let width = 40;
    // @step When it is decoded with that render width
    let spans = decode_diff_line_padded(line, width);
    // @step Then the resulting span content display-width equals the render width
    assert_eq!(spans.len(), 1);
    assert_eq!(span_width(&spans[0]), width);
    // @step And the span background is rgb 0,100,0 and the foreground is white
    assert_eq!(spans[0].style.bg, Some(DECODE_BG_ADDED));
    assert_eq!(spans[0].style.bg, Some(DIFF_BG_ADDED));
    assert_eq!(spans[0].style.fg, Some(Color::White));
    // @step And the span content contains no added marker
    assert!(!spans[0].content.contains("[A]"));
}

#[test]
fn context_line_is_not_given_a_colored_background_bar() {
    // @step Given a decoded context line of the form 'L 250   foo'
    let line = "L 250   foo";
    let width = 40;
    // @step When it is decoded with a render width wider than the line
    let spans = decode_diff_line_padded(line, width);
    // @step Then it produces a gray-gutter span and a white-content span
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].style.fg, Some(Color::Gray));
    assert_eq!(spans[1].style.fg, Some(Color::White));
    // @step And neither span has a background colour
    assert!(spans[0].style.bg.is_none());
    assert!(spans[1].style.bg.is_none());
    // @step And the content is not padded to the render width with a background
    let total: usize = spans.iter().map(span_width).sum();
    assert!(total < width, "context must not be padded to width");
}

#[test]
fn gap_marker_or_plain_line_is_unchanged() {
    // @step Given a gap-marker line of the form '... (5 lines)'
    let line = "    ... (5 lines)";
    let width = 40;
    // @step When it is decoded with a render width
    let spans = decode_diff_line_padded(line, width);
    // @step Then it is a single span with no background and no extra padding bar
    assert_eq!(spans.len(), 1);
    assert!(spans[0].style.bg.is_none());
    assert_eq!(spans[0].content.as_ref(), line);
}

#[test]
fn content_already_at_or_over_width_is_not_padded_or_truncated() {
    // @step Given a decoded added diff line whose stripped content display-width is at least the render width
    let content_after_strip = format!("12 + {}", "x".repeat(40));
    let line = format!("12 [A]+ {}", "x".repeat(40));
    let stripped_width = content_after_strip.chars().count();
    let width = 10;
    assert!(stripped_width >= width);
    // @step When it is decoded with that render width
    let spans = decode_diff_line_padded(&line, width);
    // @step Then the content is returned unchanged with no added spaces and no truncation
    assert_eq!(spans.len(), 1);
    assert_eq!(span_width(&spans[0]), stripped_width);
    assert!(!spans[0].content.ends_with(' '));
    // @step And the span background is rgb 0,100,0
    assert_eq!(spans[0].style.bg, Some(DIFF_BG_ADDED));
}

#[test]
fn zero_width_does_not_panic_and_pads_non_negatively() {
    // @step Given a decoded removed diff line
    let line = "  2 [R]- line2";
    // @step When it is decoded with width zero
    let spans = decode_diff_line_padded(line, 0);
    // @step Then it does not panic and no padding is added
    assert_eq!(spans.len(), 1);
    assert!(!spans[0].content.ends_with(' '));
    assert!(spans[0].content.contains("line2"));
}

fn render_rows(modal: &TurnContentModal, w: u16, h: u16) -> Vec<Vec<ratatui::buffer::Cell>> {
    let backend = TestBackend::new(w, h);
    let mut term = Terminal::new(backend).expect("Terminal::new");
    term.draw(|frame| modal.render(frame.area(), frame.buffer_mut()))
        .expect("draw");
    let buf = term.backend().buffer().clone();
    (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].clone())
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn modal_diff_rows_emit_full_width_bars_while_non_diff_rows_do_not() {
    // @step Given a modal body containing a removed-marker row, an added-marker row, and a plain row
    let source = diff_source("line2", "CHANGED");
    let full = source.full_text.clone().expect("full text");
    let modal = TurnContentModal::new(full, Some(source.kind));

    // @step When the modal decodes rows at the content width
    let rows = render_rows(&modal, 60, 24);

    // @step Then the diff rows are padded full-width with the diff background
    let removed_row = rows
        .iter()
        .find(|row| {
            row.iter().any(|c| c.bg == DIFF_BG_REMOVED)
                && row
                    .iter()
                    .map(Cell::symbol)
                    .collect::<String>()
                    .contains("line2")
        })
        .expect("removed bar row present");
    let added_row = rows
        .iter()
        .find(|row| {
            row.iter().any(|c| c.bg == DIFF_BG_ADDED)
                && row
                    .iter()
                    .map(Cell::symbol)
                    .collect::<String>()
                    .contains("CHANGED")
        })
        .expect("added bar row present");

    // The colored background must extend to (near) the modal content width
    // — i.e. there is substantial trailing padding past the last text glyph,
    // not merely a background under the bare content.
    let red_cells = removed_row
        .iter()
        .filter(|c| c.bg == DIFF_BG_REMOVED)
        .count();
    let green_cells = added_row.iter().filter(|c| c.bg == DIFF_BG_ADDED).count();
    // At 60 cols the modal inner content width is ~52; a padded bar fills it.
    assert!(
        red_cells >= 40,
        "red bar must be padded to ~content width; got {red_cells} cells"
    );
    assert!(
        green_cells >= 40,
        "green bar must be padded to ~content width; got {green_cells} cells"
    );

    // @step And the plain row is a single unpadded raw span
    // The title bar / footer rows must NOT carry a diff background; only the
    // two diff rows do.
    let diff_bg_rows = rows
        .iter()
        .filter(|row| {
            row.iter()
                .any(|c| c.bg == DIFF_BG_REMOVED || c.bg == DIFF_BG_ADDED)
        })
        .count();
    assert_eq!(
        diff_bg_rows, 2,
        "exactly the two diff rows carry a diff background"
    );
}
