use super::{FinishReason, StreamingCompletionChunk, StreamingCompletionResponse};
use crate::completion::{CompletionError, GetTokenUsage};
use crate::http_client::HttpClientExt;
use crate::http_client::sse::{Event, GenericEventSource};
use crate::message::{ToolCall, ToolFunction};
use crate::streaming::{self, RawStreamingChoice};
use async_stream::stream;
use futures::StreamExt;
use http::Request;
use std::collections::BTreeMap;
use tracing_futures::Instrument;

pub async fn send_compatible_streaming_request<T>(
    http_client: T,
    req: Request<Vec<u8>>,
) -> Result<streaming::StreamingCompletionResponse<StreamingCompletionResponse>, CompletionError>
where
    T: HttpClientExt + Clone + 'static,
{
    let span = tracing::Span::current();
    let mut event_source = GenericEventSource::new(http_client, req);
    let stream = stream! {
        let span = tracing::Span::current();
        let mut tool_calls: BTreeMap<usize, ToolCall> = BTreeMap::new();
        let mut text_content = String::new();
        let mut final_usage = None;
        let mut captured_finish_reason: Option<String> = None;
        while let Some(event_result) = event_source.next().await {
            match event_result {
                Ok(Event::Open) => {
                    tracing::trace!("SSE connection opened");
                    continue;
                }
                Ok(Event::Message(message)) => {
                    if message.data.trim().is_empty() {
                        continue;
                    }
                    if message.data.trim() == "[DONE]" {
                        break;
                    }
                    let value = match serde_json::from_str::<serde_json::Value>(&message.data) {
                        Ok(value) => value,
                        Err(error) => {
                            yield Err(CompletionError::ResponseError(format!("malformed OpenAI stream frame: {error}")));
                            return;
                        }
                    };
                    if let Some(error) = value.get("error").filter(|error| !error.is_null()) {
                        let message = error.get("message").and_then(serde_json::Value::as_str).unwrap_or("OpenAI provider error");
                        let code = error.get("code").and_then(serde_json::Value::as_str).unwrap_or("unknown");
                        yield Err(CompletionError::ProviderError(format!("{message} ({code})")));
                        return;
                    }
                    let data = match serde_json::from_value::<StreamingCompletionChunk>(value) {
                        Ok(data) => data,
                        Err(error) => {
                            yield Err(CompletionError::ResponseError(format!("malformed OpenAI stream frame: {error}")));
                            return;
                        }
                    };
                    if let Some(usage) = data.usage {
                        final_usage = Some(usage.clone());
                        if let Some(normalized) = usage.token_usage() {
                            yield Ok(streaming::RawStreamingChoice::Usage(normalized));
                        }
                    }
                    let Some(choice) = data.choices.first() else {
                        tracing::debug!("There is no choice");
                        continue;
                    };
                    let delta = &choice.delta;
                    if !delta.tool_calls.is_empty() {
                        for tool_call in &delta.tool_calls {
                            let index = tool_call.index;
                            let existing_tool_call = tool_calls.entry(index).or_insert_with(|| ToolCall {
                                id: String::new(),
                                call_id: None,
                                function: ToolFunction {
                                    name: String::new(),
                                    arguments: serde_json::Value::Null,
                                },
                                signature: None,
                                additional_params: None,
                            });
                            if let Some(id) = &tool_call.id && !id.is_empty() {
                                existing_tool_call.id = id.clone();
                            }
                            if let Some(name) = &tool_call.function.name && !name.is_empty() {
                                existing_tool_call.function.name = name.clone();
                                yield Ok(streaming::RawStreamingChoice::ToolCallDelta {
                                    id: existing_tool_call.id.clone(),
                                    content: streaming::ToolCallDeltaContent::Name(name.clone()),
                                });
                            }
                            if let Some(chunk) = &tool_call.function.arguments && !chunk.is_empty() {
                                let current_args = match &existing_tool_call.function.arguments {
                                    serde_json::Value::Null => String::new(),
                                    serde_json::Value::String(arguments) => arguments.clone(),
                                    arguments => arguments.to_string(),
                                };
                                let combined = format!("{current_args}{chunk}");
                                if combined.trim_start().starts_with('{') && combined.trim_end().ends_with('}') {
                                    match serde_json::from_str(&combined) {
                                        Ok(parsed) => existing_tool_call.function.arguments = parsed,
                                        Err(_) => existing_tool_call.function.arguments = serde_json::Value::String(combined),
                                    }
                                } else {
                                    existing_tool_call.function.arguments = serde_json::Value::String(combined);
                                }
                                yield Ok(streaming::RawStreamingChoice::ToolCallDelta {
                                    id: existing_tool_call.id.clone(),
                                    content: streaming::ToolCallDeltaContent::Delta(chunk.clone()),
                                });
                            }
                        }
                    }
                    if delta.reasoning_content.is_some() || delta.reasoning.is_some() {
                        let mut combined_reasoning = String::new();
                        if let Some(reasoning_content) = &delta.reasoning_content {
                            combined_reasoning.push_str(reasoning_content);
                        }
                        if let Some(reasoning) = &delta.reasoning {
                            combined_reasoning.push_str(reasoning);
                        }
                        if !combined_reasoning.is_empty() {
                            yield Ok(streaming::RawStreamingChoice::ReasoningDelta {
                                id: None,
                                reasoning: combined_reasoning,
                            });
                        }
                    }
                    if let Some(content) = &delta.content && !content.is_empty() {
                        text_content += content;
                        yield Ok(streaming::RawStreamingChoice::Message(content.clone()));
                    }
                    if let Some(finish_reason) = &choice.finish_reason {
                        captured_finish_reason = Some(match finish_reason {
                            FinishReason::Stop => "end_turn".to_string(),
                            FinishReason::Length => "max_tokens".to_string(),
                            FinishReason::ToolCalls => "tool_use".to_string(),
                            FinishReason::ContentFilter => "content_filter".to_string(),
                            FinishReason::Other(reason) => reason.clone(),
                        });
                    }
                }
                Err(crate::http_client::Error::StreamEnded) => break,
                Err(error) => {
                    tracing::error!(?error, "SSE error");
                    event_source.close();
                    yield Err(CompletionError::HttpError(error));
                    return;
                }
            }
        }
        event_source.close();
        if captured_finish_reason.as_deref().is_none_or(|reason| reason.trim().is_empty()) {
            yield Err(CompletionError::ResponseError("missing OpenAI finish reason".into()));
            return;
        }
        let Some(final_usage) = final_usage else {
            yield Err(CompletionError::ResponseError("missing OpenAI usage".into()));
            return;
        };
        if final_usage.token_usage().is_none() {
            yield Err(CompletionError::ResponseError("inconsistent OpenAI usage".into()));
            return;
        }
        if !span.is_disabled() {
            span.record("gen_ai.usage.input_tokens", final_usage.prompt_tokens);
            span.record("gen_ai.usage.output_tokens", final_usage.total_tokens - final_usage.prompt_tokens);
        }
        yield Ok(RawStreamingChoice::FinalResponse(StreamingCompletionResponse {
            usage: final_usage,
            stop_reason: captured_finish_reason,
        }));
        for (_index, tool_call) in tool_calls.into_iter() {
            yield Ok(streaming::RawStreamingChoice::ToolCall(
                streaming::RawStreamingToolCall::new(
                    tool_call.id,
                    tool_call.function.name,
                    tool_call.function.arguments,
                )
            ));
        }
    }.instrument(span);
    Ok(streaming::StreamingCompletionResponse::stream(Box::pin(
        stream,
    )))
}
