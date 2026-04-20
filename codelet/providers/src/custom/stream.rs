//! Streaming SSE bridge for the Rhai custom provider (PROV-064).
//!
//! The bridge parses SSE frames in Rust via `eventsource-stream` and
//! invokes the script's `parse_stream_chunk(data)` function inside
//! `tokio::task::spawn_blocking` for each event. The Rhai return value
//! — a `Map` with a `kind` field — is translated into one or more
//! [`StreamChunk`]s. Tool-call argument fragments are accumulated in
//! Rust so the script does not need to track state across events.
//!
//! Shape of the Rhai `parse_stream_chunk` return value:
//!
//! ```rhai
//! #{ kind: "text_delta",       text: "..." }
//! #{ kind: "reasoning_delta",  text: "..." }   // also accepts "thinking_delta"
//! #{ kind: "tool_call_delta",  index: 0, id: "call_1", name: "read_file", arguments: "..." }
//! #{ kind: "stop",             reason: "end_turn" | "tool_use" | "max_tokens" }
//! #{ kind: "ignore" }
//! ```
//!
//! `data: [DONE]` terminates the stream in Rust without invoking
//! `parse_stream_chunk`. Rhai runtime errors surface as a single
//! `Err(ProviderError::Api)` and terminate the stream gracefully.

use std::collections::HashMap;
use std::sync::Arc;

use rhai::{Dynamic, Engine, Scope, AST};

use super::error_mapping::map_rhai_error_to_provider;
use crate::error::ProviderError;
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
}

/// Accumulator for a single in-flight tool call.
#[derive(Debug, Default)]
pub(super) struct ToolCallAccumulator {
    /// Tool-call id from the provider. The id is the primary key; if the
    /// script only supplies an `index`, the processor synthesises an id
    /// like `idx_{index}` the first time it sees that index.
    pub(super) id: String,
    /// Tool name.
    pub(super) name: String,
    /// Concatenated `arguments` fragments.
    pub(super) arguments_json: String,
    /// Whether we've already emitted a `ToolCallStart` for this call.
    pub(super) started: bool,
}

/// Stateful per-stream processor. Holds the compiled Rhai AST, the
/// engine handle, and buffers tool-call argument fragments keyed by id.
pub struct RhaiStreamProcessor {
    engine: Arc<Engine>,
    ast: Arc<AST>,
    provider: String,
    config: Dynamic,
    /// Tool-call accumulators keyed by id. Insertion order is preserved
    /// through the companion `tool_call_order` vec so the final
    /// `ToolCallComplete` items are emitted in the order they started.
    tool_calls: HashMap<String, ToolCallAccumulator>,
    /// Preserves insertion order for deterministic flush output.
    tool_call_order: Vec<String>,
    /// Pending stop reason queued for emission after tool-call flush.
    pending_stop: Option<StopReason>,
    /// If true, `process_event` is a no-op (post-DONE / post-runtime-error).
    terminated: bool,
}

impl RhaiStreamProcessor {
    /// Construct a new processor. Arguments are shared `Arc`s so the
    /// processor is cheap to clone into spawn_blocking closures.
    pub fn new(
        engine: Arc<Engine>,
        ast: Arc<AST>,
        provider: String,
        config: Dynamic,
    ) -> Self {
        Self {
            engine,
            ast,
            provider,
            config,
            tool_calls: HashMap::new(),
            tool_call_order: Vec::new(),
            pending_stop: None,
            terminated: false,
        }
    }

    /// Mark the stream as complete (e.g. after receiving `[DONE]`).
    pub fn mark_done(&mut self) -> Vec<StreamChunk> {
        self.terminated = true;
        self.flush_pending()
    }

    /// Flush any pending accumulators at stream end. Useful as a final
    /// safety net if no explicit `stop` or `[DONE]` arrived.
    pub fn finish(&mut self) -> Vec<StreamChunk> {
        if self.terminated {
            return Vec::new();
        }
        self.terminated = true;
        self.flush_pending()
    }

    /// Convert any buffered tool calls into `ToolCallComplete` and the
    /// pending stop reason into `StopReason`.
    fn flush_pending(&mut self) -> Vec<StreamChunk> {
        let mut out = Vec::new();
        let ids = std::mem::take(&mut self.tool_call_order);
        for id in ids {
            if let Some(acc) = self.tool_calls.remove(&id) {
                let input = parse_arguments_json(&acc.arguments_json);
                out.push(StreamChunk::ToolCallComplete {
                    id: acc.id,
                    name: acc.name,
                    input,
                });
            }
        }
        self.tool_calls.clear();
        if let Some(reason) = self.pending_stop.take() {
            out.push(StreamChunk::StopReason(reason));
        }
        out
    }

    /// Process a single SSE `data` payload. Runs the Rhai script on the
    /// tokio blocking pool and converts the result into zero or more
    /// [`StreamChunk`] items.
    pub async fn process_event(
        &mut self,
        data: &str,
    ) -> Result<Vec<StreamChunk>, ProviderError> {
        tracing::warn!(
            provider = %self.provider,
            terminated = self.terminated,
            data_len = data.len(),
            data_preview = %super::log_helpers::truncate_str(data, 400),
            "[rhai-dispatch] process_event ENTER: dispatching SSE frame to parse_stream_chunk"
        );
        if self.terminated {
            return Ok(Vec::new());
        }
        let engine = Arc::clone(&self.engine);
        let ast = Arc::clone(&self.ast);
        let provider = self.provider.clone();
        let config = self.config.clone();
        let data_owned = data.to_string();

        let result = tokio::task::spawn_blocking(
            move || -> Result<Dynamic, ProviderError> {
                let mut scope = Scope::new();
                engine
                    .call_fn::<Dynamic>(
                        &mut scope,
                        &ast,
                        "parse_stream_chunk",
                        (config, data_owned),
                    )
                    .map_err(|e| {
                        map_rhai_error_to_provider(&provider, "parse_stream_chunk", &e)
                    })
            },
        )
        .await
        .map_err(|e| {
            ProviderError::api(self.provider.clone(), format!("spawn_blocking join failed: {e}"))
        })?;

        let dynamic = match result {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!(
                    provider = %self.provider,
                    error = %e,
                    "[rhai-dispatch] process_event: parse_stream_chunk returned ERROR, terminating"
                );
                self.terminated = true;
                return Err(e);
            }
        };

        let chunks = super::stream_convert::dynamic_to_chunks(self, dynamic);
        tracing::warn!(
            provider = %self.provider,
            chunk_count = chunks.len(),
            "[rhai-dispatch] process_event EXIT: parse_stream_chunk produced chunks"
        );
        Ok(chunks)
    }
}

/// Best-effort JSON parser for accumulated tool-call arguments.
///
/// If the accumulated fragments form valid JSON, the parsed value is
/// returned. If fragments are empty, an empty JSON object is returned.
/// Otherwise the raw concatenation is surfaced as a JSON string so the
/// downstream tool executor can still inspect it.
fn parse_arguments_json(raw: &str) -> serde_json::Value {
    if raw.trim().is_empty() {
        return serde_json::Value::Object(serde_json::Map::new());
    }
    match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                error = %e,
                raw = raw,
                "tool-call arguments did not parse as JSON; surfacing raw string"
            );
            serde_json::Value::String(raw.to_string())
        }
    }
}

// Accumulator mutators used by `stream_convert`. Kept `pub(super)` so
// the conversion helper in the sibling module can manipulate per-id
// buffers without re-exposing private fields to the rest of the crate.
impl RhaiStreamProcessor {
    pub(super) fn record_stop(&mut self, reason: StopReason) {
        self.pending_stop = Some(reason);
    }

    pub(super) fn tool_call_entry(
        &mut self,
        id: &str,
    ) -> &mut ToolCallAccumulator {
        if !self.tool_calls.contains_key(id) {
            self.tool_call_order.push(id.to_string());
            self.tool_calls.insert(
                id.to_string(),
                ToolCallAccumulator {
                    id: id.to_string(),
                    ..Default::default()
                },
            );
        }
        self.tool_calls
            .get_mut(id)
            .unwrap_or_else(|| unreachable!("entry inserted above"))
    }

    pub(super) fn flush_single(&mut self, key: &str) -> Option<StreamChunk> {
        self.tool_call_order.retain(|k| k != key);
        self.tool_calls.remove(key).map(|acc| {
            StreamChunk::ToolCallComplete {
                id: acc.id,
                name: acc.name,
                input: parse_arguments_json(&acc.arguments_json),
            }
        })
    }
}

pub use super::stream_http::open_stream;
