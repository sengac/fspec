//! The streaming module for the OpenAI Responses API.
//! Please see the `openai_streaming` or `openai_streaming_with_tools` example for more practical usage.
use crate::completion::{CompletionError, GetTokenUsage};
use crate::http_client::HttpClientExt;
use crate::http_client::sse::{Event, GenericEventSource};
use crate::providers::openai::responses_api::{
    ReasoningSummary, ResponsesCompletionModel, ResponsesUsage,
};
use crate::streaming;
use crate::streaming::RawStreamingChoice;
use crate::wasm_compat::WasmCompatSend;
use async_stream::stream;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tracing::{Level, debug, enabled, info_span};
use tracing_futures::Instrument as _;

use super::{CompletionResponse, Output};

// ================================================================
// OpenAI Responses Streaming API
// ================================================================

/// A streaming completion chunk.
/// Streaming chunks can come in one of three forms:
/// - A response chunk (where the completed response will have the total token usage)
/// - An item chunk commonly referred to as a delta. In the completions API this would be referred to as the message delta.
/// - An error chunk: the Codex backend reports request-level failures (e.g. rejected tool
///   schemas) as a single SSE `{"type":"error",...}` event over HTTP 200. Without this
///   variant the untagged enum cannot deserialize it and the event is silently skipped,
///   terminating the conversation with an empty response (BUG-208).
///   Mirrors codex-api/src/sse/responses.rs:349 (`Error` event => `ApiError`).
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum StreamingCompletionChunk {
    Error(ErrorChunk),
    Response(Box<ResponseChunk>),
    Delta(ItemChunk),
}

/// An error event from the Responses API backend (BUG-208).
/// Shape: `{"type":"error","error":{"message":"...","code":"..."}}` — the `code` field
/// is optional (the OpenAI API sometimes omits it; the Codex backend includes it, e.g.
/// `invalid_function_parameters`).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ErrorChunk {
    /// Always "error" — the discriminator that makes the untagged match precise.
    #[serde(rename = "type")]
    pub kind: ErrorChunkKind,
    /// The backend error payload.
    pub error: BackendError,
}

/// Discriminator for [`ErrorChunk`].
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum ErrorChunkKind {
    #[serde(rename = "error")]
    Error,
}

/// The error payload of an [`ErrorChunk`].
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BackendError {
    /// Human-readable message from the backend (e.g. "Invalid schema for function 'X'…").
    pub message: String,
    /// Machine-readable error code (e.g. "invalid_function_parameters"), when provided.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Parameter the error refers to, when provided.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
}

/// The final streaming response from the OpenAI Responses API.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StreamingCompletionResponse {
    /// Token usage
    pub usage: ResponsesUsage,
}

impl GetTokenUsage for StreamingCompletionResponse {
    fn token_usage(&self) -> Option<crate::completion::Usage> {
        let mut usage = crate::completion::Usage::new();
        usage.input_tokens = self.usage.input_tokens;
        usage.output_tokens = self.usage.output_tokens;
        usage.total_tokens = self.usage.total_tokens;
        usage.cache_read_input_tokens = self
            .usage
            .input_tokens_details
            .as_ref()
            .map(|d| d.cached_tokens);
        usage.reasoning_tokens = Some(self.usage.output_tokens_details.reasoning_tokens);
        Some(usage)
    }
}

/// A response chunk from OpenAI's response API.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResponseChunk {
    /// The response chunk type
    #[serde(rename = "type")]
    pub kind: ResponseChunkKind,
    /// The response itself
    pub response: CompletionResponse,
    /// The item sequence
    pub sequence_number: u64,
}

/// Response chunk type.
/// Renames are used to ensure that this type gets (de)serialized properly.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ResponseChunkKind {
    #[serde(rename = "response.created")]
    ResponseCreated,
    #[serde(rename = "response.in_progress")]
    ResponseInProgress,
    #[serde(rename = "response.completed")]
    ResponseCompleted,
    #[serde(rename = "response.failed")]
    ResponseFailed,
    #[serde(rename = "response.incomplete")]
    ResponseIncomplete,
}

/// An item message chunk from OpenAI's Responses API.
/// See
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemChunk {
    /// Item ID. Optional.
    pub item_id: Option<String>,
    /// The output index of the item from a given streamed response.
    pub output_index: u64,
    /// The item type chunk, as well as the inner data.
    #[serde(flatten)]
    pub data: ItemChunkKind,
}

/// The item chunk type from OpenAI's Responses API.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum ItemChunkKind {
    #[serde(rename = "response.output_item.added")]
    OutputItemAdded(StreamingItemDoneOutput),
    #[serde(rename = "response.output_item.done")]
    OutputItemDone(StreamingItemDoneOutput),
    #[serde(rename = "response.content_part.added")]
    ContentPartAdded(ContentPartChunk),
    #[serde(rename = "response.content_part.done")]
    ContentPartDone(ContentPartChunk),
    #[serde(rename = "response.output_text.delta")]
    OutputTextDelta(DeltaTextChunk),
    #[serde(rename = "response.output_text.done")]
    OutputTextDone(OutputTextChunk),
    #[serde(rename = "response.refusal.delta")]
    RefusalDelta(DeltaTextChunk),
    #[serde(rename = "response.refusal.done")]
    RefusalDone(RefusalTextChunk),
    #[serde(rename = "response.function_call_arguments.delta")]
    FunctionCallArgsDelta(DeltaTextChunkWithItemId),
    #[serde(rename = "response.function_call_arguments.done")]
    FunctionCallArgsDone(ArgsTextChunk),
    #[serde(rename = "response.reasoning_summary_part.added")]
    ReasoningSummaryPartAdded(SummaryPartChunk),
    #[serde(rename = "response.reasoning_summary_part.done")]
    ReasoningSummaryPartDone(SummaryPartChunk),
    #[serde(rename = "response.reasoning_summary_text.delta")]
    ReasoningSummaryTextDelta(SummaryTextChunk),
    #[serde(rename = "response.reasoning_summary_text.done")]
    ReasoningSummaryTextDone(SummaryTextChunk),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StreamingItemDoneOutput {
    pub sequence_number: u64,
    pub item: Output,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContentPartChunk {
    pub content_index: u64,
    pub sequence_number: u64,
    pub part: ContentPartChunkPart,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum ContentPartChunkPart {
    OutputText { text: String },
    SummaryText { text: String },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeltaTextChunk {
    pub content_index: u64,
    pub sequence_number: u64,
    pub delta: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeltaTextChunkWithItemId {
    pub item_id: String,
    pub content_index: u64,
    pub sequence_number: u64,
    pub delta: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OutputTextChunk {
    pub content_index: u64,
    pub sequence_number: u64,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RefusalTextChunk {
    pub content_index: u64,
    pub sequence_number: u64,
    pub refusal: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArgsTextChunk {
    pub content_index: u64,
    pub sequence_number: u64,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SummaryPartChunk {
    pub summary_index: u64,
    pub sequence_number: u64,
    pub part: SummaryPartChunkPart,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SummaryTextChunk {
    pub summary_index: u64,
    pub sequence_number: u64,
    pub delta: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum SummaryPartChunkPart {
    SummaryText { text: String },
}

impl<T> ResponsesCompletionModel<T>
where
    T: HttpClientExt + Clone + Default + std::fmt::Debug + WasmCompatSend + 'static,
{
    pub(crate) async fn stream(
        &self,
        completion_request: crate::completion::CompletionRequest,
    ) -> Result<streaming::StreamingCompletionResponse<StreamingCompletionResponse>, CompletionError>
    {
        let mut request = self.create_completion_request(completion_request)?;
        request.stream = Some(true);

        if enabled!(Level::TRACE) {
            tracing::trace!(
                target: "rig::completions",
                "OpenAI Responses streaming completion request: {}",
                serde_json::to_string_pretty(&request)?
            );
        }

        let body = serde_json::to_vec(&request)?;

        let req = self
            .client
            .post("/responses")?
            .body(body)
            .map_err(|e| CompletionError::HttpError(e.into()))?;

        // let request_builder = self.client.post_reqwest("/responses").json(&request);

        let span = if tracing::Span::current().is_disabled() {
            info_span!(
                target: "rig::completions",
                "chat_streaming",
                gen_ai.operation.name = "chat_streaming",
                gen_ai.provider.name = tracing::field::Empty,
                gen_ai.request.model = tracing::field::Empty,
                gen_ai.response.id = tracing::field::Empty,
                gen_ai.response.model = tracing::field::Empty,
                gen_ai.usage.output_tokens = tracing::field::Empty,
                gen_ai.usage.input_tokens = tracing::field::Empty,
            )
        } else {
            tracing::Span::current()
        };
        span.record("gen_ai.provider.name", "openai");
        span.record("gen_ai.request.model", &self.model);
        // Build the request with proper headers for SSE
        let client = self.client.clone();

        let mut event_source = GenericEventSource::new(client, req);

        let stream = stream! {
            let mut final_usage = ResponsesUsage::new();
            // BUG-208: set when a backend error event terminated the stream; in that
            // case no (silent, empty) FinalResponse may be emitted afterwards.
            let mut stream_errored = false;

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
                    Ok(Event::Message(evt)) => {
                        // Skip heartbeat messages or empty data
                        if evt.data.trim().is_empty() {
                            continue;
                        }

                        let data = serde_json::from_str::<StreamingCompletionChunk>(&evt.data);

                        let Ok(data) = data else {
                            let err = data.unwrap_err();
                            debug!("Couldn't serialize data as StreamingCompletionResponse: {err:?}\nRAW SSE DATA: {}", &evt.data);
                            continue;
                        };

                        // BUG-208: a backend error event must surface as a stream error,
                        // never be silently skipped (mirrors codex-api sse/responses.rs
                        // stream loop sending Err(error) on an Error event).
                        if let StreamingCompletionChunk::Error(err_chunk) = data {
                            let code = err_chunk
                                .error
                                .code
                                .as_deref()
                                .map(|c| format!(" (code: {c})"))
                                .unwrap_or_default();
                            yield Err(CompletionError::ProviderError(format!(
                                "Responses API backend error: {}{code}",
                                err_chunk.error.message
                            )));
                            stream_errored = true;
                            break;
                        }

                        if let StreamingCompletionChunk::Delta(chunk) = &data {
                            match &chunk.data {
                                ItemChunkKind::OutputItemAdded(message) => {
                                    if let StreamingItemDoneOutput { item: Output::FunctionCall(func), .. } = message {
                                        yield Ok(streaming::RawStreamingChoice::ToolCallDelta {
                                            id: func.id.clone(),
                                            content: streaming::ToolCallDeltaContent::Name(func.name.clone()),
                                        });
                                    }
                                }
                                ItemChunkKind::OutputItemDone(message) => {
                                    match message {
                                        StreamingItemDoneOutput {  item: Output::FunctionCall(func), .. } => {
                                            tool_calls.push(streaming::RawStreamingChoice::ToolCall(
                                                streaming::RawStreamingToolCall::new(
                                                    func.id.clone(),
                                                    func.name.clone(),
                                                    func.arguments.clone(),
                                                )
                                                .with_call_id(func.call_id.clone())
                                            ));
                                        }

                                        StreamingItemDoneOutput {  item: Output::Reasoning {  summary, id }, .. } => {
                                            let reasoning = summary
                                                .iter()
                                                .map(|x| {
                                                    let ReasoningSummary::SummaryText { text } = x;
                                                    text.to_owned()
                                                })
                                                .collect::<Vec<String>>()
                                                .join("\n");
                                            yield Ok(streaming::RawStreamingChoice::Reasoning {
                                                id: Some(id.to_string()),
                                                reasoning,
                                                signature: None,
                                            })
                                        }
                                        _ => continue
                                    }
                                }
                                ItemChunkKind::OutputTextDelta(delta) => {
                                    combined_text.push_str(&delta.delta);
                                    yield Ok(streaming::RawStreamingChoice::Message(delta.delta.clone()))
                                }
                                ItemChunkKind::ReasoningSummaryTextDelta(delta) => {
                                    yield Ok(streaming::RawStreamingChoice::ReasoningDelta { id: None, reasoning: delta.delta.clone() })
                                }
                                ItemChunkKind::RefusalDelta(delta) => {
                                    combined_text.push_str(&delta.delta);
                                    yield Ok(streaming::RawStreamingChoice::Message(delta.delta.clone()))
                                }
                                ItemChunkKind::FunctionCallArgsDelta(delta) => {
                                    yield Ok(streaming::RawStreamingChoice::ToolCallDelta { id: delta.item_id.clone(), content: streaming::ToolCallDeltaContent::Delta(delta.delta.clone()) })
                                }

                                _ => { continue }
                            }
                        }

                        if let StreamingCompletionChunk::Response(chunk) = data {
                            if let ResponseChunk { kind: ResponseChunkKind::ResponseCompleted, response, .. } = *chunk {
                                span.record("gen_ai.response.id", response.id);
                                span.record("gen_ai.response.model", response.model);
                                if let Some(usage) = response.usage {
                                    final_usage = usage.clone();
                                    // Emit Usage event so stream_loop gets real-time token data
                                    // (mirrors what Chat Completions API already does)
                                    let crate_usage = crate::completion::Usage {
                                        input_tokens: usage.input_tokens,
                                        output_tokens: usage.output_tokens,
                                        total_tokens: usage.total_tokens,
                                        cache_read_input_tokens: usage.input_tokens_details
                                            .as_ref()
                                            .map(|d| d.cached_tokens),
                                        reasoning_tokens: Some(usage.output_tokens_details.reasoning_tokens),
                                        cache_creation_input_tokens: None, // Responses API doesn't report cache creation
                                    };
                                    yield Ok(RawStreamingChoice::Usage(crate_usage));
                                }
                            } else {
                                continue;
                            }
                        }
                    }
                    Err(crate::http_client::Error::StreamEnded) => {
                        event_source.close();
                    }
                    Err(error) => {
                        tracing::error!(?error, "SSE error");
                        yield Err(CompletionError::ProviderError(error.to_string()));
                        break;
                    }
                }
            }

            // Ensure event source is closed when stream ends
            event_source.close();

            // BUG-208: after a backend error event the stream must end with the Err
            // only — emitting an empty FinalResponse afterwards is exactly the
            // silent-termination symptom (the agent treats it as a completed turn).
            if stream_errored {
                return;
            }

            for tool_call in &tool_calls {
                yield Ok(tool_call.to_owned())
            }

            span.record("gen_ai.usage.input_tokens", final_usage.input_tokens);
            span.record("gen_ai.usage.output_tokens", final_usage.output_tokens);
            tracing::info!("OpenAI stream finished");

            yield Ok(RawStreamingChoice::FinalResponse(StreamingCompletionResponse {
                usage: final_usage
            }));
        }.instrument(span);

        Ok(streaming::StreamingCompletionResponse::stream(Box::pin(
            stream,
        )))
    }
}
