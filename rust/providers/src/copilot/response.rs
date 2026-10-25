//! Rig → fspec response conversion for the GitHub Copilot provider.
//!
//! Mirrors the pattern used by [`ZAIProvider::rig_response_to_completion`]
//! and [`OpenAIProvider::rig_response_to_completion`] — extracted into a
//! free function here to keep [`CopilotProvider`] focused on composition.

use super::completion_model::CopilotCompletionResponse;
use crate::error::ProviderError;
use crate::{convert_assistant_content, CompletionResponse, StopReason};
use codelet_common::MessageContent;
use tracing::warn;

/// Convert a rig completion response into our [`CompletionResponse`] format.
pub(super) fn rig_response_to_completion(
    response: rig::completion::CompletionResponse<CopilotCompletionResponse>,
) -> Result<CompletionResponse, ProviderError> {
    // Reuse the shared adapter helper to translate rig's AssistantContent
    // OneOrMany into our MessageContent parts (REFAC-013).
    let content_parts = convert_assistant_content(response.choice, "github-copilot")?;

    let stop_reason = match response.raw_response {
        CopilotCompletionResponse::Responses(raw) => {
            if raw.status == rig::providers::openai::responses_api::ResponseStatus::Incomplete {
                return match raw
                    .incomplete_details
                    .as_ref()
                    .map(|details| details.reason.as_str())
                {
                    Some("max_output_tokens") => Ok(CompletionResponse {
                        content: MessageContent::Parts(content_parts),
                        stop_reason: StopReason::MaxTokens,
                    }),
                    _ => Err(ProviderError::api(
                        "github-copilot",
                        "Copilot response stopped without ordinary completion",
                    )),
                };
            }
            if content_parts
                .iter()
                .any(|part| matches!(part, codelet_common::ContentPart::ToolUse { .. }))
            {
                StopReason::ToolUse
            } else {
                StopReason::EndTurn
            }
        }
        CopilotCompletionResponse::Chat(raw) => {
            raw.choices.first().map_or(StopReason::EndTurn, |choice| {
                match choice.finish_reason.as_str() {
                    "tool_calls" | "function_call" => StopReason::ToolUse,
                    "length" => StopReason::MaxTokens,
                    "stop" | "end_turn" | "" => StopReason::EndTurn,
                    other => {
                        warn!(finish_reason = %other, "Unknown finish_reason from Copilot API");
                        StopReason::EndTurn
                    }
                }
            })
        }
    };

    Ok(CompletionResponse {
        content: MessageContent::Parts(content_parts),
        stop_reason,
    })
}
