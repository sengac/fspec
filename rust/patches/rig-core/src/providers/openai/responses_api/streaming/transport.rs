use super::*;
use crate::completion::CompletionError;
use crate::http_client::HttpClientExt;
use crate::http_client::sse::{Event, GenericEventSource};
use crate::providers::openai::responses_api::{ReasoningSummary, ResponsesCompletionModel};
use crate::streaming::{self, RawStreamingChoice};
use crate::wasm_compat::WasmCompatSend;
use async_stream::stream;
use futures::StreamExt;
use tracing::{Level, enabled, info_span};
use tracing_futures::Instrument as _;

impl<T> ResponsesCompletionModel<T>
where
    T: HttpClientExt + Clone + Default + std::fmt::Debug + WasmCompatSend + 'static,
{
    pub(crate) async fn stream(
        &self,
        completion_request: crate::completion::CompletionRequest,
    ) -> Result<streaming::StreamingCompletionResponse<StreamingCompletionResponse>, CompletionError>
    {
        let body = self.streaming_request_bytes(completion_request)?;
        if enabled!(Level::TRACE) {
            tracing::trace!(target: "rig::completions",
                "OpenAI Responses streaming completion request: {}", String::from_utf8_lossy(&body));
        }
        let req = self
            .client
            .post("/responses")?
            .body(body)
            .map_err(|error| CompletionError::HttpError(error.into()))?;
        let span = if tracing::Span::current().is_disabled() {
            info_span!(target: "rig::completions", "chat_streaming",
                gen_ai.operation.name = "chat_streaming",
                gen_ai.provider.name = tracing::field::Empty,
                gen_ai.request.model = tracing::field::Empty,
                gen_ai.response.id = tracing::field::Empty,
                gen_ai.response.model = tracing::field::Empty,
                gen_ai.usage.output_tokens = tracing::field::Empty,
                gen_ai.usage.input_tokens = tracing::field::Empty)
        } else {
            tracing::Span::current()
        };
        span.record("gen_ai.provider.name", "openai");
        span.record("gen_ai.request.model", &self.model);
        let client = self.client.clone();
        let mut event_source = GenericEventSource::new(client, req);
        let stream = stream! {
            let mut terminal = None;
            let mut tool_calls: Vec<RawStreamingChoice<StreamingCompletionResponse>> = Vec::new();
            let mut combined_text = String::new();
            let span = tracing::Span::current();
            while let Some(event_result) = event_source.next().await {
                match event_result {
                    Ok(Event::Open) => {
                        tracing::trace!("SSE connection opened");
                        tracing::info!("OpenAI stream started");
                        continue;
                    }
                    Ok(Event::Message(event)) => {
                        if event.data.trim().is_empty() { continue; }
                        if event.data.trim() == "[DONE]" { break; }
                        let data = match serde_json::from_str::<StreamingCompletionChunk>(&event.data) {
                            Ok(data) => data,
                            Err(error) => {
                                tracing::debug!(?error, frame_bytes = event.data.len(), "Skipping unsupported Responses frame");
                                continue;
                            }
                        };
                        if let StreamingCompletionChunk::Error(chunk) = data {
                            yield Err(ResponsesTerminalFailure {
                                code: chunk.error.code.unwrap_or_else(|| "backend_error".into()),
                                message: chunk.error.message,
                            }.into_completion_error());
                            return;
                        }
                        if let StreamingCompletionChunk::Delta(chunk) = &data {
                            match &chunk.data {
                                ItemChunkKind::OutputItemAdded(message) => {
                                    if let StreamingItemDoneOutput { item: Output::FunctionCall(function), .. } = message {
                                        yield Ok(RawStreamingChoice::ToolCallDelta { id: function.id.clone(), content: streaming::ToolCallDeltaContent::Name(function.name.clone()) });
                                    }
                                }
                                ItemChunkKind::OutputItemDone(message) => match message {
                                    StreamingItemDoneOutput { item: Output::FunctionCall(function), .. } => {
                                        tool_calls.push(RawStreamingChoice::ToolCall(streaming::RawStreamingToolCall::new(
                                            function.id.clone(), function.name.clone(), function.arguments.clone()
                                        ).with_call_id(function.call_id.clone())));
                                    }
                                    StreamingItemDoneOutput { item: Output::Reasoning { summary, id }, .. } => {
                                        let reasoning = summary.iter().map(|summary| {
                                            let ReasoningSummary::SummaryText { text } = summary;
                                            text.to_owned()
                                        }).collect::<Vec<String>>().join("\n");
                                        yield Ok(RawStreamingChoice::Reasoning { id: Some(id.to_string()), reasoning, signature: None });
                                    }
                                    _ => continue,
                                },
                                ItemChunkKind::OutputTextDelta(delta) | ItemChunkKind::RefusalDelta(delta) => {
                                    combined_text.push_str(&delta.delta);
                                    yield Ok(RawStreamingChoice::Message(delta.delta.clone()));
                                }
                                ItemChunkKind::ReasoningSummaryTextDelta(delta) => {
                                    yield Ok(RawStreamingChoice::ReasoningDelta { id: None, reasoning: delta.delta.clone() });
                                }
                                ItemChunkKind::FunctionCallArgsDelta(delta) => {
                                    yield Ok(RawStreamingChoice::ToolCallDelta { id: delta.item_id.clone(), content: streaming::ToolCallDeltaContent::Delta(delta.delta.clone()) });
                                }
                                _ => continue,
                            }
                        }
                        if let StreamingCompletionChunk::Response(chunk) = data {
                            span.record("gen_ai.response.id", &chunk.response.id);
                            span.record("gen_ai.response.model", &chunk.response.model);
                            match super::terminal::finish(*chunk) {
                                Ok(Some(completed)) => { terminal = Some(completed); break; }
                                Ok(None) => continue,
                                Err(error) => { yield Err(error); return; }
                            }
                        }
                    }
                    Err(crate::http_client::Error::StreamEnded) => { event_source.close(); }
                    Err(error) => {
                        tracing::error!(?error, "SSE error");
                        event_source.close();
                        yield Err(CompletionError::HttpError(error));
                        return;
                    }
                }
            }
            event_source.close();
            let Some(terminal) = terminal else {
                yield Err(CompletionError::ResponseError("missing Responses terminal evidence".into()));
                return;
            };
            if let Some(usage) = terminal.response.token_usage() {
                yield Ok(RawStreamingChoice::Usage(usage));
            }
            span.record("gen_ai.usage.input_tokens", terminal.response.usage.input_tokens);
            span.record("gen_ai.usage.output_tokens", terminal.response.usage.output_tokens);
            tracing::info!("OpenAI stream finished");
            yield Ok(RawStreamingChoice::FinalResponse(terminal.response));
            if terminal.tools {
                for tool_call in tool_calls { yield Ok(tool_call); }
            }
        }.instrument(span);
        Ok(streaming::StreamingCompletionResponse::stream(Box::pin(
            stream,
        )))
    }
}
