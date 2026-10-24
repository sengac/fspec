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

use crate::completion::{CompletionError, CompletionRequest as CoreCompletionRequest};
use crate::http_client::{self, HttpClientExt};
use crate::providers::openai::client::ApiResponse;
use crate::providers::openai::completion::streaming::StreamingCompletionResponse;
use crate::providers::openai::completion::{
    AssistantContent, CompletionModel, CompletionResponse, Message,
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
    let body = model.selected_request_bytes(&model.selected_request_json(completion_request)?)?;
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

    let items = adapt_response(parsed)?;
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
) -> Result<Vec<RawStreamingChoice<StreamingCompletionResponse>>, CompletionError> {
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

    let openai_usage = response
        .usage
        .ok_or_else(|| CompletionError::ResponseError("missing OpenAI usage".into()))?;
    let usage = super::GetTokenUsage::token_usage(&openai_usage)
        .ok_or_else(|| CompletionError::ResponseError("inconsistent OpenAI usage".into()))?;
    items.push(RawStreamingChoice::Usage(usage));
    items.push(RawStreamingChoice::FinalResponse(
        StreamingCompletionResponse {
            usage: openai_usage,
            stop_reason,
        },
    ));

    Ok(items)
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
