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

use super::chunk_wrap::{wrap_source, DEFAULT_WRAP_WIDTH};
use super::pending_tool_diff::PendingToolDiff;
use crate::terminal::sanitize::sanitize_for_terminal;
use crate::views::agent::{
    ChunkKind, ChunkSource, RenderedChunk, ScrollbackList, TrimResult, MAX_SCROLLBACK_VISUAL_ROWS,
};

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
    /// [`super::record_chunk`].
    ///
    /// **TUI-111**: ingress sanitization — every visible-text payload is
    /// run through `sanitize_for_terminal` BEFORE it is stored, so all
    /// downstream consumers (scrollback, turn modal, copy, re-wrap, mux
    /// panes) paint text cleaned exactly once.
    pub fn record_chunk(&mut self, chunk: &StreamChunk) {
        super::record_chunk::record_chunk(self, chunk);
    }

    /// **TUI-111**: `push_line` is the single choke point for every
    /// direct scrollback line (session notices, reconnect notices,
    /// slash-command echoes) — the payload is sanitized on write so the
    /// stored line is always terminal-safe.
    pub fn push_line<S: Into<String>>(&mut self, line: S) {
        let source = ChunkSource {
            text: sanitize_for_terminal(&line.into()),
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

    /// **BUG-192**: trim the scrollback to [`MAX_SCROLLBACK_VISUAL_ROWS`]
    /// and shift the in-flight slot indices to match.
    ///
    /// `inserted_at`: when a chunk was just INSERTED at `idx`, slots at or
    /// beyond `idx` also move right by 1 (the insert itself); a `push`
    /// (append at the tail) passes `None`. `trim_shift` is the net chunk
    /// shift from the trim itself (`-removed + marker`); 0 when no trim
    /// fired.
    ///
    /// Invariants that keep this safe:
    /// - the trim removes a PREFIX of chunks strictly below the protected
    ///   floor (the in-flight slots' minimum), so no slot ever points at a
    ///   removed chunk;
    /// - a slot at index 0 (in-flight chunk IS the oldest) defers the trim
    ///   (the marker would occupy its index); the insert shift still
    ///   applies and the trim self-heals on the next mutation.
    fn shift_in_flight_slots(&mut self, inserted_at: Option<usize>, trim_shift: isize) {
        for slot in [&mut self.in_flight_assistant, &mut self.in_flight_thinking] {
            if let Some(i) = slot {
                let mut new = *i as isize + trim_shift;
                if let Some(inserted_at) = inserted_at {
                    if *i >= inserted_at {
                        new += 1;
                    }
                }
                *slot = Some(new.max(0) as usize);
            }
        }
    }

    /// **BUG-192**: run the widget trim (protecting the in-flight slots) and
    /// shift the slot indices by the net chunk-count change so they keep
    /// pointing at the SAME chunks.
    ///
    /// Called after every chunk-producing `push_source` /
    /// `insert_source_at` and after every in-place in-flight growth
    /// ([`Self::rewrap_and_trim_at`]) — the only two ways the total
    /// visual-row count grows.
    ///
    /// `inserted_at` carries the insert-index shift for `insert_source_at`
    /// call sites (see [`Self::shift_in_flight_slots`]). Returns the
    /// [`crate::views::agent::TrimResult`] of the widget trim — `Default`
    /// when the trim was deferred (in-flight chunk at index 0) or did not
    /// fire.
    pub(crate) fn trim_scrollback_to_cap(&mut self, inserted_at: Option<usize>) -> TrimResult {
        let floor = self
            .in_flight_assistant
            .iter()
            .chain(self.in_flight_thinking.iter())
            .copied()
            .min();
        if floor == Some(0) {
            // In-flight chunk at index 0 — the trim would remove the
            // marker's slot; defer (the insert shift still applies below).
            self.shift_in_flight_slots(inserted_at, 0);
            return TrimResult::default();
        }
        let res = self
            .scrollback
            .trim_to_cap(MAX_SCROLLBACK_VISUAL_ROWS, floor.unwrap_or(0));
        let trim_shift = res.marker_inserted as isize - res.removed_chunks as isize;
        self.shift_in_flight_slots(inserted_at, trim_shift);
        res
    }

    /// **BUG-192**: re-wrap a single (growing) chunk and then trim to the
    /// cap, shifting the in-flight slots. All store-side in-place growth
    /// (streaming deltas, tool-card progress, settle re-wraps) funnels
    /// through here so the cap holds even between chunk pushes.
    pub(crate) fn rewrap_and_trim_at(&mut self, idx: usize) {
        self.scrollback.rewrap_at(idx);
        self.trim_scrollback_to_cap(None);
    }

    /// Lower-level push that allocates the seq cursor and performs
    /// the initial wrap. **RPC-091** pub(crate).
    ///
    /// **BUG-192**: trims after the push (see
    /// [`Self::trim_scrollback_to_cap`]); existing in-flight slots are
    /// shifted by the trim's net change (the pushed chunk is always the
    /// tail and is never removed by the trim, so callers that adopt it as
    /// a new in-flight slot read `chunk_count() - 1` afterwards).
    pub(crate) fn push_source(&mut self, source: ChunkSource) {
        let seq = self.scrollback_next_seq;
        self.scrollback_next_seq = self.scrollback_next_seq.saturating_add(1);
        let lines = wrap_source(&source, DEFAULT_WRAP_WIDTH);
        self.scrollback.push(RenderedChunk {
            seq,
            lines,
            source: Some(source),
        });
        self.trim_scrollback_to_cap(None);
    }

    /// Insert a chunk at `idx`, shifting subsequent chunks right.
    /// Mirrors [`push_source`] but uses
    /// [`ScrollbackList::insert`]. **RPC-093**: used by
    /// `chunk_processor::append_thinking` to splice a new thinking
    /// chunk BEFORE an in-flight assistant chunk (TS parity with
    /// `appendThinking` splice-before-streaming-assistant rule).
    ///
    /// Returns the allocated `seq`.
    ///
    /// **BUG-192**: trims after the insert and shifts the in-flight slots
    /// by BOTH the insert (+1 for slots at or beyond `idx`) and the trim's
    /// net change, so pre-existing slots keep pointing at the same chunks.
    pub(crate) fn insert_source_at(&mut self, idx: usize, source: ChunkSource) -> u64 {
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
        self.trim_scrollback_to_cap(Some(idx));
        seq
    }
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
}
