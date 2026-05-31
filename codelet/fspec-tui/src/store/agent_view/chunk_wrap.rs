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

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::views::agent::text_wrap::wrap_to_width;
use crate::views::agent::{ChunkKind, ChunkSource};

/// Default viewport width used when a chunk is recorded before the
/// scrollback widget has observed its first render area.
pub const DEFAULT_WRAP_WIDTH: u16 = 80;

pub fn wrap_source(source: &ChunkSource, width: u16) -> Vec<Line<'static>> {
    let style = Style::default().fg(source.color);
    let body_style = match &source.kind {
        ChunkKind::ToolCall { is_error: true, .. } => Style::default().fg(Color::Red),
        _ => style,
    };
    let prefix = match &source.kind {
        ChunkKind::UserInput => "You: ",
        ChunkKind::AssistantText | ChunkKind::ToolCall { .. } => "\u{25CF} ",
        ChunkKind::Thinking
        | ChunkKind::Error
        | ChunkKind::Interrupted
        | ChunkKind::Notification
        | ChunkKind::Incoming => "",
    };
    let mut out: Vec<Line<'static>> = Vec::new();
    // **RPC-093**: Thinking chunks render with a `[Thinking]`
    // header line on top, then the wrapped body. The header is
    // NOT stored in `source.text`.
    if matches!(source.kind, ChunkKind::Thinking) {
        out.push(Line::from(Span::styled("[Thinking]".to_string(), style)));
    }
    let hard_lines: Vec<&str> = source.text.split('\n').collect();
    for (i, hard) in hard_lines.iter().enumerate() {
        let mut wrapped = wrap_to_width(hard, width as usize);
        if wrapped.is_empty() {
            wrapped.push(String::new());
        }
        for (j, w) in wrapped.into_iter().enumerate() {
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
    // NOTE: streaming "..." suffix is intentionally NOT appended —
    // parity with TS `conversationUtils.ts:88-90` is deferred per
    // `agentview-chunk-rendering-parity.feature`.
    out
}
