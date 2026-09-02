//! `SessionContext` — per-session state container introduced by RPC-024.
//!
//! Feature: spec/features/rpc024-multi-session-store.feature
//!          spec/features/agentview-chunk-rendering-parity.feature
//!          spec/features/agentview-chunkprocessor-parity.feature
//!
//! Each open AgentView session keeps its own scrollback, scrollback-
//! sequence cursor, and input-draft string so cycling between sessions
//! (Shift+←/→) preserves per-session UI state.
//!
//! **RPC-091**: `record_chunk` dispatches to `chunk_processor`, which
//! is a faithful port of the TS Ink `processStreamingChunk` algorithm
//! (`src/tui/utils/chunkProcessor.ts`). Streaming Text deltas
//! accumulate into a single in-flight AssistantText chunk; ToolCall /
//! ToolResult / ToolProgress are rendered as cards; Done finalises
//! markdown tables.

use codelet_rpc_types::{SessionId, StreamChunk};
use ratatui::style::Color;
use std::collections::HashMap;

use super::chunk_processor::{
    append_assistant_text, append_thinking, flush_in_flight_drop_empty, handle_done, handle_error,
    handle_tool_call, handle_tool_progress, handle_tool_result,
};
use super::chunk_wrap::{wrap_source, DEFAULT_WRAP_WIDTH};
use super::pending_tool_diff::PendingToolDiff;
use crate::views::agent::{ChunkKind, ChunkSource, RenderedChunk, ScrollbackList};

#[derive(Debug)]
pub struct SessionContext {
    pub id: SessionId,
    pub work_unit_id: Option<String>,
    pub scrollback: ScrollbackList,
    pub scrollback_next_seq: u64,
    pub input_draft: String,
    /// **RPC-091**: index into `scrollback.chunks` of the currently-
    /// accumulating `AssistantText` chunk. Cleared on `Done` / `Error`
    /// / `Interrupted` and any `ToolCall` (implicit flush).
    pub in_flight_assistant: Option<usize>,
    /// **RPC-093**: index into `scrollback.chunks` of the currently-
    /// accumulating `Thinking` chunk. Cleared on `Done` / `Error` /
    /// `Interrupted` / `UserInput` (turn boundary — slot-only clear,
    /// chunk untouched) and on `ToolCall` (where the chunk is also
    /// finalised — `is_streaming` set to false).
    ///
    /// Analogue of TS `findActiveThinkingBlock` from
    /// `src/tui/utils/thinkingBlockManager.ts`: "last streaming
    /// thinking with no `UserInput` after it".
    pub in_flight_thinking: Option<usize>,
    /// **RPC-391**: Edit/Write tool inputs captured at tool-call time,
    /// keyed by `ToolCallInfo.id`, consumed on the matching ToolResult to
    /// build the colored diff. Mirrors TS `pendingToolDiffsRef`.
    pub pending_tool_diffs: HashMap<String, PendingToolDiff>,
}

impl SessionContext {
    pub fn new(id: SessionId) -> Self {
        Self {
            id,
            work_unit_id: None,
            scrollback: ScrollbackList::new(),
            scrollback_next_seq: 0,
            input_draft: String::new(),
            in_flight_assistant: None,
            in_flight_thinking: None,
            pending_tool_diffs: HashMap::new(),
        }
    }

    pub fn with_work_unit(id: SessionId, work_unit_id: Option<String>) -> Self {
        let mut ctx = Self::new(id);
        ctx.work_unit_id = work_unit_id;
        ctx
    }

    /// Append a chunk's rendered lines to this session's scrollback,
    /// using the TS Ink chunkProcessor accumulation algorithm
    /// (RPC-091). All variant-specific logic lives in
    /// [`super::chunk_processor`].
    pub fn record_chunk(&mut self, chunk: &StreamChunk) {
        match chunk {
            StreamChunk::Text { text, .. } => append_assistant_text(self, text),
            StreamChunk::UserInput { text } => {
                flush_in_flight_drop_empty(self);
                // RPC-093: UserInput is a turn boundary. Clear the
                // thinking slot WITHOUT mutating the existing chunk
                // (parity with TS findActiveThinkingBlock returning -1
                // once a user-input message follows).
                self.in_flight_thinking = None;
                self.push_chunk(ChunkSource {
                    text: text.clone(),
                    color: Color::Green,
                    kind: ChunkKind::UserInput,
                    is_streaming: false,
                    full_text: None,
                });
            }
            StreamChunk::Thinking { thinking, .. } => {
                // RPC-093: port of TS appendThinking — accumulate into
                // the in-flight thinking chunk, or splice a new one
                // (BEFORE in_flight_assistant when present).
                append_thinking(self, thinking);
            }
            StreamChunk::ToolCall { tool_call, .. } => handle_tool_call(self, tool_call),
            StreamChunk::ToolResult { tool_result, .. } => handle_tool_result(self, tool_result),
            StreamChunk::ToolProgress { tool_progress, .. } => {
                handle_tool_progress(self, tool_progress)
            }
            StreamChunk::Done => handle_done(self),
            StreamChunk::Error { error } => handle_error(self, error),
            StreamChunk::Interrupted { .. } => {
                flush_in_flight_drop_empty(self);
                // RPC-093: Interrupted is a flush trigger. Clear the
                // thinking slot WITHOUT mutating the existing chunk.
                self.in_flight_thinking = None;
                self.push_chunk(ChunkSource {
                    text: "\u{26A0} Interrupted".to_string(),
                    color: Color::White,
                    kind: ChunkKind::Interrupted,
                    is_streaming: false,
                    full_text: None,
                });
            }
            StreamChunk::UserNotification { message, .. } => {
                self.push_chunk(ChunkSource {
                    text: message.clone(),
                    color: Color::White,
                    kind: ChunkKind::Notification,
                    is_streaming: false,
                    full_text: None,
                });
            }
            StreamChunk::IncomingMessage { text, .. } => {
                let (role, body) = parse_supervisor_envelope(text);
                self.push_chunk(ChunkSource {
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

    pub fn push_line<S: Into<String>>(&mut self, line: S) {
        let source = ChunkSource {
            text: line.into(),
            color: Color::White,
            kind: ChunkKind::Notification,
            is_streaming: false,
            full_text: None,
        };
        self.push_source(source);
    }

    pub fn reset_scrollback(&mut self) {
        self.scrollback.reset();
        self.scrollback_next_seq = 0;
        self.in_flight_assistant = None;
        self.in_flight_thinking = None;
        self.pending_tool_diffs.clear();
    }

    /// Push a chunk with whatever `is_streaming` the caller set.
    /// **RPC-091**: exposed `pub(crate)` so `chunk_processor` can push.
    pub(crate) fn push_chunk(&mut self, source: ChunkSource) {
        self.push_source(source);
    }

    /// Lower-level push that allocates the seq cursor and performs
    /// the initial wrap. **RPC-091** pub(crate).
    pub(crate) fn push_source(&mut self, source: ChunkSource) {
        let seq = self.scrollback_next_seq;
        self.scrollback_next_seq = self.scrollback_next_seq.saturating_add(1);
        let lines = wrap_source(&source, DEFAULT_WRAP_WIDTH);
        self.scrollback.push(RenderedChunk {
            seq,
            lines,
            source: Some(source),
        });
    }

    /// Insert a chunk at `idx`, shifting subsequent chunks right.
    /// Mirrors [`push_source`] but uses
    /// [`ScrollbackList::insert`]. **RPC-093**: used by
    /// `chunk_processor::append_thinking` to splice a new thinking
    /// chunk BEFORE an in-flight assistant chunk (TS parity with
    /// `appendThinking` splice-before-streaming-assistant rule).
    pub(crate) fn insert_source_at(&mut self, idx: usize, source: ChunkSource) {
        let seq = self.scrollback_next_seq;
        self.scrollback_next_seq = self.scrollback_next_seq.saturating_add(1);
        let lines = wrap_source(&source, DEFAULT_WRAP_WIDTH);
        self.scrollback.insert(
            idx,
            RenderedChunk {
                seq,
                lines,
                source: Some(source),
            },
        );
    }
}

/// Parse a `StreamChunk::IncomingMessage` body of the form
/// `"[SUPERVISOR: <role> | Session: <sid>]<sep><body>"` where `<sep>` is a
/// space or a newline. The backend (`format_incoming_message`) uses a space;
/// replay/legacy paths may use `\n`. Mirrors the TS reference
/// (`src/tui/utils/chunkProcessor.ts`), which consumes the header up to `]`
/// and an optional newline, so the body survives either separator.
fn parse_supervisor_envelope(raw: &str) -> (String, String) {
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
    fn new_context_has_empty_scrollback_and_draft() {
        let ctx = SessionContext::new(SessionId::new("s-1"));
        assert_eq!(ctx.id, SessionId::new("s-1"));
        assert_eq!(ctx.scrollback.chunk_count(), 0);
        assert_eq!(ctx.scrollback_next_seq, 0);
        assert_eq!(ctx.input_draft, "");
        assert!(ctx.work_unit_id.is_none());
        assert!(ctx.in_flight_assistant.is_none());
    }

    #[test]
    fn streaming_text_accumulates_into_single_chunk() {
        let mut ctx = SessionContext::new(SessionId::new("s-1"));
        ctx.record_chunk(&StreamChunk::text("Hello".to_string()));
        ctx.record_chunk(&StreamChunk::text(" world".to_string()));
        assert_eq!(ctx.scrollback.chunk_count(), 1);
        assert_eq!(ctx.in_flight_assistant, Some(0));
    }

    #[test]
    fn reset_scrollback_drops_chunks_and_resets_seq() {
        let mut ctx = SessionContext::new(SessionId::new("s-1"));
        ctx.record_chunk(&StreamChunk::text("hi".to_string()));
        ctx.reset_scrollback();
        assert_eq!(ctx.scrollback.chunk_count(), 0);
        assert_eq!(ctx.scrollback_next_seq, 0);
        assert!(ctx.in_flight_assistant.is_none());
    }

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
