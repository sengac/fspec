//! Port of the ToolCall / ToolResult / ToolResult-adjacent branches of
//! `src/tui/utils/chunkProcessor.ts` — the streaming tool-card handlers
//! used by `SessionContext::record_chunk`.
//!
//! Feature: spec/features/agentview-chunkprocessor-parity.feature
//!
//! Extracted from `chunk_processor.rs` (which pins the accumulation
//! algorithm) to keep both files under the 300-LoC ceiling.

use codelet_rpc_types::{ToolCallInfo, ToolProgressInfo, ToolResultInfo};
use ratatui::style::Color;

use super::chunk_processor::{finalize_in_flight_thinking, flush_in_flight_drop_empty};
use super::pending_tool_diff::{capture_pending_diff, produce_diff_strings};
use super::sanitize::sanitize_for_terminal;
use super::session_context::SessionContext;
use super::stderr::maybe_mark;
use super::tool_args::extract_tool_args_display;
use crate::views::agent::{ChunkKind, ChunkSource};

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
                    let body = source
                        .text
                        .split('\n')
                        .skip(1)
                        .collect::<Vec<_>>()
                        .join("\n");
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
