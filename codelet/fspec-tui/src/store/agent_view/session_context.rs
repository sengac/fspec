//! `SessionContext` — per-session state container introduced by RPC-024.
//!
//! Feature: spec/features/rpc024-multi-session-store.feature
//!
//! Replaces the single-slot `current_session: Option<SessionId>` field
//! on `AgentViewStore` with a `Vec<SessionContext>`. Each open AgentView
//! session keeps its own scrollback, scrollback-sequence cursor, and
//! input-draft string so cycling between sessions (Shift+←/→) preserves
//! per-session UI state.
//!
//! Lives in its own sub-module so the parent `agent_view.rs` stays
//! under the 300-LoC ceiling pinned by `rpc024-source-shape.feature`.

use codelet_rpc_types::{SessionId, StreamChunk};
use ratatui::text::{Line, Span};

use crate::views::agent::{RenderedChunk, ScrollbackList};

/// Per-session UI state held by `AgentViewStore`.
///
/// Mirrors the slice of `src/tui/store/sessionStore.ts` that survives
/// Shift+Left/Right cycling: each session keeps its own scrollback +
/// in-progress input draft, and the App task switches between them by
/// moving `AgentViewStore.current_session_index`.
#[derive(Debug)]
pub struct SessionContext {
    /// Stable session identifier (matches `SessionInfo.id` on the
    /// session manager side).
    pub id: SessionId,
    /// Optional work-unit attachment recorded when the session was
    /// created via `Action::EnterWorkUnit`. `None` for unattached
    /// sessions (e.g. created from `Action::OpenAgentView(None)`).
    pub work_unit_id: Option<String>,
    /// Windowed scrollback of pre-rendered chunks. Owned per-session
    /// so background sessions can keep accumulating output while the
    /// user is on another tab.
    pub scrollback: ScrollbackList,
    /// Monotonic seq cursor used as a tie-breaker when chunks arrive
    /// for the same session in rapid succession.
    pub scrollback_next_seq: u64,
    /// Saved MultiLineInput buffer — restored when the user switches
    /// back to this session via Shift+←/→.
    pub input_draft: String,
}

impl SessionContext {
    /// Construct a fresh context for `id` with empty scrollback and
    /// empty input draft.
    pub fn new(id: SessionId) -> Self {
        Self {
            id,
            work_unit_id: None,
            scrollback: ScrollbackList::new(),
            scrollback_next_seq: 0,
            input_draft: String::new(),
        }
    }

    /// Construct a context with an attached work-unit id.
    pub fn with_work_unit(id: SessionId, work_unit_id: Option<String>) -> Self {
        let mut ctx = Self::new(id);
        ctx.work_unit_id = work_unit_id;
        ctx
    }

    /// Append a chunk's rendered lines to this session's scrollback.
    /// Mirrors the pre-RPC-024 `AgentView::record_chunk` logic but
    /// scoped to a single SessionContext so background chunks accumulate
    /// in the right context.
    pub fn record_chunk(&mut self, chunk: &StreamChunk) {
        let seq = self.scrollback_next_seq;
        self.scrollback_next_seq = self.scrollback_next_seq.saturating_add(1);
        let lines = chunk_to_lines(chunk);
        self.scrollback.push(RenderedChunk { seq, lines });
    }

    /// Append a raw text line. Mirrors `AgentView::push_line` — used
    /// by App::dispatch's slash-command notice path.
    pub fn push_line<S: Into<String>>(&mut self, line: S) {
        let seq = self.scrollback_next_seq;
        self.scrollback_next_seq = self.scrollback_next_seq.saturating_add(1);
        self.scrollback.push(RenderedChunk {
            seq,
            lines: vec![Line::from(Span::raw(line.into()))],
        });
    }

    /// Reset scrollback + seq cursor. Called by App::dispatch on
    /// `SlashCommandSelected(Clear)` for the focused session.
    pub fn reset_scrollback(&mut self) {
        self.scrollback.reset();
        self.scrollback_next_seq = 0;
    }
}

/// Convert a `StreamChunk` into pre-rendered scrollback lines.
///
/// Lifted from the prior `AgentView::chunk_to_lines` so the per-session
/// scrollback can be filled from `App::dispatch` without bouncing
/// through the AgentView orchestrator.
fn chunk_to_lines(chunk: &StreamChunk) -> Vec<Line<'static>> {
    let body: String = match chunk {
        StreamChunk::Text { text, .. } => format!("assistant> {text}"),
        StreamChunk::Thinking { thinking, .. } => format!("(thinking) {thinking}"),
        StreamChunk::UserNotification { message, .. } => format!("[notice] {message}"),
        StreamChunk::Error { error } => format!("[error] {error}"),
        StreamChunk::Done => "[done]".to_string(),
        other => format!("{other:?}"),
    };
    vec![Line::from(Span::raw(body))]
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
    }

    #[test]
    fn record_chunk_appends_and_bumps_seq() {
        let mut ctx = SessionContext::new(SessionId::new("s-1"));
        ctx.record_chunk(&StreamChunk::text("hi".to_string()));
        ctx.record_chunk(&StreamChunk::text("there".to_string()));
        assert_eq!(ctx.scrollback.chunk_count(), 2);
        assert_eq!(ctx.scrollback_next_seq, 2);
    }

    #[test]
    fn reset_scrollback_drops_chunks_and_resets_seq() {
        let mut ctx = SessionContext::new(SessionId::new("s-1"));
        ctx.record_chunk(&StreamChunk::text("hi".to_string()));
        ctx.reset_scrollback();
        assert_eq!(ctx.scrollback.chunk_count(), 0);
        assert_eq!(ctx.scrollback_next_seq, 0);
    }
}
