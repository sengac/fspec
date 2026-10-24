//! `RhaiCustomProviderModel` — a `rig::completion::CompletionModel`
//! that bridges rig's completion driver to a Rhai-backed custom provider
//! (PROV-092).
//!
//! This is the keystone type that lets `rig::agent::Agent<M>` actually
//! drive a Rhai shadow provider. Earlier work (PROV-063, PROV-064,
//! PROV-067) wired the `LlmProvider` shape and standalone NAPI helpers
//! but stopped short of plugging into rig's `CompletionModel` trait.
//! Without that plug-in, calling `agent.prompt(...)` against a custom
//! provider was impossible — the `CustomProvider::create_rig_agent`
//! shim returned an opaque introspection wrapper instead of a real
//! `rig::agent::Agent`. This module closes that gap.
//!
//! ## Wiring
//!
//! - `completion()` converts the rig `CompletionRequest.chat_history`
//!   into our internal `Vec<codelet_common::Message>`, extracts an
//!   optional `thinking_config` from `additional_params`, calls
//!   [`RhaiCustomProvider::invoke_build_url`] /
//!   [`RhaiCustomProvider::invoke_build_headers`] /
//!   [`RhaiCustomProvider::invoke_build_request`] to build the wire
//!   request, posts it via the shared `reqwest` client (re-using
//!   [`super::http::post_json`]), and converts the parsed
//!   [`crate::CompletionResponse`] into a rig
//!   [`rig::completion::CompletionResponse<RhaiCustomCompletion>`].
//! - `stream()` runs the same prelude but POSTs with
//!   `Accept: text/event-stream` and streams the response body through
//!   [`super::stream_http::open_stream`] +
//!   [`super::stream::RhaiStreamProcessor`], converting each
//!   [`super::stream::StreamChunk`] into a rig
//!   [`RawStreamingChoice<RhaiCustomCompletion>`] item.
//!
//! Both paths surface non-2xx responses through
//! [`RhaiCustomProvider::invoke_map_error`] so script-defined error
//! mapping behaviour matches the standalone `complete_with_tools` API.

use std::sync::Arc;

use rig::completion::{
    CompletionError, CompletionModel, CompletionRequest,
    CompletionResponse as RigCompletionResponse,
};
use rig::message::AssistantContent;
use rig::streaming::StreamingCompletionResponse;
use rig::OneOrMany;
use serde::{Deserialize, Serialize};

use codelet_common::{ContentPart, MessageContent};
use codelet_tools::ToolDefinition;

use super::provider::RhaiCustomProvider;
use super::stream::StreamUsage;
use crate::error::ProviderError;
use crate::{CompletionResponse, StopReason};

#[path = "rig_completion.rs"]
mod rig_completion;
#[path = "rig_preparation.rs"]
mod rig_preparation;
#[path = "rig_streaming.rs"]
mod rig_streaming;
#[path = "rig_usage.rs"]
mod rig_usage;

/// Raw-response payload exposed to rig consumers.
///
/// We don't surface the full Rhai parse_response output here because it
/// is already bridged into rig's strongly-typed
/// `OneOrMany<AssistantContent>` choice list. The raw response carries
/// the stop reason (mapped from Rhai's loose enum) so downstream code
/// such as the agent-loop multi-step driver can distinguish `ToolUse`
/// from `EndTurn` without re-inspecting the choice list.
///
/// PROV-103: The raw response also carries the last-known token usage
/// snapshot so [`GetTokenUsage::token_usage`] can surface a non-empty
/// [`rig::completion::Usage`]. For streaming this is populated from the
/// final [`StreamChunk::UsageDelta`] seen before `FinalResponse`; for
/// non-streaming it is populated from the `usage` map returned by the
/// script's `parse_response`. All fields default to `0`/`None` when the
/// script does not surface usage data.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RhaiCustomCompletion {
    /// Stop reason mirroring `crate::StopReason` (lower-snake-case).
    #[serde(default)]
    pub stop_reason: String,
    /// Raw input tokens (excluding cache). `0` when unknown.
    #[serde(default)]
    pub input_tokens: u64,
    /// Output tokens for the final API segment. `0` when unknown.
    #[serde(default)]
    pub output_tokens: u64,
    /// Cache read tokens (Anthropic prompt caching).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u64>,
    /// Cache creation tokens (Anthropic prompt caching).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u64>,
    /// Reasoning / thinking tokens, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage_measured: Option<bool>,
}

impl RhaiCustomCompletion {
    fn from_snapshot(stop: StopReason, snapshot: &StreamUsage) -> Self {
        Self {
            input_tokens: snapshot.input_tokens.unwrap_or(0),
            output_tokens: snapshot.output_tokens.unwrap_or(0),
            cache_read_input_tokens: snapshot.cache_read_input_tokens,
            cache_creation_input_tokens: snapshot.cache_creation_input_tokens,
            reasoning_tokens: snapshot.reasoning_tokens,
            usage_measured: Some(
                snapshot.input_tokens.is_some() && snapshot.output_tokens.is_some(),
            ),
            ..Self::from_internal(stop)
        }
    }

    fn from_internal(stop: StopReason) -> Self {
        let s = match stop {
            StopReason::EndTurn => "end_turn",
            StopReason::ToolUse => "tool_use",
            StopReason::MaxTokens => "max_tokens",
        };
        Self {
            stop_reason: s.to_string(),
            ..Self::default()
        }
    }
}

/// rig completion model backed by [`RhaiCustomProvider`].
#[derive(Clone)]
pub struct RhaiCustomProviderModel {
    inner: Arc<RhaiCustomProvider>,
}

impl RhaiCustomProviderModel {
    /// Wrap a [`RhaiCustomProvider`].
    pub fn new(inner: Arc<RhaiCustomProvider>) -> Self {
        Self { inner }
    }

    /// Borrow the wrapped provider — useful for tests.
    pub fn provider(&self) -> &RhaiCustomProvider {
        self.inner.as_ref()
    }
}

/// Extract a `thinking_config` JSON value from rig's `additional_params`,
/// matching the Claude/Codex convention where the provider stuffs a
/// `{"thinking": {...}}` (or similar) sub-object in there.
///
/// We pass the entire `additional_params` value through to Rhai under
/// `request.thinking_config` if present. Scripts that don't need
/// thinking simply ignore the field; scripts that do (e.g. Claude
/// adaptive thinking) can read it back as-is.
fn extract_thinking_config(req: &CompletionRequest) -> Option<serde_json::Value> {
    req.additional_params.clone()
}

/// Convert our internal `CompletionResponse` shape to rig's choice list.
fn internal_response_to_rig_choice(response: &CompletionResponse) -> OneOrMany<AssistantContent> {
    let mut items: Vec<AssistantContent> = Vec::new();
    match &response.content {
        MessageContent::Text(text) => {
            items.push(AssistantContent::text(text.clone()));
        }
        MessageContent::Parts(parts) => {
            for part in parts {
                match part {
                    ContentPart::Text { text } => {
                        items.push(AssistantContent::text(text.clone()));
                    }
                    ContentPart::ToolUse { id, name, input } => {
                        items.push(AssistantContent::tool_call(
                            id.clone(),
                            name.clone(),
                            input.clone(),
                        ));
                    }
                    ContentPart::ToolResult { .. } | ContentPart::Image { .. } => {
                        // Tool results never appear in assistant
                        // responses; assistant images aren't in scope.
                    }
                }
            }
        }
    }
    if items.is_empty() {
        // Empty-item fallback must still produce a valid OneOrMany; use
        // a single empty-text element rather than calling `many()` on an
        // empty Vec (which would panic).
        return OneOrMany::one(AssistantContent::text(String::new()));
    }
    // SAFETY: non-empty branch — `many` only fails on empty Vec, which
    // we just ruled out above.
    match OneOrMany::many(items) {
        Ok(many) => many,
        Err(_) => OneOrMany::one(AssistantContent::text(String::new())),
    }
}

/// Convert the rig `CompletionRequest.tools` (`rig::completion::ToolDefinition`
/// with `{name, description, parameters}`) into our internal
/// [`codelet_tools::ToolDefinition`] (`{name, description, input_schema}`)
/// so the Rhai `build_request` script sees the tool catalogue.
///
/// rig's `Agent::prompt` gathers tool definitions from its
/// `ToolServerHandle` and attaches them to `CompletionRequest.tools` —
/// previously this bridge dropped them on the floor, which meant
/// script-driven providers (e.g. `claude-rhai`) received an empty
/// `tools` array and therefore never advertised tool calls to the
/// upstream API.
fn tools_for_rhai(req: &CompletionRequest) -> Vec<ToolDefinition> {
    req.tools
        .iter()
        .map(|rig_tool| ToolDefinition {
            name: rig_tool.name.clone(),
            description: rig_tool.description.clone(),
            input_schema: rig_tool.parameters.clone(),
        })
        .collect()
}

impl CompletionModel for RhaiCustomProviderModel {
    type Response = RhaiCustomCompletion;
    type StreamingResponse = RhaiCustomCompletion;
    type Client = ();

    fn make(_client: &Self::Client, _model: impl Into<String>) -> Self {
        // rig's `CompletionModel::make` factory signature (`-> Self`)
        // forces an infallible constructor. We cannot support that
        // here because `RhaiCustomProviderModel` carries a fully
        // wired Rhai script handle that is only available from
        // `CustomProvider::create_rig_agent`. Callers must use the
        // agent builder path exclusively; this factory exists only
        // because the trait requires it and is unreachable via the
        // public API.
        #[allow(clippy::panic)]
        {
            panic!(
                "RhaiCustomProviderModel must be constructed via CustomProvider::create_rig_agent — \
                 rig's `make` factory is not supported because the model carries a fully \
                 wired Rhai script handle."
            );
        }
    }

    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<RigCompletionResponse<RhaiCustomCompletion>, CompletionError> {
        self.complete_request(request).await
    }

    fn prepare_request(&self, request: CompletionRequest) -> Result<Vec<u8>, CompletionError> {
        Ok(serde_json::to_vec(&self.streaming_request_body(request)?)?)
    }

    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<RhaiCustomCompletion>, CompletionError> {
        self.stream_request(request).await
    }
}

fn provider_error_to_rig(err: ProviderError) -> CompletionError {
    CompletionError::ProviderError(err.to_string())
}
