//! `wrap_source` — convert a `ChunkSource` into pre-wrapped
//! `Line`s for a given viewport width.
//!
//! Feature: spec/features/agentview-chunk-rendering-parity.feature
//!          spec/features/agentview-chunkprocessor-parity.feature
//!          spec/features/agentview-thinking-streaming-parity.feature
//!
//! Extracted from `session_context.rs` to keep that file under
//! the 300-LoC ceiling pinned by `rpc024-source-shape.feature`.
//!
//! **RPC-091**: per-variant prefixes (`'● '` for AssistantText /
//! ToolCall, `'You: '` for UserInput) are applied here at render
//! time — NOT baked into `ChunkSource::text`.
//!
//! **RPC-093**: `ChunkKind::Thinking` renders a `[Thinking]`
//! header line on top of the wrapped body for the same reason.
//!
//! **RPC-389**: `ChunkKind::ToolCall` bodies are collapsed/windowed at
//! this render layer (NOT in `ChunkSource::text`, which stays full for
//! the `TurnContentModal`). Streaming cards keep the header + the last 10
//! body lines (tail window). Mirrors `AgentView.tsx::formatCollapsedOutput`
//! (8) + `createStreamingWindow` (10), counting hard `\n`-delimited body
//! lines pre-wrap.
//!
//! **RPC-399**: settled tool-call cards are END-pinned — they keep the
//! header + the LAST 8 body lines + a `... +N lines (Tab to select, Enter to view full)`
//! indicator (N = lines hidden ABOVE the window). This supersedes the
//! original RPC-389 first-8 behavior so a card that finishes streaming
//! stays anchored to the last lines the user was watching instead of
//! jumping back to the start.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::diff_codec::parse_line;
use super::diff_decode::style_row_lines;
use super::stderr::{strip_marker, STDERR_MARKER};
use crate::views::agent::text_wrap::{wrap_rows, wrap_rows_hanging, RowEnd};
use crate::views::agent::{ChunkKind, ChunkSource};

/// Default viewport width used when a chunk is recorded before the
/// scrollback widget has observed its first render area.
pub const DEFAULT_WRAP_WIDTH: u16 = 80;

/// **RPC-389**: body lines kept inline when a tool-call card is settled.
/// Mirrors TS `COLLAPSED_LINES` (`AgentView.tsx:534`).
const COLLAPSED_LINES: usize = 8;

/// **RPC-389**: tail-window size while a tool-call card is streaming.
/// Mirrors TS `STREAMING_WINDOW_SIZE` (`AgentView.tsx:533`).
const STREAMING_WINDOW_SIZE: usize = 10;

pub fn wrap_source(source: &ChunkSource, width: u16) -> Vec<Line<'static>> {
    wrap_source_rows(source, width).0
}

/// TUI-130: [`wrap_source`] with how each line ends in copied text
/// ([`RowEnd`]), index for index, so a scrollback copy joins the rows the
/// wrap split without a line break.
pub fn wrap_source_rows(source: &ChunkSource, width: u16) -> (Vec<Line<'static>>, Vec<RowEnd>) {
    let style = Style::default().fg(source.color);
    let body_style = match &source.kind {
        ChunkKind::ToolCall { is_error: true, .. } => Style::default().fg(Color::Red),
        _ => style,
    };
    let prefix = match &source.kind {
        ChunkKind::UserInput => "You: ".to_string(),
        ChunkKind::TaggedUserInput(tag) => tag.prefix(),
        ChunkKind::AssistantText | ChunkKind::ToolCall { .. } => "\u{25CF} ".to_string(),
        ChunkKind::Thinking
        | ChunkKind::Error
        | ChunkKind::Interrupted
        | ChunkKind::Notification
        | ChunkKind::Incoming => String::new(),
    };
    let mut out: Vec<Line<'static>> = Vec::new();
    let mut ends: Vec<RowEnd> = Vec::new();
    // **RPC-093**: Thinking chunks render with a `[Thinking]`
    // header line on top, then the wrapped body. The header is
    // NOT stored in `source.text`.
    if matches!(source.kind, ChunkKind::Thinking) {
        out.push(Line::from(Span::styled("[Thinking]".to_string(), style)));
        ends.push(RowEnd::Break);
    }

    // **RPC-389**: ToolCall bodies are collapsed/windowed here (the
    // header line — first `\n`-segment — is always kept). The fully
    // rendered lines (including the dimmed indicator) are captured into
    // `out` and fall through to the single trailing-separator append
    // below (**RPC-401**), so tool-call cards get the same blank gutter
    // as every other `ChunkKind`.
    if matches!(source.kind, ChunkKind::ToolCall { .. }) {
        let is_error = matches!(source.kind, ChunkKind::ToolCall { is_error: true, .. });
        (out, ends) = wrap_tool_call(source, width, style, body_style, &prefix, is_error);
    } else {
        let hard_lines: Vec<&str> = source.text.split('\n').collect();
        for (i, hard) in hard_lines.iter().enumerate() {
            // TUI-130: the first row starts after the prefix, so it is
            // narrower by the prefix width and never cut off.
            let first_width = if i == 0 {
                (width as usize).saturating_sub(prefix.width())
            } else {
                width as usize
            };
            let wrapped = wrap_rows_hanging(hard, first_width, width as usize);
            for (j, (w, end)) in wrapped.into_iter().enumerate() {
                ends.push(end);
                let is_first = i == 0 && j == 0;
                let needs_prefix = is_first && !prefix.is_empty();
                let row_style = if i == 0 { style } else { body_style };
                if needs_prefix {
                    out.push(Line::from(Span::styled(format!("{prefix}{w}"), style)));
                } else {
                    out.push(Line::from(Span::styled(w, row_style)));
                }
            }
        }
    }
    // NOTE: streaming "..." suffix is intentionally NOT appended —
    // parity with TS `conversationUtils.ts:88-90` is deferred per
    // `agentview-chunk-rendering-parity.feature`.
    //
    // **RPC-401**: append exactly ONE blank separator line after every
    // chunk's content — the single trailing gutter row that matches the
    // TS `wrapMessageToLines` `addSeparator=true` default
    // (`conversationUtils.ts:117-127`). Applied here (the sole entry
    // point used by `push_source`, `insert_source_at` and `rewrap_chunk`)
    // so it flows uniformly into `total_visual_rows`, resize rewrap,
    // painting and the selection arrow-bar gutters — for EVERY
    // `ChunkKind`. It does NOT leak into the `TurnContentModal`, which
    // sources full text from `ChunkSource::text` / `full_text_for_seq`.
    out.push(Line::default());
    ends.push(RowEnd::Break);
    (out, ends)
}

/// **RPC-389**: render a `ChunkKind::ToolCall` source with its body
/// collapsed (settled) or tail-windowed (streaming).
///
/// `source.text` is `"ToolName(args)\n<body...>"` — the header is the
/// first `\n`-segment, the body is everything after it. The collapse
/// threshold counts hard `\n`-delimited BODY lines (pre-wrap) to match
/// the TS `content.split('\n')` semantics; the retained lines are then
/// width-wrapped exactly as the default path does.
fn wrap_tool_call(
    source: &ChunkSource,
    width: u16,
    style: Style,
    body_style: Style,
    prefix: &str,
    is_error: bool,
) -> (Vec<Line<'static>>, Vec<RowEnd>) {
    let (header, body) = match source.text.split_once('\n') {
        Some((h, b)) => (h, Some(b)),
        None => (source.text.as_str(), None),
    };

    let mut out: Vec<Line<'static>> = Vec::new();
    let mut ends: Vec<RowEnd> = Vec::new();
    // Header line — always kept, carries the `● ` prefix.
    for (j, (w, end)) in wrap_header(header, width, prefix).into_iter().enumerate() {
        ends.push(end);
        let needs_prefix = j == 0 && !prefix.is_empty();
        if needs_prefix {
            out.push(Line::from(Span::styled(format!("{prefix}{w}"), style)));
        } else {
            out.push(Line::from(Span::styled(w, style)));
        }
    }

    let Some(body) = body else {
        return (out, ends);
    };

    // **RPC-393 (CRITICAL #3)**: diff cards parse each HARD canonical line back
    // to a typed `DiffDisplayRow` ONCE (via the single codec) and wrap it
    // continuation-safe through `style_row_lines` — the gutter/marker/bar is
    // styled only on the first visual row, so a long content-wrapping diff line
    // can never resurrect a phantom colored/context row on resize. The diff is
    // already self-collapsed at 25 by `build_diff_rows`, so it BYPASSES the
    // RPC-389 8-line collapse.
    if matches!(source.kind, ChunkKind::ToolCall { is_diff: true, .. }) {
        for hard in body.split('\n') {
            let row = parse_line(hard);
            for spans in style_row_lines(&row, width as usize) {
                out.push(Line::from(spans));
                ends.push(RowEnd::Break);
            }
        }
        return (out, ends);
    }

    let body_lines: Vec<&str> = body.split('\n').collect();
    let (visible, indicator) = collapse_tool_body(&body_lines, source.is_streaming);

    for hard in visible {
        // RPC-400: a body line is red when the whole card is an error OR the
        // line carries the stderr marker; the marker is stripped BEFORE wrap
        // so it never reaches the screen (parity with AgentView.tsx:5393-5422).
        let red = is_error || hard.contains(STDERR_MARKER);
        let stripped = strip_marker(hard);
        let line_style = if red {
            Style::default().fg(Color::Red)
        } else {
            body_style
        };
        for (w, end) in wrap_rows(&stripped, width as usize) {
            out.push(Line::from(Span::styled(w, line_style)));
            ends.push(end);
        }
    }

    if let Some(text) = indicator {
        // Indicator renders dimmed — match the existing DIM convention
        // used elsewhere in the AgentView (e.g. popup hint rows).
        let dim = Style::default().add_modifier(Modifier::DIM);
        out.push(Line::from(Span::styled(text, dim)));
        ends.push(RowEnd::Break);
    }

    (out, ends)
}

/// Wrap a tool-call header line, accounting for the `● ` prefix width on
/// the first visual row.
fn wrap_header(header: &str, width: u16, prefix: &str) -> Vec<(String, RowEnd)> {
    let first_width = (width as usize).saturating_sub(prefix.width());
    wrap_rows(header, first_width.max(1))
}

/// **RPC-389/RPC-399**: choose the visible body lines + optional indicator.
///
/// - Streaming: last `STREAMING_WINDOW_SIZE` lines, no indicator.
/// - Settled: if `> COLLAPSED_LINES`, the LAST `COLLAPSED_LINES` lines
///   (end-pinned) + a `... +N lines (Tab to select, Enter to view full)` indicator, where
///   N is the number of lines hidden ABOVE the window (N = total - 8);
///   else all lines, no indicator.
fn collapse_tool_body<'a>(
    body_lines: &[&'a str],
    is_streaming: bool,
) -> (Vec<&'a str>, Option<String>) {
    let total = body_lines.len();
    if is_streaming {
        if total > STREAMING_WINDOW_SIZE {
            let start = total - STREAMING_WINDOW_SIZE;
            return (body_lines[start..].to_vec(), None);
        }
        return (body_lines.to_vec(), None);
    }
    if total > COLLAPSED_LINES {
        let remaining = total - COLLAPSED_LINES;
        let indicator = super::diff_codec::hidden_lines_hint(remaining);
        let start = total - COLLAPSED_LINES;
        return (body_lines[start..].to_vec(), Some(indicator));
    }
    (body_lines.to_vec(), None)
}

#[cfg(test)]
#[path = "chunk_wrap_tests.rs"]
mod tests;
