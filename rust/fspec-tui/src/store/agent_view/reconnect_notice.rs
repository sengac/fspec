//! RPC-416 — inline reconnect-status helpers for [`SessionContext`].
//!
//! Feature: spec/features/inline-reconnect-status-in-scrollback.feature
//!
//! Extracted from `session_context.rs` (which is near the 300-LoC
//! source-shape ceiling) so the parent stays slim, matching the
//! `chunk_processor` split pattern. The reconnect notice is a single
//! `ChunkKind::Notification` chunk pushed on `Action::Disconnected` and
//! then MUTATED IN PLACE (never re-pushed) across the
//! disconnect→reconnecting→reconnected lifecycle, keyed by the stable
//! `seq` allocated at push time so it survives focus changes and new
//! streaming chunks arriving in between.

use ratatui::style::Color;

use super::session_context::SessionContext;
use crate::views::agent::{ChunkKind, ChunkSource};

impl SessionContext {
    /// RPC-416: push an inline `Notification` line and return its stable
    /// `seq`, so the App can track `(SessionId, seq)` and later replace /
    /// remove exactly this chunk regardless of what streams in after it.
    pub fn push_notice_line<S: Into<String>>(&mut self, text: S, color: Color) -> u64 {
        let seq = self.scrollback_next_seq;
        self.push_chunk(ChunkSource {
            text: text.into(),
            color,
            kind: ChunkKind::Notification,
            is_streaming: false,
            full_text: None,
        });
        seq
    }

    /// RPC-416: replace the text + color of the chunk with stable `seq`
    /// in place (no re-push), then re-wrap that single chunk. Silent
    /// no-op when `seq` is absent (the chunk was cleared or the session
    /// changed underneath us).
    pub fn replace_notice_by_seq(&mut self, seq: u64, text: impl Into<String>, color: Color) {
        let Some(idx) = self.scrollback.chunks().iter().position(|c| c.seq == seq) else {
            return;
        };
        if let Some(chunk) = self.scrollback.chunks_mut().get_mut(idx) {
            // Invariant: a reconnect notice is always a `ChunkKind::Notification`
            // pushed by `push_notice_line`, which ALWAYS carries a `ChunkSource`.
            // The `if let Some(source)` guard below is therefore never expected
            // to be skipped in production; the `debug_assert!` makes that
            // invariant explicit so a future refactor that pushes a
            // source-less notice trips a test rather than silently no-opping.
            debug_assert!(
                chunk.source.is_some(),
                "reconnect notice chunk (seq {seq}) must carry a ChunkSource"
            );
            if let Some(source) = chunk.source.as_mut() {
                source.text = text.into();
                source.color = color;
            }
        }
        self.scrollback.rewrap_at(idx);
    }

    /// RPC-416: remove the chunk with stable `seq` from scrollback,
    /// re-anchoring the in-flight assistant / thinking slots and the
    /// SELECT-mode selection just like the `chunk_processor` removals.
    /// Silent no-op when `seq` is absent.
    pub fn remove_notice_by_seq(&mut self, seq: u64) {
        let Some(idx) = self.scrollback.chunks().iter().position(|c| c.seq == seq) else {
            return;
        };
        self.scrollback.chunks_mut().remove(idx);
        // Re-anchor the in-flight slot indices: anything AFTER the
        // removed chunk shifts left by one; the removed slot itself is
        // cleared (a reconnect notice is never an in-flight chunk, but
        // guard anyway so the invariant is airtight).
        self.in_flight_assistant = reanchor_slot(self.in_flight_assistant, idx);
        self.in_flight_thinking = reanchor_slot(self.in_flight_thinking, idx);
        // Re-pin the SELECT-mode selection from its remembered seq (the
        // selection is cleared automatically if that turn is gone).
        self.scrollback.reanchor_after_removal();
    }
}

/// Shift an optional chunk-slot index left by one when a chunk BEFORE it
/// was removed; clear it when the removed chunk WAS the slot.
fn reanchor_slot(slot: Option<usize>, removed_idx: usize) -> Option<usize> {
    match slot {
        Some(i) if i == removed_idx => None,
        Some(i) if i > removed_idx => Some(i - 1),
        other => other,
    }
}
