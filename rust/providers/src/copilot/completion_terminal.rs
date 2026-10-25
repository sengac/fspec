use rig::{
    completion::{CompletionError, CompletionResponse},
    message::AssistantContent,
    providers::openai::responses_api::{
        self,
        streaming::{validate_response_terminal, ResponseChunk, ResponseChunkKind},
        ResponseStatus,
    },
    OneOrMany,
};

pub(super) fn validate(
    mut response: CompletionResponse<responses_api::CompletionResponse>,
) -> Result<CompletionResponse<responses_api::CompletionResponse>, CompletionError> {
    let kind = match response.raw_response.status {
        ResponseStatus::Completed => ResponseChunkKind::ResponseCompleted,
        ResponseStatus::Failed => ResponseChunkKind::ResponseFailed,
        ResponseStatus::Incomplete => ResponseChunkKind::ResponseIncomplete,
        _ => {
            return Err(CompletionError::ResponseError(
                "Copilot Responses request has no settled terminal".into(),
            ))
        }
    };
    let terminal = validate_response_terminal(ResponseChunk {
        kind,
        response: response.raw_response.clone(),
        sequence_number: 0,
    })?
    .ok_or_else(|| CompletionError::ResponseError("Copilot Responses terminal missing".into()))?;
    if !terminal.tools {
        let content = response
            .choice
            .into_iter()
            .filter(|content| !matches!(content, AssistantContent::ToolCall(_)));
        response.choice = OneOrMany::many(content).map_err(|_| {
            CompletionError::ResponseError(
                "Copilot incomplete response has no settled assistant content".into(),
            )
        })?;
    }
    Ok(response)
}
