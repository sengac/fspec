//! Gemini Provider implementation using rig
//!
//! Implements the LlmProvider trait for Google Gemini API communication.
//! Uses rig::providers::gemini for HTTP communication.

use crate::{
    convert_assistant_content, convert_tools_to_rig, detect_credential_from_env,
    extract_prompt_data, extract_text_from_content, validate_api_key_static, CompletionResponse,
    LlmProvider, ProviderAdapter, ProviderError, StopReason,
};
use async_trait::async_trait;
use codelet_common::{Message, MessageContent};
use codelet_tools::ToolDefinition as OurToolDefinition;
use rig::client::CompletionClient;
use rig::completion::CompletionRequestBuilder;
use rig::providers::gemini;
use tracing::warn;

/// Default Gemini model
const DEFAULT_MODEL: &str = "gemini-2.0-flash-exp";

/// Gemini 2.0 Flash context window size
pub const CONTEXT_WINDOW: usize = 1_000_000;

/// Gemini 2.0 Flash max output tokens
const MAX_OUTPUT_TOKENS: usize = 8192;

/// Gemini Provider for Google Generative AI API (using rig)
#[derive(Clone, Debug)]
pub struct GeminiProvider {
    completion_model: gemini::completion::CompletionModel,
    rig_client: gemini::Client,
    model_name: String,
}

impl ProviderAdapter for GeminiProvider {
    fn provider_name(&self) -> &'static str {
        "gemini"
    }
}

impl GeminiProvider {
    /// Create a new GeminiProvider using API key from environment
    ///
    /// Checks for API key in GOOGLE_GENERATIVE_AI_API_KEY environment variable.
    /// Model can be overridden via GEMINI_MODEL environment variable.
    ///
    /// Uses shared detect_credential_from_env() helper (REFAC-013).
    pub fn new() -> Result<Self, ProviderError> {
        // Use shared credential detection helper (REFAC-013)
        let api_key = detect_credential_from_env("gemini", &["GOOGLE_GENERATIVE_AI_API_KEY"])?;

        // Allow model override via GEMINI_MODEL env var
        let model_name =
            std::env::var("GEMINI_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());

        Self::from_api_key(&api_key, &model_name)
    }

    /// Create a new GeminiProvider with an explicit API key and model
    ///
    /// Uses shared validate_api_key_static() helper (REFAC-013).
    pub fn from_api_key(api_key: &str, model: &str) -> Result<Self, ProviderError> {
        // Use shared validation helper (REFAC-013)
        validate_api_key_static("gemini", api_key)?;

        // Build rig gemini client
        let rig_client = gemini::Client::new(api_key).map_err(|e| {
            ProviderError::config("gemini", format!("Failed to build Gemini client: {e}"))
        })?;

        // Create completion model using the client
        let completion_model = rig_client.completion_model(model);

        Ok(Self {
            completion_model,
            rig_client,
            model_name: model.to_string(),
        })
    }

    /// Get the configured rig gemini::Client
    pub fn client(&self) -> &gemini::Client {
        &self.rig_client
    }

    /// Create a rig Agent with all 8 tools configured for this provider
    ///
    /// # Arguments
    /// * `preamble` - Optional system prompt/preamble for the agent
    pub fn create_rig_agent(
        &self,
        preamble: Option<&str>,
    ) -> rig::agent::Agent<gemini::completion::CompletionModel> {
        use codelet_tools::{
            AstGrepTool, BashTool, EditTool, GlobTool, GrepTool, LsTool, ReadTool, WebSearchTool,
            WriteTool,
        };

        // Build agent with all 9 tools using rig's builder pattern (WEB-001: Added WebSearchTool)
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
        response: rig::completion::CompletionResponse<
            gemini::completion::gemini_api_types::GenerateContentResponse,
        >,
    ) -> Result<CompletionResponse, ProviderError> {
        // Convert rig AssistantContent to our ContentPart format using shared helper (REFAC-013)
        let content_parts = convert_assistant_content(response.choice, "gemini")?;

        // Map Gemini's finish_reason to our StopReason enum (provider-specific)
        use gemini::completion::gemini_api_types::FinishReason;
        let stop_reason = match response.raw_response.candidates.first() {
            Some(candidate) => match &candidate.finish_reason {
                Some(FinishReason::Stop) => StopReason::EndTurn,
                Some(FinishReason::MaxTokens) => StopReason::MaxTokens,
                Some(FinishReason::Safety) => StopReason::EndTurn,
                Some(FinishReason::Recitation) => StopReason::EndTurn,
                Some(FinishReason::FinishReasonUnspecified) => StopReason::EndTurn,
                Some(other) => {
                    warn!(finish_reason = ?other, "Unknown finish_reason from Gemini API");
                    StopReason::EndTurn
                }
                None => {
                    warn!("No finish_reason in Gemini candidate");
                    StopReason::EndTurn
                }
            },
            None => {
                warn!("No candidates in Gemini response");
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
impl LlmProvider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
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
        false // Gemini does not support prompt caching yet
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
            .map_err(|e| ProviderError::api("gemini", format!("Rig completion failed: {e}")))?;

        // Convert rig response to our CompletionResponse format
        self.rig_response_to_completion(response)
    }
}
