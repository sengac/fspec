//! OpenAIProvider model-limit resolution and LlmProvider implementation
//! (REFAC-031: moved from openai.rs).

use super::{anti_ramble_additional_params, OpenAIProvider, ANTI_RAMBLE_TEMPERATURE};
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

// ---------------------------------------------------------------------------
// LIMITS-003: ModelLimitsResolver for OpenAI
// ---------------------------------------------------------------------------

impl crate::model_limits::ModelLimitsResolver for OpenAIProvider {
    /// OpenAI trusts registry values — no hard ceiling.
    fn max_context_window(&self) -> Option<usize> {
        None
    }

    /// OpenAI trusts registry values — no hard ceiling.
    fn max_output_tokens_limit(&self) -> Option<usize> {
        None
    }

    /// PROV-171: `OPENAI_CONTEXT_WINDOW` read at construction; `None` when unset.
    fn default_context_window(&self) -> Option<usize> {
        self.context_window
    }

    /// PROV-171: `OPENAI_MAX_OUTPUT_TOKENS` read at construction (or the
    /// session's resolved limit); `None` when not declared.
    fn default_max_output_tokens(&self) -> Option<usize> {
        self.max_output_tokens
    }
}

impl OpenAIProvider {
    /// Convert rig response to our CompletionResponse format
    fn rig_response_to_completion(
        &self,
        response: rig::completion::CompletionResponse<openai::completion::CompletionResponse>,
    ) -> Result<CompletionResponse, ProviderError> {
        // Convert rig AssistantContent to our ContentPart format using shared helper (REFAC-013)
        let content_parts = convert_assistant_content(response.choice, "openai")?;

        // Map OpenAI's finish_reason to our StopReason enum (provider-specific)
        let stop_reason = match response.raw_response.choices.first() {
            Some(choice) => match choice.finish_reason.as_str() {
                "tool_calls" => StopReason::ToolUse,
                "length" => StopReason::MaxTokens,
                "stop" | "end_turn" => StopReason::EndTurn,
                other => {
                    warn!(finish_reason = %other, "Unknown finish_reason from OpenAI API");
                    StopReason::EndTurn
                }
            },
            None => {
                warn!("No choices in OpenAI response");
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
impl LlmProvider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn model(&self) -> &str {
        &self.model_name
    }

    fn context_window(&self) -> Option<usize> {
        self.context_window
    }

    fn max_output_tokens(&self) -> Option<usize> {
        self.max_output_tokens
    }

    fn supports_caching(&self) -> bool {
        false // OpenAI does not support prompt caching
    }

    fn supports_streaming(&self) -> bool {
        self.streaming // PROV-140: reflects the per-profile OPENAI_STREAMING flag
    }

    async fn complete(&self, messages: &[Message]) -> Result<String, ProviderError> {
        // Reuse complete_with_tools with no tools to avoid code duplication
        let response = self.complete_with_tools(messages, &[]).await?;

        // Extract text using shared helper (REFAC-013)
        Ok(extract_text_from_content(&response.content))
    }

    async fn complete_with_tools(
        &self,
        messages: &[Message],
        tools: &[OurToolDefinition],
    ) -> Result<CompletionResponse, ProviderError> {
        // Extract prompt data using shared helper (REFAC-013)
        let (preamble, prompt) = extract_prompt_data(messages);

        // Convert tools to rig format using shared helper (REFAC-013)
        let rig_tools = convert_tools_to_rig(tools)?;

        // Build and send completion request using rig's builder pattern
        let mut builder =
            CompletionRequestBuilder::new(self.completion_model.clone(), prompt).tools(rig_tools);

        // PROV-171: cap the output only with a declared limit; the session path
        // (`ProviderManager::get_openai`) refuses to build without one.
        if let Some(max_output_tokens) = self.max_output_tokens {
            builder = builder.max_tokens(max_output_tokens as u64);
        }

        if let Some(preamble_text) = preamble {
            builder = builder.preamble(preamble_text);
        }

        // Anti-ramble sampling for local endpoints — same values as the
        // agent path (see `create_rig_agent`).
        if self.is_local_endpoint() {
            builder = builder
                .temperature(ANTI_RAMBLE_TEMPERATURE)
                .additional_params(anti_ramble_additional_params());
        }

        // Send request and get response
        let response = builder
            .send()
            .await
            .map_err(|e| ProviderError::api("openai", format!("Rig completion failed: {e}")))?;

        // Convert rig response to our CompletionResponse format
        self.rig_response_to_completion(response)
    }
}
