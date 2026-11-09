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

use super::text_wrap::RowEnd;

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
    /// **TUI-115**: green user line submitted while the session was busy;
    /// the tag picks its `"You (steer): "` / `"Queued #n: "` prefix.
    TaggedUserInput(UserInputTag),
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
        /// **RPC-391/393**: true when the body is an Edit/Write diff stored
        /// as the canonical codec string (`diff_format::to_line`). The wrap
        /// layer parses each fragment back to a typed `DiffDisplayRow` and
        /// styles it via the shared `style_row`, AND bypasses the RPC-389
        /// 8-line collapse (the diff is already self-collapsed at 25 by
        /// `build_diff_rows`).
        is_diff: bool,
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

/// TUI-115: how a busy user line was submitted, shown as its prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserInputTag {
    /// Steered into a running turn: `"You (steer): "`.
    Steer,
    /// Queued behind a busy turn as the session's n-th queued input.
    Queued(u32),
}

impl UserInputTag {
    /// The prefix painted before the user's text.
    pub fn prefix(self) -> String {
        match self {
            Self::Steer => "You (steer): ".to_string(),
            Self::Queued(n) => format!("Queued #{n}: "),
        }
    }
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
    /// **RPC-391**: for a diff tool card, the FULL (uncollapsed) marker
    /// string shown by the `TurnContentModal`. `text` holds the inline
    /// 25-line-collapsed body; `full_text` holds every diff line. `None`
    /// for all non-diff chunks (the modal falls back to `text`).
    pub full_text: Option<String>,
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
    /// TUI-130: how each row of `lines` ends in copied text, index for
    /// index, set with `lines` by every wrap of `source`. Opaque
    /// pre-rendered rows (`source: None`) carry none: each of them is a
    /// line of its own.
    pub row_ends: Vec<RowEnd>,
}

impl RenderedChunk {
    /// TUI-130: how row `index` ends in copied text.
    pub fn row_end(&self, index: usize) -> RowEnd {
        self.row_ends.get(index).copied().unwrap_or(RowEnd::Break)
    }
}
