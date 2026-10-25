use async_stream::stream;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tracing::{Level, enabled, info_span};
use tracing_futures::Instrument;

use super::completion::CompletionModel;
use super::completion::gemini_api_types::{ContentCandidate, FinishReason, Part, PartKind};
use crate::completion::{CompletionError, CompletionRequest, GetTokenUsage};
use crate::http_client::HttpClientExt;
use crate::http_client::sse::{Event, GenericEventSource};
use crate::streaming;
use crate::telemetry::SpanCombinator;

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PartialUsage {
    pub total_token_count: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_content_token_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidates_token_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thoughts_token_count: Option<i32>,
    pub prompt_token_count: i32,
}

impl GetTokenUsage for PartialUsage {
    fn token_usage(&self) -> Option<crate::completion::Usage> {
        super::usage::normalize(
            self.prompt_token_count,
            self.candidates_token_count,
            self.thoughts_token_count,
            self.cached_content_token_count,
            self.total_token_count,
        )
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamGenerateContentResponse {
    /// Candidate responses from the model.
    pub candidates: Vec<ContentCandidate>,
    pub model_version: Option<String>,
    pub usage_metadata: Option<PartialUsage>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StreamingCompletionResponse {
    pub usage_metadata: PartialUsage,
    /// PROV-039: stop_reason captured from the streaming finish_reason
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
}

impl GetTokenUsage for StreamingCompletionResponse {
    fn token_usage(&self) -> Option<crate::completion::Usage> {
        self.usage_metadata.token_usage()
    }
    /// PROV-039: Return the captured stop_reason (mapped from Gemini finish_reason)
    fn stop_reason(&self) -> Option<&str> {
        self.stop_reason.as_deref()
    }
}

impl<T> CompletionModel<T>
where
    T: HttpClientExt + Clone + 'static,
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
                gen_ai.provider.name = "gcp.gemini",
                gen_ai.request.model = self.model,
                gen_ai.system_instructions = &completion_request.preamble,
                gen_ai.response.id = tracing::field::Empty,
                gen_ai.response.model = self.model,
                gen_ai.usage.output_tokens = tracing::field::Empty,
                gen_ai.usage.input_tokens = tracing::field::Empty,
            )
        } else {
            tracing::Span::current()
        };
        let body = self.selected_request_bytes(completion_request)?;

        if enabled!(Level::TRACE) {
            tracing::trace!(
                target: "rig::streaming",
                "Gemini streaming completion request: {}",
                String::from_utf8_lossy(&body)
            );
        }

        let req = self
            .client
            .post_sse(format!(
                "/v1beta/models/{}:streamGenerateContent",
                self.model
            ))?
            .header("Content-Type", "application/json")
            .body(body)
            .map_err(|e| CompletionError::HttpError(e.into()))?;

        let mut event_source = GenericEventSource::new(self.client.clone(), req);

        let stream = stream! {
            let mut final_usage = None;
            // PROV-039: Capture finish_reason from streaming for stop_reason propagation
            let mut captured_stop_reason: Option<String> = None;
            while let Some(event_result) = event_source.next().await {
                match event_result {
                    Ok(Event::Open) => {
                        tracing::debug!("SSE connection opened");
                        continue;
                    }
                    Ok(Event::Message(message)) => {
                        // Skip heartbeat messages or empty data
                        if message.data.trim().is_empty() {
                            continue;
                        }

                        let data = match serde_json::from_str::<StreamGenerateContentResponse>(&message.data) {
                            Ok(d) => d,
                            Err(error) => {
                                tracing::error!(?error, body_bytes = message.data.len(), body = %message.data, "Failed to parse SSE message");
                                continue;
                            }
                        };

                        // Emit usage event FIRST if we have usage metadata
                        // This ensures token tracking works even for SSE messages without content
                        // (e.g., initial messages that only have usage_metadata)
                        if let Some(ref usage_meta) = data.usage_metadata
                            && let Some(usage) = usage_meta.token_usage()
                        {
                            yield Ok(streaming::RawStreamingChoice::Usage(usage));
                        }

                        // Process the response data
                        let Some(choice) = data.candidates.into_iter().next() else {
                            tracing::debug!("There is no content candidate");
                            continue;
                        };

                        for part in choice.content.into_iter().flat_map(|content| content.parts) {
                            match part {
                                Part {
                                    part: PartKind::Text(text),
                                    thought: Some(true),
                                    ..
                                } => {
                                    yield Ok(streaming::RawStreamingChoice::ReasoningDelta {
                                        id: None,
                                        reasoning: text,
                                    });
                                },
                                Part {
                                    part: PartKind::Text(text),
                                    ..
                                } => {
                                    yield Ok(streaming::RawStreamingChoice::Message(text));
                                },
                                Part {
                                    part: PartKind::FunctionCall(function_call),
                                    thought_signature,
                                    ..
                                } => {
                                    yield Ok(streaming::RawStreamingChoice::ToolCall(
                                        streaming::RawStreamingToolCall::new(uuid::Uuid::new_v4().to_string(), function_call.name.clone(), function_call.args.clone())
                                            .with_signature(thought_signature)
                                    ));
                                },
                                part => {
                                    tracing::warn!(?part, "Unsupported response type with streaming");
                                }
                            }
                        }

                        // Check if this is the final response
                        if choice.finish_reason.is_some() {
                            // PROV-039: Map Gemini FinishReason to normalized stop_reason string
                            captured_stop_reason = Some(match &choice.finish_reason {
                                Some(FinishReason::Stop) => "end_turn".to_string(),
                                Some(FinishReason::MaxTokens) => "max_tokens".to_string(),
                                Some(FinishReason::Safety) => "content_filter".to_string(),
                                Some(FinishReason::Recitation) => "content_filter".to_string(),
                                Some(FinishReason::Language) => "content_filter".to_string(),
                                Some(FinishReason::Blocklist) => "content_filter".to_string(),
                                Some(FinishReason::ProhibitedContent) => "content_filter".to_string(),
                                Some(FinishReason::Spii) => "content_filter".to_string(),
                                Some(FinishReason::MalformedFunctionCall) => "end_turn".to_string(),
                                Some(FinishReason::Other) => "unknown".to_string(),
                                Some(FinishReason::FinishReasonUnspecified) => "unknown".to_string(),
                                None => "end_turn".to_string(),
                            });
                            let span = tracing::Span::current();
                            span.record_token_usage(&data.usage_metadata);
                            final_usage = data.usage_metadata;
                            break;
                        }
                    }
                    Err(crate::http_client::Error::StreamEnded) => {
                        break;
                    }
                    Err(error) => {
                        tracing::error!(?error, "SSE error");
                        event_source.close();
                        yield Err(CompletionError::HttpError(error));
                        return;
                    }
                }
            }

            // Ensure event source is closed when stream ends
            event_source.close();

            let Some(final_usage) = final_usage else {
                yield Err(CompletionError::ResponseError("missing Gemini usage".into()));
                return;
            };
            if final_usage.token_usage().is_none() {
                yield Err(CompletionError::ResponseError("inconsistent Gemini usage".into()));
                return;
            }
            yield Ok(streaming::RawStreamingChoice::FinalResponse(StreamingCompletionResponse {
                usage_metadata: final_usage,
                // PROV-039: Propagate captured stop_reason through FinalResponse
                stop_reason: captured_stop_reason,
            }));
        }.instrument(span);

        Ok(streaming::StreamingCompletionResponse::stream(Box::pin(
            stream,
        )))
    }
}

#[cfg(test)]
#[path = "streaming/deserialization_tests.rs"]
mod deserialization_tests;

#[cfg(test)]
#[path = "streaming/usage_tests.rs"]
mod usage_tests;
