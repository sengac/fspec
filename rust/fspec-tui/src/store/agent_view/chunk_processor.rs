//! Port of `src/tui/utils/chunkProcessor.ts` — the streaming-chunk
//! accumulation algorithm used by `SessionContext::record_chunk`.
//!
//! Feature: spec/features/agentview-chunkprocessor-parity.feature
//!
//! Extracted from `session_context.rs` so the parent stays under the
//! 300-LoC ceiling pinned by `rpc024-source-shape.feature`.

use ratatui::style::Color;

use super::markdown_tables::format_markdown_tables;
use super::session_context::SessionContext;
use crate::views::agent::{ChunkKind, ChunkSource};

/// Mirrors `processStreamingChunk` Text branch
/// (`chunkProcessor.ts:444-461`).
pub fn append_assistant_text(ctx: &mut SessionContext, text: &str) {
    if let Some(idx) = ctx.in_flight_assistant {
        if let Some(chunk) = ctx.scrollback.chunks_mut().get_mut(idx) {
            if let Some(source) = chunk.source.as_mut() {
                source.text.push_str(text);
            }
        }
        ctx.scrollback.rewrap_at(idx);
        return;
    }
    let source = ChunkSource {
        text: text.to_string(),
        color: Color::White,
        kind: ChunkKind::AssistantText,
        is_streaming: true,
        full_text: None,
    };
    let new_idx = ctx.scrollback.chunk_count();
    ctx.push_source(source);
    ctx.in_flight_assistant = Some(new_idx);
}

/// Mirrors `processStreamingChunk` Thinking branch
/// (`chunkProcessor.ts:463-466`) → delegates to
/// `appendThinking` (`thinkingBlockManager.ts:139-181`).
///
/// **RPC-093**.
///
/// - If `in_flight_thinking.is_some()`: append `delta` to the
///   existing chunk's `source.text` and re-wrap that one chunk.
/// - Else if `in_flight_assistant.is_some()`: splice a new
///   `Thinking` chunk BEFORE the assistant chunk (parity with TS
///   `splice(streamingIdx, 0, newThinking)`); bump
///   `in_flight_assistant` by 1; set `in_flight_thinking` to the
///   spliced index.
/// - Else: push a new `Thinking` chunk at the tail; set
///   `in_flight_thinking`.
pub fn append_thinking(ctx: &mut SessionContext, delta: &str) {
    if delta.is_empty() {
        return;
    }

    if let Some(idx) = ctx.in_flight_thinking {
        if let Some(chunk) = ctx.scrollback.chunks_mut().get_mut(idx) {
            if let Some(source) = chunk.source.as_mut() {
                source.text.push_str(delta);
            }
        }
        ctx.scrollback.rewrap_at(idx);
        return;
    }

    let source = ChunkSource {
        text: delta.to_string(),
        color: Color::Yellow,
        kind: ChunkKind::Thinking,
        is_streaming: true,
        full_text: None,
    };

    if let Some(assist_idx) = ctx.in_flight_assistant {
        ctx.insert_source_at(assist_idx, source);
        ctx.in_flight_thinking = Some(assist_idx);
        ctx.in_flight_assistant = Some(assist_idx + 1);
    } else {
        let new_idx = ctx.scrollback.chunk_count();
        ctx.push_source(source);
        ctx.in_flight_thinking = Some(new_idx);
    }
}

/// Mirrors `finalizeThinkingBlock`
/// (`thinkingBlockManager.ts:194-207`). Called ONLY by
/// `handle_tool_call` (the only explicit finalize call site in
/// `chunkProcessor.ts:469`). Sets `is_streaming=false` on the
/// in-flight thinking chunk and clears the slot.
///
/// **RPC-093**.
pub fn finalize_in_flight_thinking(ctx: &mut SessionContext) {
    if let Some(idx) = ctx.in_flight_thinking.take() {
        if let Some(chunk) = ctx.scrollback.chunks_mut().get_mut(idx) {
            if let Some(source) = chunk.source.as_mut() {
                source.is_streaming = false;
            }
        }
        ctx.scrollback.rewrap_at(idx);
    }
}


/// Mirrors `processStreamingChunk` Done branch
/// (`chunkProcessor.ts:538-558`).
///
/// RPC-432: the markdown-table formatting moved into
/// [`flush_in_flight_drop_empty`] (shared by every flush trigger), so
/// Done is just the shared flush plus the thinking-slot clear.
pub fn handle_done(ctx: &mut SessionContext) {
    flush_in_flight_drop_empty(ctx);
    // RPC-093: Done is a turn boundary. Slot-only clear — the
    // existing thinking chunk is left untouched (still visible,
    // still is_streaming=true on the chunk) so it remains as the
    // final thought of the completed turn. The next Thinking delta
    // in a new turn finds no in-flight slot and starts fresh.
    ctx.in_flight_thinking = None;
}

/// Mirrors `processStreamingChunk` Error branch
/// (`chunkProcessor.ts:560-580`).
pub fn handle_error(ctx: &mut SessionContext, error: &str) {
    flush_in_flight_drop_empty(ctx);
    // RPC-093: Error is a flush trigger. Slot-only clear; the
    // existing thinking chunk is left untouched.
    ctx.in_flight_thinking = None;
    ctx.push_chunk(ChunkSource {
        text: format!("API Error: {error}"),
        color: Color::White,
        kind: ChunkKind::Error,
        is_streaming: false,
        full_text: None,
    });
}

/// Drop trailing empty in-flight placeholder; finalise non-empty
/// in-flight (clear `is_streaming`); clear the `in_flight_assistant`
/// slot either way. Used by flush triggers (UserInput, ToolCall,
/// Error, Interrupted).
pub fn flush_in_flight_drop_empty(ctx: &mut SessionContext) {
    if let Some(idx) = ctx.in_flight_assistant.take() {
        let is_empty = ctx
            .scrollback
            .chunks()
            .get(idx)
            .and_then(|c| c.source.as_ref())
            .map(|s| s.text.is_empty())
            .unwrap_or(true);
        if is_empty {
            ctx.scrollback.chunks_mut().remove(idx);
        } else if let Some(chunk) = ctx.scrollback.chunks_mut().get_mut(idx) {
            if let Some(source) = chunk.source.as_mut() {
                // RPC-432: format at every assistant-message finalization,
                // not only at turn Done. Every flush trigger (ToolCall,
                // Error, Interrupted, UserInput, Done) routes through here,
                // so tables in intermediate messages render as grids too.
                // Idempotent — grids contain no pipe+dash-separator rows.
                source.text = format_markdown_tables(&source.text);
                source.is_streaming = false;
            }
            ctx.scrollback.rewrap_at(idx);
        }
    }
}
