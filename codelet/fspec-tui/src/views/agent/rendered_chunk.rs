//! Pre-rendered chunk + source representation used by `ScrollbackList`.
//!
//! Extracted from `views/agent.rs` to keep the orchestrator under the
//! 300-LoC ceiling pinned by `spec/features/rpc026-source-shape.feature`.
//!
//! Feature: spec/features/agentview-chunk-rendering-parity.feature
//!          spec/features/agentview-chunkprocessor-parity.feature
//!          spec/features/agentview-scrollback-wrap.feature

use ratatui::style::Color;
use ratatui::text::Line;

/// Tag carried by [`ChunkSource`] so the renderer can apply the
/// per-variant prefix (e.g. `'● '` for AssistantText / ToolCall) on
/// `lineIndex == 0` only — mirroring `src/tui/utils/conversationUtils.ts:64-71`.
///
/// **RPC-091**: replaces the previous "prefix baked into `text`"
/// approach which made accumulation impossible and produced one
/// bulleted row per streamed delta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChunkKind {
    /// Green `"You: "` prefix line.
    UserInput,
    /// White `"● "` prefix on lineIndex==0; subsequent lines unprefixed.
    AssistantText,
    /// Yellow `"[Thinking]\n…"` block; no `"● "` prefix.
    Thinking,
    /// `"● {ToolName}({argsDisplay})"` header, optional body attached
    /// from a matching `ToolResult` / `ToolProgress`.
    ToolCall {
        /// Stable id matching `ToolCallInfo.id` / `ToolResultInfo.tool_call_id`.
        tool_call_id: String,
        /// Mirrors the TS `isError` flag set by `ToolResult`. Controls
        /// the body's foreground colour at render time.
        is_error: bool,
    },
    /// `"API Error: …"` white status line.
    Error,
    /// `"⚠ Interrupted"` white status line.
    Interrupted,
    /// `UserNotification` body, rendered verbatim.
    Notification,
    /// `IncomingMessage` parsed into `"[W] {role}> {body}"` magenta line.
    Incoming,
}

/// Original chunk source used to re-derive `RenderedChunk::lines`
/// when the viewport width changes.
///
/// Stored alongside the cached `lines` so resizing the terminal does
/// NOT permanently truncate a wrapped chunk — the scrollback widget
/// re-wraps from `text` against the new width.
#[derive(Debug, Clone)]
pub struct ChunkSource {
    /// The full body to wrap. **RPC-091**: per-variant prefixes
    /// (`"● "`, `"You: "`) are NO LONGER baked into `text` for
    /// AssistantText / UserInput — they are applied by the renderer on
    /// `lineIndex == 0` so streaming Text deltas can be accumulated
    /// into a single in-flight chunk without re-bulleting on every
    /// append. Hard breaks (`\n`) are preserved as separate visual
    /// paragraphs.
    pub text: String,
    /// Foreground colour applied to every span produced from `text`.
    pub color: Color,
    /// Variant tag used by the renderer to choose a prefix / colour
    /// rule. **RPC-091** addition.
    pub kind: ChunkKind,
    /// True while the chunk is still accumulating deltas (assistant
    /// bubble being streamed, or fresh placeholder after a ToolResult).
    /// Cleared on `Done` / `Error` / `Interrupted` / next `ToolCall`.
    /// **RPC-091** addition — mirrors TS
    /// `ConversationMessage.isStreaming`.
    pub is_streaming: bool,
}

/// Pre-rendered chunk row keyed by chunk seq.
///
/// `lines` is a cache derived from `source.text` wrapped to the
/// scrollback widget's most-recent viewport width. When `source` is
/// `Some`, the scrollback widget re-wraps on width change (RPC-078).
/// When `source` is `None`, `lines` is treated as opaque pre-rendered
/// content — test fixtures and legacy push-paths that don't carry a
/// wrappable body use this mode.
#[derive(Debug, Clone)]
pub struct RenderedChunk {
    pub seq: u64,
    pub lines: Vec<Line<'static>>,
    /// Optional source for re-wrap on viewport resize. `None` for
    /// pre-RPC-078 callers that pushed already-styled `Line`s directly.
    pub source: Option<ChunkSource>,
}
