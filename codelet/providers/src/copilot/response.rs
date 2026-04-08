//! Rig → fspec response conversion for the GitHub Copilot provider.
//!
//! Mirrors the pattern used by [`ZAIProvider::rig_response_to_completion`]
//! and [`OpenAIProvider::rig_response_to_completion`] — extracted into a
//! free function here to keep [`CopilotProvider`] focused on composition.

use crate::error::ProviderError;
use crate::{convert_assistant_content, CompletionResponse, StopReason};
use codelet_common::MessageContent;
use tracing::warn;

/// Convert a rig completion response into our [`CompletionResponse`] format.
pub(super) fn rig_response_to_completion(
    response: rig::completion::CompletionResponse<
        rig::providers::openai::completion::CompletionResponse,
    >,
) -> Result<CompletionResponse, ProviderError> {
    // Reuse the shared adapter helper to translate rig's AssistantContent
    // OneOrMany into our MessageContent parts (REFAC-013).
    let content_parts = convert_assistant_content(response.choice, "github-copilot")?;

    let stop_reason = response
        .raw_response
        .choices
        .first()
        .map_or(StopReason::EndTurn, |choice| match choice
            .finish_reason
            .as_str()
        {
            "tool_calls" | "function_call" => StopReason::ToolUse,
            "length" => StopReason::MaxTokens,
            "stop" | "end_turn" | "" => StopReason::EndTurn,
            other => {
                warn!(finish_reason = %other, "Unknown finish_reason from Copilot API");
                StopReason::EndTurn
            }
        });

    Ok(CompletionResponse {
        content: MessageContent::Parts(content_parts),
        stop_reason,
    })
}
