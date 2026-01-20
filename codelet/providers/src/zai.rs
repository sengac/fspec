//! Z.AI Provider implementation using rig
//!
//! Implements the LlmProvider trait for Z.AI GLM API communication.
//! Uses rig's OpenAI-compatible client with custom base_url.
//!
//! Z.AI supports two API endpoints:
//! - Normal API: https://api.z.ai/api/paas/v4 (uses ZAI_API_KEY)
//! - Coding Plan API: https://api.z.ai/api/coding/paas/v4 (uses ZAI_PLAN_API_KEY)

use crate::{
    convert_assistant_content, convert_tools_to_rig,
    extract_prompt_data, extract_text_from_content, validate_api_key_static, CompletionResponse,
    LlmProvider, ProviderAdapter, ProviderError, StopReason,
};
use async_trait::async_trait;
use codelet_common::{Message, MessageContent};
use codelet_tools::ToolDefinition as OurToolDefinition;
use rig::client::CompletionClient;
use rig::completion::CompletionRequestBuilder;
use rig::providers::openai;
use tracing::warn;

/// Z.AI normal API base URL (OpenAI-compatible)
const ZAI_API_BASE_URL: &str = "https://api.z.ai/api/paas/v4";

/// Z.AI coding plan API base URL (OpenAI-compatible)
const ZAI_PLAN_API_BASE_URL: &str = "https://api.z.ai/api/coding/paas/v4";

/// Default Z.AI model
const DEFAULT_MODEL: &str = "glm-4.7";

/// Z.AI GLM context window size (GLM-4.7)
pub const CONTEXT_WINDOW: usize = 128_000;

/// Z.AI GLM max output tokens
pub const MAX_OUTPUT_TOKENS: usize = 8192;

/// Z.AI Provider for GLM models (using rig's OpenAI-compatible client)
#[derive(Clone)]
pub struct ZAIProvider {
    completion_model: openai::completion::CompletionModel,
    rig_client: openai::CompletionsClient,
    model_name: String,
    /// Whether using the coding plan endpoint
    is_plan_endpoint: bool,
}

impl std::fmt::Debug for ZAIProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ZAIProvider")
            .field("model", &self.model_name)
            .field("is_plan_endpoint", &self.is_plan_endpoint)
            .finish()
    }
}

impl ProviderAdapter for ZAIProvider {
    fn provider_name(&self) -> &'static str {
        "zai"
    }
}

impl ZAIProvider {
    /// Create a new ZAIProvider using API key from environment
    ///
    /// Checks for API key in order:
    /// 1. ZAI_PLAN_API_KEY - uses coding plan endpoint (https://api.z.ai/api/coding/paas/v4)
    /// 2. ZAI_API_KEY - uses normal endpoint (https://api.z.ai/api/paas/v4)
    ///
    /// Model can be overridden via ZAI_MODEL environment variable.
    pub fn new() -> Result<Self, ProviderError> {
        Self::new_with_model(None)
    }

    /// Create a new ZAIProvider with optional custom model
    ///
    /// If model is None, uses DEFAULT_MODEL (glm-4.7).
    /// Automatically detects which endpoint to use based on available env var.
    pub fn new_with_model(model: Option<&str>) -> Result<Self, ProviderError> {
        // Check for plan API key first, then normal API key
        let (api_key, is_plan) = if let Ok(key) = std::env::var("ZAI_PLAN_API_KEY") {
            if !key.is_empty() {
                (key, true)
            } else if let Ok(key) = std::env::var("ZAI_API_KEY") {
                (key, false)
            } else {
                return Err(ProviderError::auth(
                    "zai",
                    "Missing ZAI_API_KEY or ZAI_PLAN_API_KEY environment variable",
                ));
            }
        } else if let Ok(key) = std::env::var("ZAI_API_KEY") {
            (key, false)
        } else {
            return Err(ProviderError::auth(
                "zai",
                "Missing ZAI_API_KEY or ZAI_PLAN_API_KEY environment variable",
            ));
        };

        // Allow model override via ZAI_MODEL env var or parameter
        let model_name = model
            .map(ToString::to_string)
            .or_else(|| std::env::var("ZAI_MODEL").ok())
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());

        Self::from_api_key_with_endpoint(&api_key, &model_name, is_plan)
    }

    /// Create a new ZAIProvider with an explicit API key and model (uses normal endpoint)
    pub fn from_api_key(api_key: &str, model: &str) -> Result<Self, ProviderError> {
        Self::from_api_key_with_endpoint(api_key, model, false)
    }

    /// Create a new ZAIProvider with explicit API key, model, and endpoint selection
    ///
    /// # Arguments
    /// * `api_key` - The Z.AI API key
    /// * `model` - The model name (e.g., "glm-4.7")
    /// * `use_plan_endpoint` - If true, uses coding plan endpoint; otherwise uses normal endpoint
    pub fn from_api_key_with_endpoint(
        api_key: &str,
        model: &str,
        use_plan_endpoint: bool,
    ) -> Result<Self, ProviderError> {
        // Use shared validation helper
        validate_api_key_static("zai", api_key)?;

        // Select base URL based on endpoint type
        let base_url = if use_plan_endpoint {
            ZAI_PLAN_API_BASE_URL
        } else {
            ZAI_API_BASE_URL
        };

        // Build rig OpenAI-compatible client with appropriate Z.AI base URL
        let rig_client = openai::CompletionsClient::builder()
            .api_key(api_key)
            .base_url(base_url)
            .build()
            .map_err(|e| {
                ProviderError::config("zai", format!("Failed to build Z.AI client: {e}"))
            })?;

        // Create completion model using the client
        let completion_model = openai::completion::CompletionModel::new(rig_client.clone(), model);

        Ok(Self {
            completion_model,
            rig_client,
            model_name: model.to_string(),
            is_plan_endpoint: use_plan_endpoint,
        })
    }

    /// Check if this provider is using the coding plan endpoint
    pub fn is_plan_endpoint(&self) -> bool {
        self.is_plan_endpoint
    }

    /// Get the base URL being used
    pub fn base_url(&self) -> &'static str {
        if self.is_plan_endpoint {
            ZAI_PLAN_API_BASE_URL
        } else {
            ZAI_API_BASE_URL
        }
    }

    /// Get the configured rig client
    pub fn client(&self) -> &openai::CompletionsClient {
        &self.rig_client
    }

    /// Check if the model supports reasoning/thinking mode
    ///
    /// Based on Z.AI documentation, these models support thinking:
    /// - glm-4-plus, glm-4.7, glm-4.6, glm-4.5, glm-4.5-air, glm-4.5-x, glm-4.5-airx, glm-4.5-flash
    ///
    /// Vision models (glm-4.6v, glm-4.5v) do NOT support reasoning
    pub fn supports_reasoning(&self) -> bool {
        let model = &self.model_name;
        // Vision models don't support reasoning
        if model.contains("glm-4.6v") || model.contains("glm-4.5v") {
            return false;
        }
        model.contains("glm-4-plus")
            || model.contains("glm-4.7")
            || model.contains("glm-4.6")
            || model.contains("glm-4.5")
    }

    /// Create a rig Agent with all tools configured for this provider
    ///
    /// Uses Z.AI/GLM-specific tool facades for optimal tool calling behavior.
    /// GLM models work best with snake_case tool names and flat JSON schemas.
    ///
    /// # Arguments
    /// * `preamble` - Optional system prompt/preamble for the agent
    /// * `thinking_config` - Optional thinking configuration JSON (currently unused for Z.AI/GLM models)
    ///   Note: GLM models handle reasoning internally and don't use the same thinking config format
    ///   as Claude or Gemini. The reasoning capability is model-dependent.
    pub fn create_rig_agent(
        &self,
        preamble: Option<&str>,
        _thinking_config: Option<serde_json::Value>,
    ) -> rig::agent::Agent<openai::completion::CompletionModel> {
        use codelet_tools::facade::{
            BashToolFacadeWrapper, FileToolFacadeWrapper, LsToolFacadeWrapper,
            SearchToolFacadeWrapper, ZAIEditFileFacade, ZAIFindFilesFacade, ZAIGrepFilesFacade,
            ZAIListDirFacade, ZAIReadFileFacade, ZAIRunCommandFacade, ZAIWriteFileFacade,
        };
        use codelet_tools::{AstGrepRefactorTool, AstGrepTool, WebSearchTool};
        use std::sync::Arc;

        // Create Z.AI/GLM-specific tool facades (PROV-004)
        // These provide GLM-native tool names and flat schemas that GLM understands
        
        // File operation facades
        let read_file = FileToolFacadeWrapper::new(Arc::new(ZAIReadFileFacade));
        let write_file = FileToolFacadeWrapper::new(Arc::new(ZAIWriteFileFacade));
        let edit_file = FileToolFacadeWrapper::new(Arc::new(ZAIEditFileFacade));

        // Shell command facade
        let run_command = BashToolFacadeWrapper::new(Arc::new(ZAIRunCommandFacade));

        // Search facades
        let grep_files = SearchToolFacadeWrapper::new(Arc::new(ZAIGrepFilesFacade));
        let find_files = SearchToolFacadeWrapper::new(Arc::new(ZAIFindFilesFacade));

        // Directory listing facade
        let list_dir = LsToolFacadeWrapper::new(Arc::new(ZAIListDirFacade));

        // Build agent with Z.AI-optimized tools using rig's builder pattern
        let mut agent_builder = self
            .rig_client
            .agent(&self.model_name)
            .max_tokens(MAX_OUTPUT_TOKENS as u64)
            .tool(read_file) // Z.AI-native read_file
            .tool(write_file) // Z.AI-native write_file
            .tool(edit_file) // Z.AI-native edit_file
            .tool(run_command) // Z.AI-native run_command
            .tool(grep_files) // Z.AI-native grep_files
            .tool(find_files) // Z.AI-native find_files
            .tool(list_dir) // Z.AI-native list_dir
            .tool(AstGrepTool::new())
            .tool(AstGrepRefactorTool::new())
            .tool(WebSearchTool::new());

        // Set preamble if provided
        if let Some(preamble_text) = preamble {
            agent_builder = agent_builder.preamble(preamble_text);
        }

        // PROV-002: Apply generation config for GLM models
        // Based on opencode research, GLM-4.6/4.7 models require specific temperature and topP values
        // - temperature: 1.0 (required for optimal performance, matches Gemini)
        // - topP: 0.95 (standard value for reasoning models)
        //
        // Note: GLM models do NOT use the same thinking config format as Claude or Gemini.
        // Their reasoning capability is built into the model and enabled by default for
        // reasoning-capable models (glm-4.7, glm-4.6, etc.)
        let generation_config = serde_json::json!({
            "temperature": 1.0,
            "top_p": 0.95
        });
        agent_builder = agent_builder.additional_params(generation_config);

        agent_builder.build()
    }

    /// Convert rig response to our CompletionResponse format
    fn rig_response_to_completion(
        &self,
        response: rig::completion::CompletionResponse<openai::completion::CompletionResponse>,
    ) -> Result<CompletionResponse, ProviderError> {
        // Convert rig AssistantContent to our ContentPart format
        let content_parts = convert_assistant_content(response.choice, "zai")?;

        // Map finish_reason to our StopReason enum (same as OpenAI)
        let stop_reason = match response.raw_response.choices.first() {
            Some(choice) => match choice.finish_reason.as_str() {
                "tool_calls" => StopReason::ToolUse,
                "length" => StopReason::MaxTokens,
                "stop" | "end_turn" => StopReason::EndTurn,
                other => {
                    warn!(finish_reason = %other, "Unknown finish_reason from Z.AI API");
                    StopReason::EndTurn
                }
            },
            None => {
                warn!("No choices in Z.AI response");
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
impl LlmProvider for ZAIProvider {
    fn name(&self) -> &str {
        "zai"
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
        // Z.AI supports prompt caching via cached_tokens in usage
        true
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    async fn complete(&self, messages: &[Message]) -> Result<String, ProviderError> {
        // Reuse complete_with_tools with no tools
        let response = self.complete_with_tools(messages, &[]).await?;
        Ok(extract_text_from_content(&response.content))
    }

    async fn complete_with_tools(
        &self,
        messages: &[Message],
        tools: &[OurToolDefinition],
    ) -> Result<CompletionResponse, ProviderError> {
        // Extract prompt data
        let (preamble, prompt) = extract_prompt_data(messages);

        // Convert tools to rig format
        let rig_tools = convert_tools_to_rig(tools);

        // Build and send completion request
        let mut builder = CompletionRequestBuilder::new(self.completion_model.clone(), prompt)
            .max_tokens(MAX_OUTPUT_TOKENS as u64)
            .tools(rig_tools);

        if let Some(preamble_text) = preamble {
            builder = builder.preamble(preamble_text);
        }

        // Send request and get response
        let response = builder
            .send()
            .await
            .map_err(|e| ProviderError::api("zai", format!("Rig completion failed: {e}")))?;

        // Convert rig response to our CompletionResponse format
        self.rig_response_to_completion(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_model() {
        assert_eq!(DEFAULT_MODEL, "glm-4.7");
    }

    #[test]
    fn test_base_urls() {
        assert_eq!(ZAI_API_BASE_URL, "https://api.z.ai/api/paas/v4");
        assert_eq!(ZAI_PLAN_API_BASE_URL, "https://api.z.ai/api/coding/paas/v4");
    }

    #[test]
    fn test_supports_reasoning() {
        // Mock provider for testing reasoning support detection
        // In actual use, this would need a real API key
        let reasoning_models = vec![
            "glm-4-plus",
            "glm-4.7",
            "glm-4.6",
            "glm-4.5",
            "glm-4.5-air",
            "glm-4.5-x",
        ];

        for model in reasoning_models {
            assert!(
                model.contains("glm-4-plus")
                    || model.contains("glm-4.7")
                    || model.contains("glm-4.6")
                    || model.contains("glm-4.5"),
                "{} should support reasoning",
                model
            );
        }
    }
}
