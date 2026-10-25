use super::{ResponseChunk, ResponseChunkKind, StreamingCompletionResponse};
use crate::completion::{CompletionError, GetTokenUsage};
use crate::providers::openai::responses_api::ResponseStatus;

#[derive(Clone, Debug, thiserror::Error)]
#[error("Responses API {code}: {message}")]
pub struct ResponsesTerminalFailure {
    pub code: String,
    pub message: String,
}

impl ResponsesTerminalFailure {
    pub(super) fn into_completion_error(self) -> CompletionError {
        crate::http_client::Error::Instance(Box::new(self)).into()
    }
}

pub struct Terminal {
    pub response: StreamingCompletionResponse,
    pub tools: bool,
}

pub fn finish(chunk: ResponseChunk) -> Result<Option<Terminal>, CompletionError> {
    let expected = match chunk.kind {
        ResponseChunkKind::ResponseCompleted => ResponseStatus::Completed,
        ResponseChunkKind::ResponseFailed => ResponseStatus::Failed,
        ResponseChunkKind::ResponseIncomplete => ResponseStatus::Incomplete,
        _ => return Ok(None),
    };
    let response = chunk.response;
    if response.status != expected || response.id.is_empty() {
        return Err(CompletionError::ResponseError(
            "Responses terminal status or identity mismatch".into(),
        ));
    }
    if let Some(error) = response.error {
        return Err(ResponsesTerminalFailure {
            code: error.code,
            message: error.message,
        }
        .into_completion_error());
    }
    let (stop, tools) = match expected {
        ResponseStatus::Completed => ("end_turn", true),
        ResponseStatus::Incomplete => match response
            .incomplete_details
            .as_ref()
            .map(|details| details.reason.as_str())
        {
            Some("max_output_tokens") => ("max_tokens", false),
            Some("content_filter") => ("content_filter", false),
            reason => {
                return Err(ResponsesTerminalFailure {
                    code: reason.unwrap_or("missing_incomplete_reason").into(),
                    message: "incomplete response cannot complete this request".into(),
                }
                .into_completion_error());
            }
        },
        _ => {
            return Err(CompletionError::ResponseError(
                "failed Responses terminal missing backend error".into(),
            ));
        }
    };
    let usage = response
        .usage
        .ok_or_else(|| CompletionError::ResponseError("missing Responses usage".into()))?;
    if usage.token_usage().is_none() {
        return Err(CompletionError::ResponseError(
            "inconsistent Responses usage".into(),
        ));
    }
    Ok(Some(Terminal {
        response: StreamingCompletionResponse {
            usage,
            stop_reason: Some(stop.into()),
            request_context: None,
            request_usage: Vec::new(),
        },
        tools,
    }))
}
