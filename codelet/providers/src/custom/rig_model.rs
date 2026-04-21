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

use async_stream::stream;
use futures::StreamExt;
use rig::completion::{
    CompletionError, CompletionModel, CompletionRequest, CompletionResponse as RigCompletionResponse,
};
use rig::message::AssistantContent;
use rig::streaming::{
    RawStreamingChoice, RawStreamingToolCall, StreamingCompletionResponse, StreamingResult,
};
use rig::OneOrMany;
use serde::{Deserialize, Serialize};

use codelet_common::{ContentPart, MessageContent};
use codelet_tools::ToolDefinition;

use super::http::post_sse;
use super::provider::RhaiCustomProvider;
use super::rig_message_convert::rig_messages_to_internal;
use super::stream::{RhaiStreamProcessor, StreamChunk};
use super::stream_http::open_stream;
use crate::error::ProviderError;
use crate::{CompletionResponse, StopReason};

/// Raw-response payload exposed to rig consumers.
///
/// We don't surface the full Rhai parse_response output here because it
/// is already bridged into rig's strongly-typed
/// `OneOrMany<AssistantContent>` choice list. The raw response carries
/// the stop reason (mapped from Rhai's loose enum) so downstream code
/// such as the agent-loop multi-step driver can distinguish `ToolUse`
/// from `EndTurn` without re-inspecting the choice list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RhaiCustomCompletion {
    /// Stop reason mirroring `crate::StopReason` (lower-snake-case).
    #[serde(default)]
    pub stop_reason: String,
}

impl RhaiCustomCompletion {
    fn from_internal(stop: StopReason) -> Self {
        let s = match stop {
            StopReason::EndTurn => "end_turn",
            StopReason::ToolUse => "tool_use",
            StopReason::MaxTokens => "max_tokens",
        };
        Self {
            stop_reason: s.to_string(),
        }
    }
}

impl rig::completion::GetTokenUsage for RhaiCustomCompletion {
    fn token_usage(&self) -> Option<rig::completion::Usage> {
        None
    }

    fn stop_reason(&self) -> Option<&str> {
        Some(self.stop_reason.as_str())
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
fn internal_response_to_rig_choice(
    response: &CompletionResponse,
) -> OneOrMany<AssistantContent> {
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
        panic!(
            "RhaiCustomProviderModel must be constructed via CustomProvider::create_rig_agent — \
             rig's `make` factory is not supported because the model carries a fully \
             wired Rhai script handle."
        );
    }

    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<RigCompletionResponse<RhaiCustomCompletion>, CompletionError> {
        let preamble = request.preamble.as_deref();
        let history: Vec<rig::completion::Message> = request.chat_history.clone().into_iter().collect();
        let messages = rig_messages_to_internal(preamble, &history);
        let tools = tools_for_rhai(&request);
        let thinking = extract_thinking_config(&request);

        let provider = self.inner.as_ref();
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        let thinking_str = thinking
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "none".to_string());
        tracing::warn!(
            provider = %provider.config_name(),
            history_len = history.len(),
            internal_msg_count = messages.len(),
            tool_count = tools.len(),
            tool_names = ?tool_names,
            has_preamble = preamble.is_some(),
            has_thinking = thinking.is_some(),
            "[rhai-dispatch] CompletionModel::completion ENTER tool_count={tc} tool_names={tns:?} thinking_config={th}",
            tc = tools.len(),
            tns = tool_names,
            th = thinking_str,
        );
        let url = provider
            .invoke_build_url()
            .await
            .map_err(provider_error_to_rig)?;
        let headers = provider
            .invoke_build_headers()
            .await
            .map_err(provider_error_to_rig)?;
        let body = provider
            .invoke_build_request(&messages, &tools, thinking)
            .await
            .map_err(provider_error_to_rig)?;

        let (status, body_text) = super::http::post_json(
            provider.http_client_handle(),
            provider.config_name(),
            &url,
            headers,
            &body,
        )
        .await
        .map_err(provider_error_to_rig)?;

        tracing::warn!(
            provider = %provider.config_name(),
            status = status,
            body_len = body_text.len(),
            "[rhai-dispatch] CompletionModel::completion received HTTP response"
        );

        if !(200..300).contains(&status) {
            let err = provider.invoke_map_error(status, &body_text).await;
            tracing::warn!(
                provider = %provider.config_name(),
                status = status,
                "[rhai-dispatch] CompletionModel::completion non-2xx → returning mapped error"
            );
            return Err(provider_error_to_rig(err));
        }

        let body_json: serde_json::Value = serde_json::from_str(&body_text).map_err(|e| {
            tracing::warn!(
                provider = %provider.config_name(),
                error = %e,
                "[rhai-dispatch] CompletionModel::completion response body is not JSON"
            );
            CompletionError::ResponseError(format!("custom provider response was not JSON: {e}"))
        })?;

        let internal = provider
            .invoke_parse_response(&body_json)
            .await
            .map_err(provider_error_to_rig)?;

        let choice = internal_response_to_rig_choice(&internal);
        let raw = RhaiCustomCompletion::from_internal(internal.stop_reason);

        tracing::warn!(
            provider = %provider.config_name(),
            choice_len = choice.iter().count(),
            stop_reason = %raw.stop_reason,
            "[rhai-dispatch] CompletionModel::completion EXIT ok"
        );

        Ok(RigCompletionResponse {
            choice,
            usage: rig::completion::Usage::new(),
            raw_response: raw,
        })
    }

    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<RhaiCustomCompletion>, CompletionError> {
        let preamble = request.preamble.as_deref();
        let history: Vec<rig::completion::Message> = request.chat_history.clone().into_iter().collect();
        let messages = rig_messages_to_internal(preamble, &history);
        let tools = tools_for_rhai(&request);
        let thinking = extract_thinking_config(&request);

        let provider = self.inner.clone();
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        let thinking_str = thinking
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "none".to_string());
        tracing::warn!(
            provider = %provider.config_name(),
            history_len = history.len(),
            internal_msg_count = messages.len(),
            tool_count = tools.len(),
            tool_names = ?tool_names,
            has_preamble = preamble.is_some(),
            has_thinking = thinking.is_some(),
            "[rhai-dispatch] CompletionModel::stream ENTER tool_count={tc} tool_names={tns:?} thinking_config={th}",
            tc = tools.len(),
            tns = tool_names,
            th = thinking_str,
        );
        let url = provider
            .invoke_build_url()
            .await
            .map_err(provider_error_to_rig)?;
        let headers = provider
            .invoke_build_headers()
            .await
            .map_err(provider_error_to_rig)?;
        let body = provider
            .invoke_build_stream_request(&messages, &tools, thinking)
            .await
            .map_err(provider_error_to_rig)?;

        let (status, byte_stream) = post_sse(
            provider.http_client_handle(),
            provider.config_name(),
            &url,
            headers,
            &body,
        )
        .await
        .map_err(provider_error_to_rig)?;

        tracing::warn!(
            provider = %provider.config_name(),
            status = status,
            "[rhai-dispatch] CompletionModel::stream opened SSE response"
        );

        let processor = RhaiStreamProcessor::new(
            provider.engine_handle(),
            provider.ast_handle(),
            provider.config_name().to_string(),
            provider.config_dynamic_accessor(),
        );

        let provider_for_err = provider.clone();
        let chunk_stream = open_stream(processor, status, byte_stream, move |st, body_text| {
            let p = provider_for_err.clone();
            async move { p.invoke_map_error(st, &body_text).await }
        });

        let provider_name_for_stream = provider.config_name().to_string();
        let rig_stream: StreamingResult<RhaiCustomCompletion> = Box::pin(stream! {
            let provider_name = provider_name_for_stream;
            let mut chunk_stream = chunk_stream;
            let mut final_stop = StopReason::EndTurn;
            let mut text_chunks = 0usize;
            let mut reasoning_chunks = 0usize;
            let mut tool_chunks = 0usize;
            while let Some(item) = chunk_stream.next().await {
                match item {
                    Ok(StreamChunk::TextDelta(text)) => {
                        text_chunks += 1;
                        tracing::warn!(
                            provider = %provider_name,
                            chunk_index = text_chunks,
                            text_len = text.len(),
                            "[rhai-dispatch] stream yielding TextDelta"
                        );
                        yield Ok(RawStreamingChoice::Message(text));
                    }
                    Ok(StreamChunk::ReasoningDelta(text)) => {
                        reasoning_chunks += 1;
                        tracing::warn!(
                            provider = %provider_name,
                            chunk_index = reasoning_chunks,
                            text_len = text.len(),
                            "[rhai-dispatch] stream yielding ReasoningDelta"
                        );
                        yield Ok(RawStreamingChoice::ReasoningDelta {
                            id: None,
                            reasoning: text,
                        });
                    }
                    Ok(StreamChunk::ToolCallStart { .. }) => {
                        // We surface tool calls as a single
                        // RawStreamingChoice::ToolCall when the
                        // accumulator flushes — the start signal alone
                        // is not actionable for rig's stream consumer.
                    }
                    Ok(StreamChunk::ToolCallArgsDelta { id, chunk }) => {
                        yield Ok(RawStreamingChoice::ToolCallDelta {
                            id,
                            content: rig::streaming::ToolCallDeltaContent::Delta(chunk),
                        });
                    }
                    Ok(StreamChunk::ToolCallComplete { id, name, input }) => {
                        tool_chunks += 1;
                        // Keep structured fields for filterable log
                        // scraping AND inline the tool_name in the
                        // message so the NAPI log bridge (which strips
                        // structured fields when forwarding to the TS
                        // `fspec.log` sink) still surfaces the tool
                        // name for downstream tests / diagnostics.
                        tracing::warn!(
                            provider = %provider_name,
                            tool_id = %id,
                            tool_name = %name,
                            "[rhai-dispatch] stream yielding ToolCallComplete tool_name={name} tool_id={id}"
                        );
                        let mut tc = RawStreamingToolCall::new(id, name, input);
                        tc = tc.with_signature(None);
                        yield Ok(RawStreamingChoice::ToolCall(tc));
                    }
                    Ok(StreamChunk::StopReason(reason)) => {
                        tracing::warn!(
                            provider = %provider_name,
                            stop_reason = ?reason,
                            "[rhai-dispatch] stream received StopReason"
                        );
                        final_stop = reason;
                    }
                    Err(e) => {
                        tracing::warn!(
                            provider = %provider_name,
                            error = %e,
                            "[rhai-dispatch] stream received ERROR, terminating"
                        );
                        yield Err(provider_error_to_rig(e));
                        return;
                    }
                }
            }
            let raw = RhaiCustomCompletion::from_internal(final_stop);
            tracing::warn!(
                provider = %provider_name,
                stop_reason = %raw.stop_reason,
                text_chunks = text_chunks,
                reasoning_chunks = reasoning_chunks,
                tool_chunks = tool_chunks,
                "[rhai-dispatch] stream EXIT with FinalResponse"
            );
            yield Ok(RawStreamingChoice::FinalResponse(raw));
        });

        Ok(StreamingCompletionResponse::stream(rig_stream))
    }
}

fn provider_error_to_rig(err: ProviderError) -> CompletionError {
    CompletionError::ProviderError(err.to_string())
}
