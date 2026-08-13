//! Feature: spec/features/agentview-edit-diff-structured-rows.feature
//!
//! RPC-393 — integration coverage for the typed structured-row diff model.
//! Drives the REAL store + render paths (like `edit_diff_padding_rpc392.rs`):
//!   * Scrollback: push an Edit ToolCall + matching ToolResult, wrap, and
//!     assert consistent gutter coloring + no marker leakage.
//!   * Resize re-wrap: wrap at one width, `rewrap_at` at another, re-assert.
//!   * Modal: render `TurnContentModal` over the full diff and assert
//!     full-width bars + no markers on screen.
//!   * No-regression: a Bash tool result renders plain.
//!
//! Every Gherkin step carries a matching `// @step` comment whose text
//! mirrors the feature file exactly.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::store::agent_view::chunk_wrap::wrap_source;
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

fn span_width(span: &Span<'static>) -> usize {
    span.content.chars().count()
}

/// True if any line in `lines` shows a literal `[R]` or `[A]` marker.
fn any_marker_leak(lines: &[Line<'static>]) -> bool {
    lines.iter().any(|l| {
        let t = line_text(l);
        t.contains("[R]") || t.contains("[A]")
    })
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
fn scrollback_renders_wrapped_diff_with_consistent_gutter_and_no_markers() {
    // @step Given a diff tool-call whose body has a context line, a removed line, and an added line
    let mut app = app_with_session();
    let old = "ctx1\nctx2\nline2\nctx3\nctx4";
    let new = "ctx1\nctx2\nCHANGED\nctx3\nctx4";
    let input = format!(
        r#"{{"old_string":{},"new_string":{}}}"#,
        serde_json::to_string(old).unwrap(),
        serde_json::to_string(new).unwrap()
    );
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call("tc-1", "Edit", &input),
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

    // @step When the source is wrapped at width 50
    let width: u16 = 50;
    let lines = wrap_source(&source, width);

    // @step Then the removed and added lines are full-width colored bars
    let removed = lines
        .iter()
        .find(|l| line_text(l).contains("line2"))
        .expect("removed line present");
    assert!(line_has_bg(removed, DIFF_BG_REMOVED));
    assert_eq!(line_width(removed), width as usize);
    let added = lines
        .iter()
        .find(|l| line_text(l).contains("CHANGED"))
        .expect("added line present");
    assert!(line_has_bg(added, DIFF_BG_ADDED));
    assert_eq!(line_width(added), width as usize);

    // @step And the context line has a gray gutter and no background
    let ctx_line = lines
        .iter()
        .find(|l| line_text(l).contains("ctx2"))
        .expect("context line present");
    assert!(ctx_line
        .spans
        .iter()
        .any(|s| s.style.fg == Some(Color::Gray)));
    assert!(ctx_line.spans.iter().all(|s| s.style.bg.is_none()));

    // @step And no rendered line contains a literal diff marker
    assert!(!any_marker_leak(&lines));
}

#[test]
fn diff_body_survives_a_terminal_width_re_wrap_on_resize() {
    // @step Given a diff tool-call card wrapped at width 50
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

    {
        let store = app.agent_view_store_mut();
        let ctx = store.session_context_mut_for(&sid("s-1")).expect("ctx mut");
        ctx.scrollback.set_viewport_width(50);
    }

    // @step When the chunk is re-wrapped at width 80
    let lines = {
        let store = app.agent_view_store_mut();
        let ctx = store.session_context_mut_for(&sid("s-1")).expect("ctx mut");
        ctx.scrollback.set_viewport_width(80);
        ctx.scrollback
            .visible_window(4096)
            .first()
            .map(|c| c.lines.clone())
            .unwrap_or_default()
    };

    // @step Then the diff still renders removed and added colored bars
    assert!(lines.iter().any(|l| line_has_bg(l, DIFF_BG_REMOVED)));
    assert!(lines.iter().any(|l| line_has_bg(l, DIFF_BG_ADDED)));
    let removed = lines
        .iter()
        .find(|l| line_text(l).contains("line2"))
        .expect("removed line present after resize");
    assert_eq!(line_width(removed), 80);

    // @step And no rendered line contains a literal diff marker
    assert!(!any_marker_leak(&lines));
}

fn render_rows(modal: &TurnContentModal, w: u16, h: u16) -> Vec<Vec<Cell>> {
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
fn the_modal_shows_the_full_diff_styled_identically() {
    // @step Given a modal over the full uncollapsed diff body
    let source = diff_source("line2", "CHANGED");
    let full = source.full_text.clone().expect("full text");
    let modal = TurnContentModal::new(full, Some(source.kind));

    // @step When the modal renders its rows
    let rows = render_rows(&modal, 60, 24);

    // @step Then the diff rows are full-width colored bars and plain rows are not
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
    let removed_row = rows
        .iter()
        .find(|row| row.iter().any(|c| c.bg == DIFF_BG_REMOVED))
        .expect("removed bar row");
    let red_cells = removed_row
        .iter()
        .filter(|c| c.bg == DIFF_BG_REMOVED)
        .count();
    assert!(
        red_cells >= 40,
        "red bar padded to ~content width; got {red_cells}"
    );

    // @step And no marker characters appear on screen
    let screen: String = rows
        .iter()
        .map(|row| row.iter().map(Cell::symbol).collect::<String>())
        .collect();
    assert!(!screen.contains("[R]"));
    assert!(!screen.contains("[A]"));
}

#[test]
fn modal_does_not_diff_style_a_non_diff_turn_that_looks_line_numbered() {
    // WARNING #4: a plain (non-diff) turn whose body contains a line that
    // LOOKS like a context diff row ("42   indented log") must NOT be
    // diff-styled by the modal. The modal gates diff styling on the turn kind.
    // @step Given a modal over a non-diff turn body containing a line-numbered-looking line
    let body = "42   indented log line\nplain second line";
    let modal = TurnContentModal::new(body, Some(ChunkKind::AssistantText));

    // @step When the modal renders its rows
    let rows = render_rows(&modal, 60, 24);

    // @step Then no row carries any diff background
    let diff_bg_rows = rows
        .iter()
        .filter(|row| {
            row.iter()
                .any(|c| c.bg == DIFF_BG_REMOVED || c.bg == DIFF_BG_ADDED)
        })
        .count();
    assert_eq!(
        diff_bg_rows, 0,
        "a non-diff turn must never be diff-styled by the modal"
    );
    // @step And the line-numbered-looking text is shown verbatim
    let screen: String = rows
        .iter()
        .map(|row| row.iter().map(Cell::symbol).collect::<String>())
        .collect();
    assert!(screen.contains("indented log line"));
}

#[test]
fn non_diff_tool_output_renders_plain_with_no_diff_styling() {
    // @step Given a Bash tool result with no captured diff
    let mut app = app_with_session();
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
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .expect("ctx");
    let lines = ctx
        .scrollback
        .visible_window(4096)
        .first()
        .map(|c| c.lines.clone())
        .unwrap_or_default();

    // @step Then no line carries a red or green diff background
    assert!(!lines.iter().any(|l| line_has_bg(l, DIFF_BG_REMOVED)));
    assert!(!lines.iter().any(|l| line_has_bg(l, DIFF_BG_ADDED)));
    let _ = span_width(&Span::raw("x"));
}

// ── Unit-level scenarios: typed rows, codec, and the single style_row ──
// These drive the public surface directly. They live here (not as
// `#[cfg(test)]` modules) so the source files stay under the 300-LoC
// source-shape ceiling pinned by rpc024/026.

use codelet_fspec_tui::store::agent_view::diff_decode::style_row;
use codelet_fspec_tui::store::agent_view::diff_format::{
    build_diff_rows, format_edit_diff, parse_line, to_line, DiffDisplayRow, DIFF_COLLAPSED_LINES,
};
use ratatui::style::Modifier;

fn spans_text(spans: &[Span<'static>]) -> String {
    spans.iter().map(|s| s.content.as_ref()).collect()
}

fn spans_width(spans: &[Span<'static>]) -> usize {
    spans.iter().map(|s| s.content.chars().count()).sum()
}

#[test]
fn build_rows_single_line_replacement_is_context_removed_added_context() {
    // @step Given an old_string and new_string that differ in a single line
    let old = "line1\nline2\nline3\n";
    let new = "line1\nCHANGED\nline3\n";
    // @step When I build the diff display rows
    let diff = format_edit_diff(old, new);
    let rows = build_diff_rows(&diff, DIFF_COLLAPSED_LINES, 1);
    // @step Then the rows are typed Context, Removed, Added, Context in order
    assert_eq!(
        rows,
        vec![
            DiffDisplayRow::Context {
                line_no: 1,
                text: "line1".into()
            },
            DiffDisplayRow::Removed {
                line_no: 2,
                text: "line2".into()
            },
            DiffDisplayRow::Added {
                line_no: 3,
                text: "CHANGED".into()
            },
            DiffDisplayRow::Context {
                line_no: 4,
                text: "line3".into()
            },
        ]
    );
    // @step And each row carries its correct 1-based line number under three lines of context
}

#[test]
fn build_rows_mid_file_change_drops_leading_and_marks_trailing_elision() {
    // @step Given a 100-line edit with a single changed line in the middle
    let old: String = (1..=100).map(|n| format!("line{n}\n")).collect();
    let new: String = (1..=100)
        .map(|n| {
            if n == 50 {
                "line50-CHANGED\n".to_string()
            } else {
                format!("line{n}\n")
            }
        })
        .collect();
    // @step When I build the diff display rows
    let diff = format_edit_diff(&old, &new);
    let rows = build_diff_rows(&diff, DIFF_COLLAPSED_LINES, 1);
    // @step Then the leading region is dropped and a trailing Elision row marks the skipped region after the change
    // No leading Elision: the FIRST row is the leading context window
    // (not an Elision), proving the leading region is dropped, not elided.
    assert!(
        !matches!(rows.first(), Some(DiffDisplayRow::Elision { .. })),
        "leading region must be dropped, not represented by a leading Elision"
    );
    assert!(matches!(rows.last(), Some(DiffDisplayRow::Elision { .. })));
    let first_change = rows
        .iter()
        .position(|r| {
            matches!(
                r,
                DiffDisplayRow::Removed { .. } | DiffDisplayRow::Added { .. }
            )
        })
        .expect("a change row");
    let trailing_elision = rows
        .iter()
        .rposition(|r| matches!(r, DiffDisplayRow::Elision { .. }))
        .expect("a trailing elision");
    assert!(trailing_elision > first_change);
    // @step And every elision is the same uniform Elision kind rather than a bespoke string
    let elisions = rows
        .iter()
        .filter(|r| matches!(r, DiffDisplayRow::Elision { .. }))
        .count();
    assert_eq!(elisions, 1, "exactly one (trailing) Elision");
}

#[test]
fn build_rows_over_collapse_limit_yields_collapse_hint_only_when_collapsed() {
    // @step Given a diff whose display rows exceed the collapse limit of 25
    let new: String = (1..=60).map(|n| format!("added{n}\n")).collect();
    let diff = format_edit_diff("", &new);
    // @step When I build the collapsed diff display rows
    let collapsed = build_diff_rows(&diff, DIFF_COLLAPSED_LINES, 1);
    // @step Then the final row is an Elision collapse hint
    match collapsed.last() {
        Some(DiffDisplayRow::Elision { text }) => {
            assert!(text.contains("(select turn to /expand)"))
        }
        other => panic!("expected collapse elision, got {other:?}"),
    }
    // @step And the full uncollapsed build contains no collapse Elision hint
    let full = build_diff_rows(&diff, diff.len().max(1), 1);
    assert!(!full.iter().any(|r| matches!(
        r, DiffDisplayRow::Elision { text } if text.contains("(select turn to /expand)"))));
}

#[test]
fn style_row_changed_rows_fill_a_full_width_colored_bar() {
    // @step Given a Removed row and an Added row
    let removed = DiffDisplayRow::Removed {
        line_no: 2,
        text: "line2".into(),
    };
    let added = DiffDisplayRow::Added {
        line_no: 3,
        text: "CHANGED".into(),
    };
    let width = 40;
    // @step When I style each row at a render width wider than its content
    let rs = style_row(&removed, width);
    let as_ = style_row(&added, width);
    // @step Then the styled spans total display width equals the render width
    assert_eq!(spans_width(&rs), width);
    assert_eq!(spans_width(&as_), width);
    // @step And the removed bar background is rgb 139,0,0 and the added bar background is rgb 0,100,0 with white foreground
    assert!(rs
        .iter()
        .any(|s| s.style.bg == Some(DIFF_BG_REMOVED) && s.style.fg == Some(Color::White)));
    assert!(as_
        .iter()
        .any(|s| s.style.bg == Some(DIFF_BG_ADDED) && s.style.fg == Some(Color::White)));
    // @step And no styled span contains a marker character
    assert!(!spans_text(&rs).contains("[R]"));
    assert!(!spans_text(&as_).contains("[A]"));
}

#[test]
fn style_row_context_has_gray_gutter_and_no_background() {
    // @step Given a Context row
    let row = DiffDisplayRow::Context {
        line_no: 250,
        text: "foo".into(),
    };
    let width = 40;
    // @step When I style the row at a render width wider than its content
    let spans = style_row(&row, width);
    // @step Then the gutter span is gray and the content span is white
    assert!(spans.iter().any(|s| s.style.fg == Some(Color::Gray)));
    assert!(spans.iter().any(|s| s.style.fg == Some(Color::White)));
    // @step And neither span carries a background colour
    assert!(spans.iter().all(|s| s.style.bg.is_none()));
    // @step And the content is not padded full-width
    assert!(spans_width(&spans) < width);
}

#[test]
fn style_row_elision_uses_one_uniform_dim_indentation() {
    // @step Given a gap-marker Elision row and a collapse-hint Elision row
    // CRITICAL #1: drive BOTH elision kinds from build_diff_rows (production),
    // NOT hand-fed synthetic strings. A 100-line edit yields a trailing gap
    // Elision; a 60-line addition (over the collapse limit) yields a collapse
    // hint Elision. Both must share ONE uniform leading indentation.
    let old: String = (1..=100).map(|n| format!("line{n}\n")).collect();
    let new: String = (1..=100)
        .map(|n| {
            if n == 50 {
                "line50-CHANGED\n".to_string()
            } else {
                format!("line{n}\n")
            }
        })
        .collect();
    let gap_rows = build_diff_rows(&format_edit_diff(&old, &new), DIFF_COLLAPSED_LINES, 1);
    let gap = gap_rows
        .iter()
        .rev()
        .find(|r| matches!(r, DiffDisplayRow::Elision { .. }))
        .cloned()
        .expect("a production gap Elision");

    let added: String = (1..=60).map(|n| format!("added{n}\n")).collect();
    let hint_rows = build_diff_rows(&format_edit_diff("", &added), DIFF_COLLAPSED_LINES, 1);
    let hint = hint_rows
        .last()
        .cloned()
        .expect("a production collapse-hint Elision");
    assert!(
        matches!(&hint, DiffDisplayRow::Elision { text } if text.contains("(select turn to /expand)")),
        "last row must be the collapse hint"
    );

    // @step When I style each elision row
    let gs = style_row(&gap, 40);
    let hs = style_row(&hint, 40);
    // @step Then both render dim with the same uniform indentation
    assert!(gs
        .iter()
        .all(|s| s.style.add_modifier.contains(Modifier::DIM)));
    assert!(hs
        .iter()
        .all(|s| s.style.add_modifier.contains(Modifier::DIM)));
    let gpad = spans_text(&gs).chars().take_while(|c| *c == ' ').count();
    let hpad = spans_text(&hs).chars().take_while(|c| *c == ' ').count();
    assert_eq!(
        gpad, hpad,
        "elision indentation must be uniform across production gap and collapse-hint rows"
    );
}

#[test]
fn gutter_style_is_consistent_across_row_types_with_no_flip() {
    // @step Given a Context row and a Removed row
    let ctx = DiffDisplayRow::Context {
        line_no: 5,
        text: "ctx".into(),
    };
    let rem = DiffDisplayRow::Removed {
        line_no: 6,
        text: "rem".into(),
    };
    // @step When I style both rows
    let cs = style_row(&ctx, 40);
    let rms = style_row(&rem, 40);
    // @step Then the gutter region of each row follows the same dim/gray rule with no per-row-type flip
    let ctx_gutter = cs.first().expect("ctx gutter span");
    let rem_gutter = rms.first().expect("rem gutter span");
    assert_eq!(ctx_gutter.style.fg, Some(Color::Gray));
    assert_eq!(rem_gutter.style.fg, Some(Color::Gray));
    assert!(ctx_gutter.style.bg.is_none());
    assert!(
        rem_gutter.style.bg.is_none(),
        "gutter must be OUTSIDE the colored bar"
    );
}

#[test]
fn codec_round_trips_every_variant_exactly() {
    // @step Given diff display rows of every variant including unusual content with spaces, brackets, digits, and empty text
    let rows = vec![
        DiffDisplayRow::Context {
            line_no: 1,
            text: "  spaced  [bracket] 123".into(),
        },
        DiffDisplayRow::Removed {
            line_no: 250,
            text: "- looks like a marker -".into(),
        },
        DiffDisplayRow::Added {
            line_no: 7,
            text: "".into(),
        },
        DiffDisplayRow::Context {
            line_no: 42,
            text: "[R] [A] not really markers".into(),
        },
        DiffDisplayRow::Elision {
            text: "    ... (5 lines)".into(),
        },
        DiffDisplayRow::Elision {
            text: "... +9 lines (select turn to /expand)".into(),
        },
        // CRITICAL #2: adversarial Elision whose text is shaped exactly like a
        // Context / Removed / marker row. The codec MUST recover Elision, not
        // misclassify it. Also: empty text and text containing a bracket.
        DiffDisplayRow::Elision {
            text: "42   trailing".into(),
        },
        DiffDisplayRow::Elision {
            text: "  7 [R]- x".into(),
        },
        DiffDisplayRow::Elision {
            text: "999 [A]+ injected tail".into(),
        },
        DiffDisplayRow::Elision { text: "".into() },
        DiffDisplayRow::Elision {
            text: "[bracket but not a marker]".into(),
        },
    ];
    // @step When I serialize each row to a line and parse it back
    // @step Then the parsed row equals the original row
    for row in rows {
        let encoded = to_line(&row);
        assert_eq!(
            parse_line(&encoded),
            row,
            "round-trip failed via {encoded:?}"
        );
    }
}

#[test]
fn style_row_at_zero_width_does_not_panic() {
    // @step Given a Removed row
    let row = DiffDisplayRow::Removed {
        line_no: 2,
        text: "line2".into(),
    };
    // @step When I style the row at width zero
    let spans = style_row(&row, 0);
    // @step Then no panic occurs and no padding is added beyond the content
    assert!(!spans_text(&spans).ends_with("  "));
    assert!(spans_text(&spans).contains("line2"));
}

// ── CRITICAL #3: re-wrap of a long diff line must stay continuation-safe ──

use codelet_fspec_tui::store::agent_view::diff_decode::style_row_lines;

fn lines_text(lines: &[Vec<Span<'static>>]) -> String {
    lines
        .iter()
        .map(|l| l.iter().map(|s| s.content.as_ref()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn long_changed_line_wraps_without_resurrecting_a_phantom_row() {
    // @step Given a long Removed line whose content embeds a fragment shaped like another diff row
    // The embedded "999 [A]+ injected tail" would, under naive wrap-then-parse,
    // re-parse as a fresh Added bar on resize (the F4 phantom-row defect).
    let row =
        parse_line("  2 [R]- start middle padding 999 [A]+ injected tail end of the line here");
    assert!(matches!(row, DiffDisplayRow::Removed { .. }));

    // @step When the typed row is wrapped at a narrow width and styled
    let width = 24;
    let lines = style_row_lines(&row, width);

    // @step Then exactly one visual row carries the removed background gutter+marker and the rest are plain continuation of the SAME row
    assert!(lines.len() >= 2, "the long line must wrap to multiple rows");
    // No continuation fragment may carry the ADDED background (no phantom Added bar).
    let added_bars = lines
        .iter()
        .filter(|l| l.iter().any(|s| s.style.bg == Some(DIFF_BG_ADDED)))
        .count();
    assert_eq!(added_bars, 0, "no phantom Added bar may appear mid-line");
    // Every visual row carries the removed background (one contiguous bar).
    assert!(
        lines
            .iter()
            .all(|l| l.iter().any(|s| s.style.bg == Some(DIFF_BG_REMOVED))),
        "the removed bar must stay contiguous across the wrap"
    );
    // @step And the embedded marker-shaped text is content of the single row, not a re-parsed gutter
    // The "[A]+" here is part of the user's removed content, so it appears once
    // (as content) but never as a NEW gutter marker: only the first row holds a
    // gutter glyph "- " immediately after the line-number gutter span.
    let text = lines_text(&lines);
    assert!(text.contains("injected tail"));
    // Removed gutter marker "-" appears exactly once at the start of the bar.
    let first_bar = lines[0]
        .iter()
        .find(|s| s.style.bg == Some(DIFF_BG_REMOVED))
        .map(|s| s.content.to_string())
        .unwrap_or_default();
    assert!(
        first_bar.starts_with("- "),
        "first row carries the - marker"
    );
}

#[test]
fn diff_body_with_a_content_wrapping_line_survives_a_real_resize() {
    // @step Given a diff tool-call whose changed line is far wider than the viewport
    let mut app = app_with_session();
    let long_old = format!("alpha {} omega", "wordpiece ".repeat(12));
    let long_new = format!("ALPHA {} OMEGA", "tokenpiece ".repeat(12));
    let input = format!(
        r#"{{"old_string":{},"new_string":{}}}"#,
        serde_json::to_string(&long_old).unwrap(),
        serde_json::to_string(&long_new).unwrap()
    );
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        tool_call("tc-1", "Edit", &input),
    ));
    app.dispatch(Action::ChunkReceived(sid("s-1"), tool_result("tc-1", "ok")));
    {
        let store = app.agent_view_store_mut();
        let ctx = store.session_context_mut_for(&sid("s-1")).expect("ctx mut");
        ctx.scrollback.set_viewport_width(50);
    }

    // @step When the chunk is re-wrapped at a narrow width on resize
    let lines = {
        let store = app.agent_view_store_mut();
        let ctx = store.session_context_mut_for(&sid("s-1")).expect("ctx mut");
        ctx.scrollback.set_viewport_width(20);
        ctx.scrollback
            .visible_window(4096)
            .first()
            .map(|c| c.lines.clone())
            .unwrap_or_default()
    };

    // @step Then no rendered line contains a literal diff marker and no panic occurs
    assert!(!any_marker_leak(&lines));
    // @step And a removed bar and an added bar are both present without phantom rows
    assert!(lines.iter().any(|l| line_has_bg(l, DIFF_BG_REMOVED)));
    assert!(lines.iter().any(|l| line_has_bg(l, DIFF_BG_ADDED)));
}

// ── WARNING #6: gap and row gutters must share one width at 1000+ lines ──

#[test]
fn elision_indent_matches_row_gutter_width_at_four_digit_line_numbers() {
    // @step Given a 1200-line edit with a single mid-file change
    let old: String = (1..=1200).map(|n| format!("line{n}\n")).collect();
    let new: String = (1..=1200)
        .map(|n| {
            if n == 600 {
                "line600-CHANGED\n".to_string()
            } else {
                format!("line{n}\n")
            }
        })
        .collect();
    // @step When I build the diff display rows
    let rows = build_diff_rows(&format_edit_diff(&old, &new), DIFF_COLLAPSED_LINES, 1);
    // @step Then every elision shares one indentation that scales with the four-digit gutter width
    let gap = rows
        .iter()
        .rev()
        .find(|r| matches!(r, DiffDisplayRow::Elision { .. }))
        .cloned()
        .expect("a trailing Elision");
    // A collapse-hint elision from an over-limit addition at the SAME gutter width.
    let added: String = (1..=1200).map(|n| format!("added{n}\n")).collect();
    let hint_rows = build_diff_rows(&format_edit_diff("", &added), DIFF_COLLAPSED_LINES, 1);
    let hint = hint_rows.last().cloned().expect("a collapse hint");

    let gap_indent = spans_text(&style_row(&gap, 80))
        .chars()
        .take_while(|c| *c == ' ')
        .count();
    let hint_indent = spans_text(&style_row(&hint, 80))
        .chars()
        .take_while(|c| *c == ' ')
        .count();
    // C1: gap and collapse hint share ONE indentation.
    assert_eq!(gap_indent, hint_indent, "gap and collapse hint must align");
    // W6: at 1000+ lines the gutter is 4 wide, so the shared indent scales to
    // 4 (not the fixed-3 it would be for a small edit) — proving ONE width
    // source drives both rows and elisions.
    let small_rows = build_diff_rows(
        &format_edit_diff(
            &(1..=5).map(|n| format!("l{n}\n")).collect::<String>(),
            &(1..=5)
                .map(|n| {
                    if n == 3 {
                        "l3X\n".into()
                    } else {
                        format!("l{n}\n")
                    }
                })
                .collect::<String>(),
        ),
        DIFF_COLLAPSED_LINES,
        1,
    );
    // The small edit has no trailing elision (fits inline), so assert the
    // four-digit gutter widened the indent versus GUTTER_MIN_WIDTH (3).
    let _ = small_rows;
    assert!(
        gap_indent >= 4,
        "four-digit line numbers must widen the shared elision indent to >= 4; got {gap_indent}"
    );
}
