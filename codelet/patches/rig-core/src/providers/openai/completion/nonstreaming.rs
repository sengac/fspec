// ================================================================
// PROV-140: OpenAI non-streaming request + single-response adapter
//
// Feature: spec/features/openai-nonstreaming-request-loop.feature
//
// When a model's `stream` flag is `false`, `CompletionModel::stream()`
// delegates here. We issue ONE non-streaming Chat Completions request
// (`stream: false`, deliberately no `stream_options`) and adapt the single
// JSON response into the SAME one-item `RawStreamingChoice` stream the
// multi-turn state machine consumes for the SSE path — so tool calls still
// drive the loop and the terminal item sequence is identical.
// ================================================================

use async_stream::stream;
use serde_json::json;

use crate::completion::{CompletionError, CompletionRequest as CoreCompletionRequest};
use crate::http_client::{self, HttpClientExt};
use crate::json_utils::merge;
use crate::providers::openai::client::ApiResponse;
use crate::providers::openai::completion::streaming::StreamingCompletionResponse;
use crate::providers::openai::completion::{
    AssistantContent, CompletionModel, CompletionRequest, CompletionResponse, Message,
    OpenAIRequestParams, Usage,
};
use crate::streaming::{self, RawStreamingChoice, RawStreamingToolCall};

/// Issue a single non-streaming completion request and yield its content as a
/// one-item stream in the shape the multi-turn driver expects.
pub(crate) async fn stream_nonstreaming<T>(
    model: &CompletionModel<T>,
    completion_request: CoreCompletionRequest,
) -> Result<streaming::StreamingCompletionResponse<StreamingCompletionResponse>, CompletionError>
where
    T: HttpClientExt + Clone + 'static,
{
    let request = CompletionRequest::try_from(OpenAIRequestParams {
        model: model.model.clone(),
        request: completion_request,
        strict_tools: model.strict_tools,
        tool_result_array_content: model.tool_result_array_content,
    })?;

    let mut request_as_json = serde_json::to_value(request)?;
    // PROV-140: explicit `stream: false`, and NO `stream_options` key — the
    // non-streaming endpoint rejects (or ignores) streaming-only options.
    request_as_json = merge(request_as_json, json!({ "stream": false }));

    let body = serde_json::to_vec(&request_as_json)?;
    let req = model
        .client
        .post("/chat/completions")?
        .body(body)
        .map_err(|e| CompletionError::HttpError(e.into()))?;

    let response = model.client.send(req).await?;
    let status = response.status();
    let text = http_client::text(response).await?;

    if !status.is_success() {
        return Err(CompletionError::ProviderError(text));
    }

    let parsed = match serde_json::from_str::<ApiResponse<CompletionResponse>>(&text)? {
        ApiResponse::Ok(parsed) => parsed,
        ApiResponse::Err(err) => return Err(CompletionError::ProviderError(err.message)),
    };

    let items = adapt_response(parsed);
    let stream = stream! {
        for item in items {
            yield Ok(item);
        }
    };

    Ok(streaming::StreamingCompletionResponse::stream(Box::pin(
        stream,
    )))
}

/// Convert a single non-streaming `CompletionResponse` into the ordered
/// `RawStreamingChoice` items a streaming turn would have produced: optional
/// reasoning, text, tool calls, a usage event, then the terminal
/// `FinalResponse` carrying usage + normalized stop_reason.
fn adapt_response(
    response: CompletionResponse,
) -> Vec<RawStreamingChoice<StreamingCompletionResponse>> {
    let mut items: Vec<RawStreamingChoice<StreamingCompletionResponse>> = Vec::new();

    let stop_reason = response
        .choices
        .first()
        .map(|choice| map_finish_reason(&choice.finish_reason));

    if let Some(choice) = response.choices.first() {
        if let Message::Assistant {
            content,
            tool_calls,
            reasoning,
            ..
        } = &choice.message
        {
            if let Some(reasoning_text) = reasoning {
                if !reasoning_text.is_empty() {
                    items.push(RawStreamingChoice::ReasoningDelta {
                        id: None,
                        reasoning: reasoning_text.clone(),
                    });
                }
            }

            for part in content {
                if let AssistantContent::Text { text } = part {
                    if !text.is_empty() {
                        items.push(RawStreamingChoice::Message(text.clone()));
                    }
                }
            }

            for call in tool_calls {
                items.push(RawStreamingChoice::ToolCall(RawStreamingToolCall::new(
                    call.id.clone(),
                    call.function.name.clone(),
                    call.function.arguments.clone(),
                )));
            }
        }
    }

    let openai_usage = response.usage.unwrap_or_default();
    items.push(RawStreamingChoice::Usage(core_usage(&openai_usage)));
    items.push(RawStreamingChoice::FinalResponse(StreamingCompletionResponse {
        usage: openai_usage,
        stop_reason,
    }));

    items
}

/// Map OpenAI's non-streaming `finish_reason` string to the normalized
/// stop_reason the streaming path emits (parity with streaming.rs).
fn map_finish_reason(finish_reason: &str) -> String {
    match finish_reason {
        "stop" | "end_turn" => "end_turn".to_string(),
        "length" => "max_tokens".to_string(),
        "tool_calls" => "tool_use".to_string(),
        "content_filter" => "content_filter".to_string(),
        other => other.to_string(),
    }
}

/// Build the crate-level `Usage` event from the OpenAI usage block, mirroring
/// the streaming decoder's token accounting.
fn core_usage(usage: &Usage) -> crate::completion::Usage {
    let cached_tokens = usage
        .prompt_tokens_details
        .as_ref()
        .map(|d| d.cached_tokens as u64)
        .filter(|c| *c > 0);
    let reasoning_tokens = usage
        .completion_tokens_details
        .as_ref()
        .map(|d| d.reasoning_tokens as u64);

    crate::completion::Usage {
        input_tokens: usage.prompt_tokens as u64,
        output_tokens: usage.output_tokens(),
        total_tokens: usage.total_tokens as u64,
        cache_read_input_tokens: cached_tokens,
        reasoning_tokens,
        ..Default::default()
    }
}
