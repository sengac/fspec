use super::super::completion::CompletionModel;
use super::{
    StreamingCompletionResponse, StreamingEvent, ThinkingState, ToolCallState, events::handle_event,
};
use crate::completion::{CompletionError, CompletionRequest, GetTokenUsage};
use crate::http_client::sse::{Event, GenericEventSource};
use crate::http_client::{self, HttpClientExt};
use crate::streaming::{self, RawStreamingChoice, StreamingResult};
use crate::telemetry::SpanCombinator;
use async_stream::stream;
use futures::StreamExt;
use tracing::{Level, enabled, info_span};
use tracing_futures::Instrument;

impl<T> CompletionModel<T>
where
    T: HttpClientExt + Clone + Default + 'static,
{
    pub(crate) async fn stream(
        &self,
        completion_request: CompletionRequest,
    ) -> Result<streaming::StreamingCompletionResponse<StreamingCompletionResponse>, CompletionError>
    {
        let span = if tracing::Span::current().is_disabled() {
            info_span!(
                target: "rig::completions",
                "chat_streaming",
                gen_ai.operation.name = "chat_streaming",
                gen_ai.provider.name = "anthropic",
                gen_ai.request.model = self.model,
                gen_ai.system_instructions = &completion_request.preamble,
                gen_ai.response.id = tracing::field::Empty,
                gen_ai.response.model = self.model,
                gen_ai.usage.output_tokens = tracing::field::Empty,
                gen_ai.usage.input_tokens = tracing::field::Empty,
                gen_ai.input.messages = tracing::field::Empty,
                gen_ai.output.messages = tracing::field::Empty,
            )
        } else {
            tracing::Span::current()
        };
        let body = self.streaming_request_bytes(completion_request)?;
        if enabled!(Level::TRACE) {
            tracing::trace!(
                target: "rig::completions",
                "Anthropic completion request: {}",
                String::from_utf8_lossy(&body)
            );
        }
        let req = self
            .client
            .post("/v1/messages")?
            .body(body)
            .map_err(http_client::Error::Protocol)?;
        let stream = GenericEventSource::new(self.client.clone(), req);
        // Use our SSE decoder to directly handle Server-Sent Events format
        let stream: StreamingResult<StreamingCompletionResponse> = Box::pin(stream! {
            let mut current_tool_call: Option<ToolCallState> = None;
            let mut current_thinking: Option<ThinkingState> = None;
            let mut sse_stream = Box::pin(stream);
            let mut request_usage = super::super::request_usage::RequestUsage::default();
            let mut final_usage = None;
            let mut staged_calls = Vec::new();
            let mut text_content = String::new();
            // PROV-039: Capture stop_reason from MessageDelta for propagation
            let mut captured_stop_reason: Option<String> = None;
            while let Some(sse_result) = sse_stream.next().await {
                // PROV-009-DEBUG: Log every SSE result at debug level
                tracing::debug!(
                    "[anthropic/streaming] SSE_RESULT received: is_ok={}, variant={:?}",
                    sse_result.is_ok(),
                    match &sse_result {
                        Ok(Event::Open) => "Open",
                        Ok(Event::Message(_)) => "Message",
                        Err(_) => "Error",
                    }
                );
                match sse_result {
                    Ok(Event::Open) => {
                        tracing::debug!("[anthropic/streaming] SSE Event::Open received");
                    }
                    Ok(Event::Message(sse)) => {
                        // PROV-009-DEBUG: Log raw SSE data before parsing
                        tracing::debug!(
                            "[anthropic/streaming] SSE Message raw data (first 500 chars): {}",
                            sse.data.chars().take(500).collect::<String>()
                        );
                        // Parse the SSE data as a StreamingEvent
                        match serde_json::from_str::<StreamingEvent>(&sse.data) {
                            Ok(event) => {
                                match &event {
                                    StreamingEvent::MessageStart { message } => {
                                        let span = tracing::Span::current();
                                        span.record("gen_ai.response.id", &message.id);
                                        span.record("gen_ai.response.model_name", &message.model);
                                        // Emit Usage event for MessageStart (output_tokens=0 signals new API call)
                                        // This enables real-time token tracking during streaming
                                        if let Some(usage) = request_usage.start(&message.usage) {
                                            yield Ok(RawStreamingChoice::Usage(usage));
                                        }
                                    },
                                    StreamingEvent::MessageDelta { delta, usage } => {
                                        // Emit Usage event for MessageDelta (streaming token updates)
                                        let usage = match request_usage.merge(usage) {
                                            Ok(usage) => usage,
                                            Err(error) => { yield Err(error); return; }
                                        };
                                        if let Some(normalized) = usage.token_usage() {
                                            yield Ok(RawStreamingChoice::Usage(normalized));
                                        }
                                        // PROV-005-DEBUG: Log stop_reason for debugging Opus 4.6 compaction
                                        if let Some(ref reason) = delta.stop_reason {
                                            tracing::debug!(
                                                "[anthropic/streaming] MessageDelta stop_reason={:?}, stop_sequence={:?}",
                                                reason,
                                                delta.stop_sequence
                                            );
                                            // PROV-009-DEBUG: Check if this is a compaction stop reason
                                            if reason == "compaction" {
                                                tracing::warn!(
                                                    "[anthropic/streaming] !!! SERVER-SIDE COMPACTION stop_reason DETECTED !!!"
                                                );
                                            }
                                        }
                                        if delta.stop_reason.is_some() {
                                            // PROV-039: Capture stop_reason before breaking
                                            captured_stop_reason = delta.stop_reason.clone();
                                            let span = tracing::Span::current();
                                            span.record_token_usage(&usage);
                                            final_usage = Some(usage);
                                            break;
                                        }
                                    }
                                    StreamingEvent::Unknown => {
                                        // PROV-005-DEBUG: Log unknown event type
                                        tracing::warn!(body_bytes = sse.data.len(), body = %sse.data, "[anthropic/streaming] Unknown StreamingEvent received");
                                    }
                                    _ => {}
                                }
                                if let Some(result) = handle_event(&event, &mut current_tool_call, &mut current_thinking) {
                                    match result {
                                        Ok(RawStreamingChoice::ToolCall(call)) => staged_calls.push(call),
                                        Ok(choice) => {
                                            if let RawStreamingChoice::Message(ref text) = choice {
                                                text_content += text;
                                            }
                                            yield Ok(choice);
                                        }
                                        Err(error) => { yield Err(error); return; }
                                    }
                                }
                            },
                            Err(error) => {
                                if !sse.data.trim().is_empty() {
                                    // PROV-005-DEBUG: Log JSON parsing failures with full data
                                    tracing::warn!(error = %error, body_bytes = sse.data.len(), body = %sse.data, "[anthropic/streaming] JSON parse error");
                                    yield Err(CompletionError::ResponseError(
                                        format!("Failed to parse JSON: {} ({} bytes)", error, sse.data.len())
                                    ));
                                    return;
                                }
                            }
                        }
                    },
                    Err(error) => {
                        // PROV-009-DEBUG: Log SSE errors at the lowest level
                        tracing::warn!("[anthropic/streaming] SSE ERROR received: {}", error);
                        sse_stream.close();
                        yield Err(CompletionError::HttpError(error));
                        return;
                    }
                }
            }
            // Ensure event source is closed when stream ends
            sse_stream.close();
            let Some(usage) = final_usage else {
                yield Err(CompletionError::ResponseError("incomplete Anthropic usage: missing terminal delta".into()));
                return;
            };
            yield Ok(RawStreamingChoice::FinalResponse(StreamingCompletionResponse {
                usage,
                stop_reason: captured_stop_reason.clone(),
            }));
            // PROV-039: After loop exit, if stop_reason is max_tokens and there's a pending
            // tool call that was never closed (ContentBlockStop never sent), emit an enriched
            // truncation error. This handles the case where Anthropic ends the stream mid-tool-call.
            if captured_stop_reason.as_deref() == Some("max_tokens")
                && let Some(tool_call) = Option::take(&mut current_tool_call)
            {
                yield Err(CompletionError::ResponseError(
                    format!(
                        "Tool call truncated due to output token limit. \
                         Tool '{}' received incomplete JSON arguments. \
                         The model hit max_tokens while generating the tool call. \
                         Partial arguments: {}",
                        tool_call.name,
                        if tool_call.input_json.is_empty() { "(empty)" } else { &tool_call.input_json }
                    )
                ));
                return;
            }
            if current_tool_call.is_some()
                || (!staged_calls.is_empty()
                    && !matches!(captured_stop_reason.as_deref(), Some("tool_use" | "end_turn" | "stop_sequence")))
            {
                yield Err(CompletionError::ResponseError("Anthropic client calls require a complete successful terminal".into()));
                return;
            }
            for call in staged_calls {
                yield Ok(RawStreamingChoice::ToolCall(call));
            }
        }.instrument(span));
        Ok(streaming::StreamingCompletionResponse::stream(stream))
    }
}
