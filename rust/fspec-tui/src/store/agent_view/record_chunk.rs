//! **TUI-111** — scrollback-chunk ingress recording for `SessionContext`.
//!
//! Feature: spec/features/sanitize-all-tui-output-at-ingress.feature
//!
//! Extracted from `session_context.rs` (which stays under the 300-LoC
//! ceiling pinned by `rpc024-source-shape.feature`) after the ingress
//! sanitization refactor: every visible-text chunk payload is run
//! through `sanitize_for_terminal` BEFORE it is stored, so all
//! downstream consumers (scrollback, turn modal, copy, re-wrap, mux
//! panes) paint text cleaned exactly once.
//!
//! Rule [10]: ToolProgress/ToolResult text is sanitized FIRST, then the
//! STDERR marker is applied by `maybe_mark` in `chunk_tool_result`
//! (the marker is stripped at render time by `strip_marker`).

use codelet_rpc_types::StreamChunk;
use ratatui::style::Color;

use super::chunk_processor::{
    append_assistant_text, append_thinking, flush_in_flight_drop_empty, handle_done, handle_error,
};
use super::chunk_tool_result::{handle_tool_call, handle_tool_progress, handle_tool_result};
use super::session_context::SessionContext;
use crate::terminal::sanitize::sanitize_for_terminal;
use crate::views::agent::{ChunkKind, ChunkSource};

/// Append a chunk's rendered lines to this session's scrollback,
/// using the TS Ink chunkProcessor accumulation algorithm (RPC-091).
/// All variant-specific logic lives in [`super::chunk_processor`] /
/// [`super::chunk_tool_result`].
pub fn record_chunk(ctx: &mut SessionContext, chunk: &StreamChunk) {
    match chunk {
        StreamChunk::Text { text, .. } => {
            let clean = sanitize_for_terminal(text);
            append_assistant_text(ctx, &clean);
        }
        StreamChunk::UserInput { text } => {
            let clean = sanitize_for_terminal(text);
            flush_in_flight_drop_empty(ctx);
            // RPC-093: UserInput is a turn boundary. Clear the thinking
            // slot WITHOUT mutating the existing chunk (parity with TS
            // findActiveThinkingBlock returning -1 once a user-input
            // message follows).
            ctx.in_flight_thinking = None;
            ctx.push_chunk(ChunkSource {
                text: clean,
                color: Color::Green,
                kind: ChunkKind::UserInput,
                is_streaming: false,
                full_text: None,
            });
        }
        StreamChunk::Thinking { thinking, .. } => {
            // RPC-093: port of TS appendThinking — accumulate into the
            // in-flight thinking chunk, or splice a new one (BEFORE
            // in_flight_assistant when present).
            let clean = sanitize_for_terminal(thinking);
            append_thinking(ctx, &clean);
        }
        StreamChunk::ToolCall { tool_call, .. } => handle_tool_call(ctx, tool_call),
        StreamChunk::ToolResult { tool_result, .. } => {
            // TUI-111: sanitize the content at ingress (rule [3]). The
            // stderr marker is applied AFTER, in handle_tool_progress /
            // the ToolResult body fold (rule [10]).
            let sanitized = sanitize_for_terminal(&tool_result.content);
            let mut clean = tool_result.clone();
            clean.content = sanitized;
            handle_tool_result(ctx, &clean);
        }
        StreamChunk::ToolProgress { tool_progress, .. } => {
            let sanitized = sanitize_for_terminal(&tool_progress.output_chunk);
            let mut clean = tool_progress.clone();
            clean.output_chunk = sanitized;
            handle_tool_progress(ctx, &clean)
        }
        StreamChunk::Done => handle_done(ctx),
        StreamChunk::Error { error } => {
            let clean = sanitize_for_terminal(error);
            handle_error(ctx, &clean);
        }
        StreamChunk::Interrupted { .. } => {
            flush_in_flight_drop_empty(ctx);
            // RPC-093: Interrupted is a flush trigger. Clear the
            // thinking slot WITHOUT mutating the existing chunk.
            ctx.in_flight_thinking = None;
            ctx.push_chunk(ChunkSource {
                text: "\u{26A0} Interrupted".to_string(),
                color: Color::White,
                kind: ChunkKind::Interrupted,
                is_streaming: false,
                full_text: None,
            });
        }
        StreamChunk::UserNotification { message, .. } => {
            let clean = sanitize_for_terminal(message);
            ctx.push_chunk(ChunkSource {
                text: clean,
                color: Color::White,
                kind: ChunkKind::Notification,
                is_streaming: false,
                full_text: None,
            });
        }
        StreamChunk::IncomingMessage { text, .. } => {
            // TUI-111: sanitize the envelope text FIRST, then parse the
            // role/body (so a control char can never land in the stored
            // "[W] role> body" line — rule [3]).
            let clean = sanitize_for_terminal(text);
            let (role, body) = parse_supervisor_envelope(&clean);
            let (role, body) = (sanitize_for_terminal(&role), sanitize_for_terminal(&body));
            ctx.push_chunk(ChunkSource {
                text: format!("[W] {role}> {body}"),
                color: Color::Magenta,
                kind: ChunkKind::Incoming,
                is_streaming: false,
                full_text: None,
            });
        }
        // State-only chunks — consumed elsewhere.
        StreamChunk::SessionStateChange { .. }
        | StreamChunk::IsolationStateChange { .. }
        | StreamChunk::DebugStateChange { .. }
        | StreamChunk::FooterStateUpdate { .. }
        | StreamChunk::FspecCommandRequest { .. }
        | StreamChunk::FspecCommandResult { .. }
        | StreamChunk::WorkUnitsUpdate { .. }
        | StreamChunk::SupervisorPendingInjection { .. }
        | StreamChunk::CompactionComplete { .. }
        | StreamChunk::TokenUpdate { .. }
        | StreamChunk::ContinueStateUpdate { .. }
        | StreamChunk::ContextFillUpdate { .. }
        | StreamChunk::ExecStdinRequest { .. }
        | StreamChunk::ExecStdinRequestCleared => {}
    }
}

/// Parse a `StreamChunk::IncomingMessage` body of the form
/// `"[SUPERVISOR: <role> | Session: <sid>]<sep><body>"` where `<sep>` is a
/// space or a newline. The backend (`format_incoming_message`) uses a space;
/// replay/legacy paths may use `\n`. Mirrors the TS reference
/// (`src/tui/utils/chunkProcessor.ts`), which consumes the header up to `]`
/// and an optional newline, so the body survives either separator.
pub(super) fn parse_supervisor_envelope(raw: &str) -> (String, String) {
    if !raw.starts_with('[') {
        return ("supervisor".to_string(), raw.to_string());
    }
    let Some(close_idx) = raw.find(']') else {
        return ("supervisor".to_string(), raw.to_string());
    };
    let header = &raw[..close_idx]; // excludes ']'
    let body = raw[close_idx + 1..]
        .trim_start_matches(['\n', ' '])
        .to_string();
    let inner = header.trim_start_matches('[');
    let role_segment = inner.split('|').next().unwrap_or(inner).trim();
    let role = role_segment
        .strip_prefix("SUPERVISOR:")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "supervisor".to_string());
    (role, body)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn parse_supervisor_envelope_extracts_role_and_body() {
        let (role, body) =
            parse_supervisor_envelope("[SUPERVISOR: reviewer | Session: s-2]\nplease check this");
        assert_eq!(role, "reviewer");
        assert_eq!(body, "please check this");
    }

    #[test]
    fn parse_supervisor_envelope_falls_back_to_default_role() {
        let (role, body) = parse_supervisor_envelope("raw body without header");
        assert_eq!(role, "supervisor");
        assert_eq!(body, "raw body without header");
    }
}
