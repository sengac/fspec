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
//!
//! **BUG-192**: the scrollback push/insert + cap-trim machinery lives in
//! [`super::scrollback_mutate`] — this file only keeps the one-line
//! method delegates so the 300-LoC ceiling pinned by
//! `spec/features/rpc024-multi-session-cycling.feature` holds.

use codelet_rpc_types::{SessionId, StreamChunk};
use ratatui::style::Color;
use std::collections::HashMap;

use super::pending_tool_diff::PendingToolDiff;
use super::scrollback_mutate;
use crate::terminal::sanitize::sanitize_for_terminal;
use crate::views::agent::{
    session_transition::SessionTransition, ChunkKind, ChunkSource, ScrollbackList,
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
    /// **BUG-194**: the session's own thinking-indicator transition
    /// slot (per-session `InputTransitionState` + spinner clock).
    /// Ticked by `AgentView::tick_session_transition` from this
    /// session's own `SessionStatus` — focus movement never touches it.
    pub input_transition: SessionTransition,
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
            input_transition: SessionTransition::default(),
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
        // BUG-194: a session reset drops the in-flight thinking-indicator
        // transition (no stale spinner/finish-sweep survives a /clear).
        self.input_transition = SessionTransition::default();
    }

    // ── BUG-192: one-line delegates to `scrollback_mutate` ──────────────
    // (the machinery itself moved there to keep this file < 300 LoC)

    /// Push a chunk with whatever `is_streaming` the caller set.
    /// **RPC-091**: exposed `pub(crate)` so `chunk_processor` can push.
    pub(crate) fn push_chunk(&mut self, source: ChunkSource) {
        scrollback_mutate::push_source(self, source);
    }

    /// Lower-level push that allocates the seq cursor and performs the
    /// initial wrap. **RPC-091** pub(crate). Trims after the push and
    /// shifts the in-flight slots by the trim's net change — see
    /// [`scrollback_mutate::push_source`].
    pub(crate) fn push_source(&mut self, source: ChunkSource) {
        scrollback_mutate::push_source(self, source);
    }

    /// Insert a chunk at `idx`, shifting subsequent chunks right.
    /// Returns the allocated `seq`. **RPC-093** / **BUG-192** — see
    /// [`scrollback_mutate::insert_source_at`].
    pub(crate) fn insert_source_at(&mut self, idx: usize, source: ChunkSource) -> u64 {
        scrollback_mutate::insert_source_at(self, idx, source)
    }

    /// **BUG-192**: re-wrap a single (growing) chunk and then trim to
    /// the cap, shifting the in-flight slots — see
    /// [`scrollback_mutate::rewrap_and_trim_at`].
    pub(crate) fn rewrap_and_trim_at(&mut self, idx: usize) {
        scrollback_mutate::rewrap_and_trim_at(self, idx);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use std::time::Instant;

    use super::*;
    use crate::views::agent::input_transition::InputTransitionState;

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

    /// BUG-194 rule: the per-session transition slot is reset on
    /// `reset_scrollback` (the /clear path) — no stale spinner or
    /// finish-sweep survives a session reset.
    #[test]
    fn bug194_reset_scrollback_clears_the_per_session_transition_slot() {
        let mut ctx = SessionContext::new(SessionId::new("s-1"));
        // Arm the slot as if the pane had been ticking a Running
        // session (Loading phase + spinner clock).
        ctx.input_transition.state = InputTransitionState::Loading { elapsed_ms: 480 };
        ctx.input_transition.spinner_started_at = Some(Instant::now());
        assert!(
            ctx.input_transition.is_animating()
                || matches!(
                    ctx.input_transition.state,
                    InputTransitionState::Loading { .. }
                )
        );
        ctx.reset_scrollback();
        assert!(
            matches!(ctx.input_transition.state, InputTransitionState::Idle),
            "the transition phase must be Idle after a session reset"
        );
        assert!(
            ctx.input_transition.spinner_started_at.is_none(),
            "the per-session spinner clock must be cleared on reset"
        );
        assert!(
            ctx.input_transition.last_spinner_line.is_none(),
            "the cached spinner line must be cleared on reset"
        );
    }

    /// BUG-194 rule: per-session transition slots must not leak across
    /// sessions — each context owns its own slot, and one session's
    /// in-flight state is invisible to another session's context.
    #[test]
    fn bug194_transition_slots_do_not_leak_across_sessions() {
        let mut a = SessionContext::new(SessionId::new("s-1"));
        let b = SessionContext::new(SessionId::new("s-2"));
        a.input_transition.state = InputTransitionState::Loading { elapsed_ms: 480 };
        a.input_transition.last_spinner_line = Some("⠋ Thinking...".to_string());
        assert!(
            matches!(b.input_transition.state, InputTransitionState::Idle),
            "s-2's slot must be untouched by s-1's state"
        );
        assert!(b.input_transition.last_spinner_line.is_none());
    }
}
