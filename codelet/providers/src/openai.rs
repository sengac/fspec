//! OpenAI Provider implementation using rig
//!
//! Implements the LlmProvider trait for OpenAI API communication.
//! Uses rig::providers::openai for HTTP communication.

use crate::{
    convert_assistant_content, convert_tools_to_rig, detect_credential_from_env,
    extract_prompt_data, extract_text_from_content, validate_api_key_static, CompletionResponse,
    LlmProvider, ProviderAdapter, ProviderError, StopReason,
};
use async_trait::async_trait;
use codelet_common::{Message, MessageContent};
use codelet_tools::ToolDefinition as OurToolDefinition;
use rig::completion::CompletionRequestBuilder;
use rig::providers::openai;
use tracing::warn;

/// Default OpenAI model
const DEFAULT_MODEL: &str = "gpt-4-turbo";

/// GPT-4 Turbo context window size
pub const CONTEXT_WINDOW: usize = 128_000;

/// GPT-4 Turbo max output tokens (CTX-002)
pub const MAX_OUTPUT_TOKENS: usize = 4096;

/// OpenAI Provider for OpenAI API (using rig)
#[derive(Clone)]
pub struct OpenAIProvider {
    completion_model: openai::completion::CompletionModel,
    rig_client: openai::CompletionsClient,
    model_name: String,
}

impl std::fmt::Debug for OpenAIProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAIProvider")
            .field("model", &self.model_name)
            .finish()
    }
}

impl ProviderAdapter for OpenAIProvider {
    fn provider_name(&self) -> &'static str {
        "openai"
    }
}

impl OpenAIProvider {
    /// Create a new OpenAIProvider using API key from environment
    ///
    /// Checks for API key in OPENAI_API_KEY environment variable.
    /// Model can be overridden via OPENAI_MODEL environment variable.
    ///
    /// Uses shared detect_credential_from_env() helper (REFAC-013).
    pub fn new() -> Result<Self, ProviderError> {
        // Use shared credential detection helper (REFAC-013)
        let api_key = detect_credential_from_env("openai", &["OPENAI_API_KEY"])?;

        // Allow model override via OPENAI_MODEL env var
        let model_name =
            std::env::var("OPENAI_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());

        Self::from_api_key(&api_key, &model_name)
    }

    /// Create a new OpenAIProvider with an explicit API key and model
    ///
    /// Uses shared validate_api_key_static() helper (REFAC-013).
    pub fn from_api_key(api_key: &str, model: &str) -> Result<Self, ProviderError> {
        // Use shared validation helper (REFAC-013)
        validate_api_key_static("openai", api_key)?;

        // Build rig completions client (standard OpenAI Chat Completions API)
        let rig_client = openai::CompletionsClient::builder()
            .api_key(api_key)
            .build()
            .map_err(|e| {
                ProviderError::config("openai", format!("Failed to build OpenAI client: {e}"))
            })?;

        // Create completion model using the client
        let completion_model = openai::completion::CompletionModel::new(rig_client.clone(), model);

        Ok(Self {
            completion_model,
            rig_client,
            model_name: model.to_string(),
        })
    }

    /// Get the configured rig openai::CompletionsClient
    ///
    /// This client is fully configured with:
    /// - API key authentication
    ///
    /// Use this client to create rig agents with tools.
    pub fn client(&self) -> &openai::CompletionsClient {
        &self.rig_client
    }

    /// Create a rig Agent with all 10 tools configured for this provider
    ///
    /// This method encapsulates all OpenAI-specific configuration:
    /// - Model name (gpt-4-turbo or custom)
    /// - Max tokens (4096)
    /// - All 10 tools (Read, Write, Edit, Bash, Grep, Glob, Ls, AstGrep, AstGrepRefactor, WebSearchTool)
    ///
    /// # Arguments
    /// * `preamble` - Optional system prompt/preamble for the agent
    /// * `_thinking_config` - Optional thinking configuration JSON (TOOL-010, currently unused for OpenAI)
    ///
    /// # WEB-001 Note
    /// WebSearchTool is now included as a rig tool that provides web search capabilities
    /// with Search, OpenPage, and FindInPage actions.
    ///
    /// Returns a fully configured rig::agent::Agent ready for use with RigAgent.
    pub fn create_rig_agent(
        &self,
        preamble: Option<&str>,
        _thinking_config: Option<serde_json::Value>,
    ) -> rig::agent::Agent<openai::completion::CompletionModel> {
        use codelet_tools::{
            AstGrepRefactorTool, AstGrepTool, BashTool, EditTool, GlobTool, GrepTool, LsTool,
            ReadTool, WebSearchTool, WriteTool,
        };
        use rig::client::CompletionClient;

        // Build agent with all 10 tools using rig's builder pattern (WEB-001: Added WebSearchTool)
        let mut agent_builder = self
            .rig_client
            .agent(&self.model_name)
            .max_tokens(MAX_OUTPUT_TOKENS as u64)
            .tool(ReadTool::new())
            .tool(WriteTool::new())
            .tool(EditTool::new())
            .tool(BashTool::new())
            .tool(GrepTool::new())
            .tool(GlobTool::new())
            .tool(LsTool::new())
            .tool(AstGrepTool::new())
            .tool(AstGrepRefactorTool::new())
            .tool(WebSearchTool::new()); // WEB-001: Added WebSearchTool with consistent new() pattern

        // Set preamble if provided
        if let Some(p) = preamble {
            agent_builder = agent_builder.preamble(p);
        }

        agent_builder.build()
    }

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

    fn context_window(&self) -> usize {
        CONTEXT_WINDOW
    }

    fn max_output_tokens(&self) -> usize {
        MAX_OUTPUT_TOKENS
    }

    fn supports_caching(&self) -> bool {
        false // OpenAI does not support prompt caching
    }

    fn supports_streaming(&self) -> bool {
        true // Streaming support via rig
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
        let rig_tools = convert_tools_to_rig(tools);

        // Build and send completion request using rig's builder pattern
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
            .map_err(|e| ProviderError::api("openai", format!("Rig completion failed: {e}")))?;

        // Convert rig response to our CompletionResponse format
        self.rig_response_to_completion(response)
    }
}
