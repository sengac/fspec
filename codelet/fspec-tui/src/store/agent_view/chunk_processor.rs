//! Port of `src/tui/utils/chunkProcessor.ts` — the streaming-chunk
//! accumulation algorithm used by `SessionContext::record_chunk`.
//!
//! Feature: spec/features/agentview-chunkprocessor-parity.feature
//!
//! Extracted from `session_context.rs` so the parent stays under the
//! 300-LoC ceiling pinned by `rpc024-source-shape.feature`.

use codelet_rpc_types::{ToolCallInfo, ToolProgressInfo, ToolResultInfo};
use ratatui::style::Color;

use super::markdown_tables::format_markdown_tables;
use super::pending_tool_diff::{capture_pending_diff, produce_diff_strings};
use super::sanitize::sanitize_for_terminal;
use super::session_context::SessionContext;
use super::stderr::maybe_mark;
use super::tool_args::extract_tool_args_display;
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

/// Mirrors `processStreamingChunk` ToolCall branch
/// (`chunkProcessor.ts:468-505`).
pub fn handle_tool_call(ctx: &mut SessionContext, info: &ToolCallInfo) {
    // RPC-093: finalize the in-flight thinking chunk BEFORE the
    // assistant flush + tool-card push. This is the ONLY explicit
    // `finalizeThinkingBlock` call site in chunkProcessor.ts:469.
    finalize_in_flight_thinking(ctx);
    flush_in_flight_drop_empty(ctx);
    // RPC-391: capture Edit/Write inputs for the colored diff produced on
    // the matching ToolResult. Non-diff tools / malformed input → no entry
    // (the raw tool behaviour is preserved).
    if let Some(pending) = capture_pending_diff(&info.name, &info.input) {
        ctx.pending_tool_diffs.insert(info.id.clone(), pending);
    }
    let args = extract_tool_args_display(&info.name, &info.input);
    ctx.push_chunk(ChunkSource {
        text: format!("{}({})", info.name, args),
        color: Color::White,
        kind: ChunkKind::ToolCall {
            tool_call_id: info.id.clone(),
            is_error: false,
            is_diff: false,
        },
        is_streaming: false,
        full_text: None,
    });
}

/// Mirrors `processStreamingChunk` ToolResult branch
/// (`chunkProcessor.ts:507-536`).
pub fn handle_tool_result(ctx: &mut SessionContext, info: &ToolResultInfo) {
    let target_idx =
        ctx.scrollback
            .chunks()
            .iter()
            .rposition(|c| match c.source.as_ref().map(|s| &s.kind) {
                Some(ChunkKind::ToolCall { tool_call_id, .. }) => {
                    *tool_call_id == info.tool_call_id
                }
                _ => false,
            });
    if let Some(idx) = target_idx {
        // RPC-391: if an Edit/Write diff was captured at tool-call time,
        // replace the raw body with the marker-encoded diff (collapsed
        // inline + full for the modal) and tag the card as a diff.
        let pending = ctx.pending_tool_diffs.remove(&info.tool_call_id);
        if let Some(chunk) = ctx.scrollback.chunks_mut().get_mut(idx) {
            if let Some(source) = chunk.source.as_mut() {
                if let Some(pending) = pending.as_ref() {
                    let (collapsed, full) = produce_diff_strings(pending);
                    if let Some((header, _)) = source.text.split_once('\n') {
                        source.text = format!("{header}\n{collapsed}");
                    } else {
                        source.text = format!("{}\n{collapsed}", source.text);
                    }
                    source.full_text = Some(full);
                    if let ChunkKind::ToolCall { is_diff, .. } = &mut source.kind {
                        *is_diff = true;
                    }
                } else {
                    // Only append result content if ToolProgress hasn't already
                    // streamed it into the body. ToolProgress always arrives
                    // before ToolResult (the readers stream during execution,
                    // the result is emitted after process exit). If the body
                    // is non-empty, the content is already there — skip to
                    // avoid duplication.
                    let body = source.text.split('\n').skip(1).collect::<Vec<_>>().join("\n");
                    if body.is_empty() {
                        let sanitized = sanitize_for_terminal(&info.content);
                        if !sanitized.trim().is_empty() {
                            source.text.push('\n');
                            source.text.push_str(&sanitized);
                        }
                    }
                }
                // RPC-389/RPC-399: a ToolResult settles the card — clear the
                // streaming flag so `wrap_source` switches the inline view from
                // the streaming tail window to the settled end-pinned (last-8)
                // collapse, keeping the last output lines the user was watching.
                source.is_streaming = false;
                if let ChunkKind::ToolCall { is_error, .. } = &mut source.kind {
                    *is_error = info.is_error;
                }
            }
        }
        ctx.scrollback.rewrap_at(idx);
    }
    let placeholder = ChunkSource {
        text: String::new(),
        color: Color::White,
        kind: ChunkKind::AssistantText,
        is_streaming: true,
        full_text: None,
    };
    let new_idx = ctx.scrollback.chunk_count();
    ctx.push_source(placeholder);
    ctx.in_flight_assistant = Some(new_idx);
}

/// Folds ToolProgress under the matching ToolCall card.
pub fn handle_tool_progress(ctx: &mut SessionContext, info: &ToolProgressInfo) {
    let target_idx =
        ctx.scrollback
            .chunks()
            .iter()
            .rposition(|c| match c.source.as_ref().map(|s| &s.kind) {
                Some(ChunkKind::ToolCall { tool_call_id, .. }) => {
                    *tool_call_id == info.tool_call_id
                }
                _ => false,
            });
    if let Some(idx) = target_idx {
        if let Some(chunk) = ctx.scrollback.chunks_mut().get_mut(idx) {
            if let Some(source) = chunk.source.as_mut() {
                if !source.text.ends_with('\n') {
                    source.text.push('\n');
                }
                // RPC-400: an is_stderr chunk is prefixed per line with
                // STDERR_MARKER so it renders red; is_stderr=false verbatim.
                // TUI-100: sanitize before marking to strip ANSI/control chars.
                let sanitized = sanitize_for_terminal(&info.output_chunk);
                let marked = maybe_mark(&sanitized, info.is_stderr);
                source.text.push_str(marked.trim_end_matches('\n'));
                // RPC-389: live progress keeps the card streaming (last-10
                // tail window) until a ToolResult settles it.
                source.is_streaming = true;
            }
        }
        ctx.scrollback.rewrap_at(idx);
    }
}

/// Mirrors `processStreamingChunk` Done branch
/// (`chunkProcessor.ts:538-558`).
pub fn handle_done(ctx: &mut SessionContext) {
    if let Some(idx) = ctx.in_flight_assistant {
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
                source.text = format_markdown_tables(&source.text);
                source.is_streaming = false;
            }
            ctx.scrollback.rewrap_at(idx);
        }
        ctx.in_flight_assistant = None;
    }
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
                source.is_streaming = false;
            }
            ctx.scrollback.rewrap_at(idx);
        }
    }
}
