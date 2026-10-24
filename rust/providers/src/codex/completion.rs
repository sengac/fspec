use super::{CodexProvider, CODEX_BASE_INSTRUCTIONS, CONTEXT_WINDOW, MAX_OUTPUT_TOKENS};
use crate::{
    convert_assistant_content, convert_tools_to_rig, extract_prompt_data,
    extract_text_from_content, CompletionResponse, LlmProvider, ProviderError, StopReason,
};
use async_trait::async_trait;
use codelet_common::{Message, MessageContent};
use codelet_tools::ToolDefinition as OurToolDefinition;
use rig::completion::CompletionRequestBuilder;
use rig::providers::openai;
use tracing::warn;

impl CodexProvider {
    fn rig_response_to_completion(
        &self,
        response: rig::completion::CompletionResponse<openai::responses_api::CompletionResponse>,
    ) -> Result<CompletionResponse, ProviderError> {
        let content_parts = convert_assistant_content(response.choice, "codex")?;
        let stop_reason = match response.raw_response.status {
            openai::responses_api::ResponseStatus::Completed => {
                let has_tool_calls =
                    response.raw_response.output.iter().any(|output| {
                        matches!(output, openai::responses_api::Output::FunctionCall(_))
                    });
                if has_tool_calls {
                    StopReason::ToolUse
                } else {
                    StopReason::EndTurn
                }
            }
            openai::responses_api::ResponseStatus::Incomplete => StopReason::MaxTokens,
            openai::responses_api::ResponseStatus::Failed => {
                let err_msg = response
                    .raw_response
                    .error
                    .as_ref()
                    .map(|error| error.message.as_str())
                    .unwrap_or("unknown error");
                warn!(error = %err_msg, "Codex API response failed");
                StopReason::EndTurn
            }
            other => {
                warn!(status = ?other, "Unexpected response status from Codex API");
                StopReason::EndTurn
            }
        };
        Ok(CompletionResponse {
            content: MessageContent::Parts(content_parts),
            stop_reason,
        })
    }
}

#[async_trait]
impl LlmProvider for CodexProvider {
    fn name(&self) -> &str {
        "codex"
    }
    fn model(&self) -> &str {
        &self.model_name
    }
    fn context_window(&self) -> usize {
        CONTEXT_WINDOW
    }
    fn max_output_tokens(&self) -> usize {
        MAX_OUTPUT_TOKENS
    }
    fn supports_caching(&self) -> bool {
        false
    }
    fn supports_streaming(&self) -> bool {
        true
    }

    async fn complete(&self, messages: &[Message]) -> Result<String, ProviderError> {
        let response = self.complete_with_tools(messages, &[]).await?;
        Ok(extract_text_from_content(&response.content))
    }

    async fn complete_with_tools(
        &self,
        messages: &[Message],
        tools: &[OurToolDefinition],
    ) -> Result<CompletionResponse, ProviderError> {
        let (preamble, prompt) = extract_prompt_data(messages);
        let rig_tools = convert_tools_to_rig(tools)?;
        let effective_preamble = preamble.unwrap_or_else(|| CODEX_BASE_INSTRUCTIONS.to_string());
        let builder = CompletionRequestBuilder::new(self.completion_model.clone(), prompt)
            .tools(rig_tools)
            .preamble(effective_preamble);
        let response = builder.send().await.map_err(|error| {
            ProviderError::api("codex", format!("Rig completion failed: {error}"))
        })?;
        self.rig_response_to_completion(response)
    }
}
