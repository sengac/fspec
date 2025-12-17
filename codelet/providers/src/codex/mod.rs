//! Codex Provider implementation using rig
//!
//! Implements the LlmProvider trait for Codex (ChatGPT backend API) communication.
//! Uses OAuth authentication via ~/.codex/auth.json or macOS keychain.

use crate::{
    convert_assistant_content, convert_tools_to_rig, extract_prompt_data,
    extract_text_from_content, validate_api_key_static, CompletionResponse, LlmProvider,
    ProviderAdapter, ProviderError, StopReason,
};
use async_trait::async_trait;
use codelet_common::{Message, MessageContent};
use codelet_tools::ToolDefinition as OurToolDefinition;
use rig::completion::CompletionRequestBuilder;
use rig::providers::openai;
use tracing::warn;

pub mod codex_auth;

/// Default Codex model
const DEFAULT_MODEL: &str = "gpt-5.1-codex";

/// GPT-5.1 Codex context window size (272K tokens)
pub const CONTEXT_WINDOW: usize = 272_000;

/// GPT-5.1 Codex max output tokens (assumption: same as GPT-4)
const MAX_OUTPUT_TOKENS: usize = 4096;

/// Codex Provider for ChatGPT Backend API (using rig with OAuth)
#[derive(Clone)]
pub struct CodexProvider {
    completion_model: openai::completion::CompletionModel,
    rig_client: openai::CompletionsClient,
    model_name: String,
}

impl std::fmt::Debug for CodexProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodexProvider")
            .field("model", &self.model_name)
            .finish()
    }
}

impl ProviderAdapter for CodexProvider {
    fn provider_name(&self) -> &'static str {
        "codex"
    }
}

impl CodexProvider {
    /// Create a new CodexProvider using credentials from ~/.codex/auth.json
    ///
    /// Reads credentials from:
    /// 1. macOS keychain (if on macOS)
    /// 2. ~/.codex/auth.json file
    /// 3. $CODEX_HOME/auth.json if CODEX_HOME is set
    ///
    /// If cached OPENAI_API_KEY exists in auth.json, uses it directly.
    /// Otherwise, performs OAuth refresh and token exchange.
    ///
    /// Model can be overridden via CODEX_MODEL environment variable.
    ///
    /// Note: CodexProvider uses OAuth authentication via codex_auth module,
    /// which is different from the standard detect_credential_from_env() pattern.
    pub fn new() -> Result<Self, ProviderError> {
        // Get API key from Codex auth (synchronous version using reqwest blocking)
        let api_key = codex_auth::get_codex_api_key_sync()
            .map_err(|e| ProviderError::auth("codex", format!("{e}")))?;

        // Allow model override via CODEX_MODEL env var
        let model_name = std::env::var("CODEX_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());

        Self::from_api_key(&api_key, &model_name)
    }

    /// Create a new CodexProvider with an explicit API key and model
    ///
    /// Uses shared validate_api_key_static() helper (REFAC-013).
    pub fn from_api_key(api_key: &str, model: &str) -> Result<Self, ProviderError> {
        // Use shared validation helper (REFAC-013)
        validate_api_key_static("codex", api_key)?;

        // Build rig completions client (standard OpenAI Chat Completions API)
        // Note: We can optionally use custom base URL for ChatGPT backend:
        // .base_url("https://chatgpt.com/backend-api/codex")
        // But for now, prefer standard OpenAI API via token exchange
        let rig_client = openai::CompletionsClient::builder()
            .api_key(api_key)
            .build()
            .map_err(|e| {
                ProviderError::config("codex", format!("Failed to build Codex client: {e}"))
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
    pub fn client(&self) -> &openai::CompletionsClient {
        &self.rig_client
    }

    /// Create a rig Agent with all 9 tools configured for this provider (WEB-001: Added WebSearchTool)
    ///
    /// This method encapsulates all Codex-specific configuration:
    /// - Model name (GPT or compatible)
    /// - Max tokens (4096)
    /// - All 9 tools (Read, Write, Edit, Bash, Grep, Glob, Ls, AstGrep, WebSearchTool)
    ///
    /// # Arguments
    /// * `preamble` - Optional system prompt/preamble for the agent
    ///
    /// Returns a fully configured rig::agent::Agent ready for use with RigAgent.
    pub fn create_rig_agent(
        &self,
        preamble: Option<&str>,
    ) -> rig::agent::Agent<openai::completion::CompletionModel> {
        use codelet_tools::{
            AstGrepTool, BashTool, EditTool, GlobTool, GrepTool, LsTool, ReadTool, WebSearchTool,
            WriteTool,
        };
        use rig::client::CompletionClient;

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
        response: rig::completion::CompletionResponse<openai::completion::CompletionResponse>,
    ) -> Result<CompletionResponse, ProviderError> {
        // Convert rig AssistantContent to our ContentPart format using shared helper (REFAC-013)
        let content_parts = convert_assistant_content(response.choice, "codex")?;

        // Map OpenAI's finish_reason to our StopReason enum (provider-specific)
        let stop_reason = match response.raw_response.choices.first() {
            Some(choice) => match choice.finish_reason.as_str() {
                "tool_calls" => StopReason::ToolUse,
                "length" => StopReason::MaxTokens,
                "stop" | "end_turn" => StopReason::EndTurn,
                other => {
                    warn!(finish_reason = %other, "Unknown finish_reason from Codex API");
                    StopReason::EndTurn
                }
            },
            None => {
                warn!("No choices in Codex response");
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
        false // Codex does not support prompt caching
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
            .map_err(|e| ProviderError::api("codex", format!("Rig completion failed: {e}")))?;

        // Convert rig response to our CompletionResponse format
        self.rig_response_to_completion(response)
    }
}
