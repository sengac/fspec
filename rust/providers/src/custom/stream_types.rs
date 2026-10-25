use crate::StopReason;

/// A single item yielded by the streaming bridge.
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// A fragment of assistant text.
    TextDelta(String),
    /// A fragment of assistant reasoning / thinking (PROV-089).
    ///
    /// Carries a chunk of the model's visible chain-of-thought output.
    /// Scripts produce this by returning `#{ kind: "reasoning_delta"
    /// | "thinking_delta", text: "..." }` from `parse_stream_chunk`.
    /// Downstream consumers are expected to surface this separately from
    /// [`StreamChunk::TextDelta`] (e.g. map it to
    /// `StreamedAssistantContent::ReasoningDelta` in rig's
    /// `MultiTurnStreamItem`).
    ReasoningDelta(String),
    /// A tool call has started — the script has identified the id and name.
    ToolCallStart {
        /// Provider-assigned tool-call id.
        id: String,
        /// Tool name.
        name: String,
    },
    /// A partial-argument fragment for a tool call in progress.
    ToolCallArgsDelta {
        /// Id of the tool call this fragment belongs to.
        id: String,
        /// The raw fragment as emitted by the provider.
        chunk: String,
    },
    /// A tool call has finished — its accumulated arguments have been
    /// parsed into `input`.
    ToolCallComplete {
        /// Provider-assigned tool-call id.
        id: String,
        /// Tool name.
        name: String,
        /// Parsed JSON arguments.
        input: serde_json::Value,
    },
    /// Stop reason for the completion.
    StopReason(StopReason),
    /// Token usage information emitted during a stream (PROV-103).
    ///
    /// Scripts surface this by returning `#{ kind: "usage", ... }` from
    /// `parse_stream_chunk`. Every numeric field is optional — `None`
    /// means "no update for this field" and the upstream aggregator
    /// should keep its previous value. Anthropic scripts emit one
    /// `UsageDelta` on `message_start` (carrying `input_tokens` and
    /// `cache_read_input_tokens` / `cache_creation_input_tokens`, with
    /// `output_tokens = Some(0)` to mark a new API segment) and another
    /// on `message_delta` (carrying the cumulative `output_tokens`).
    UsageDelta(StreamUsage),
}

/// Token usage snapshot surfaced via [`StreamChunk::UsageDelta`].
#[derive(Debug, Clone, Default)]
pub struct StreamUsage {
    /// Raw input tokens (NOT including cache). `None` → unchanged.
    pub input_tokens: Option<u64>,
    /// Cumulative output tokens for the current API segment. `None` →
    /// unchanged. `Some(0)` signals a new API segment (message_start).
    pub output_tokens: Option<u64>,
    /// Cache read input tokens. `None` → unchanged.
    pub cache_read_input_tokens: Option<u64>,
    /// Cache creation input tokens. `None` → unchanged.
    pub cache_creation_input_tokens: Option<u64>,
    /// Reasoning / thinking tokens. `None` → unchanged.
    pub reasoning_tokens: Option<u64>,
}
