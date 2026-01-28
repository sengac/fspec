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

/// Gemini 2.0 Flash max output tokens (CTX-002)
pub const MAX_OUTPUT_TOKENS: usize = 8192;

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

    /// Create a rig Agent with all tools configured for this provider
    ///
    /// Uses provider-specific tool facades (TOOL-001, TOOL-003):
    /// - `google_web_search` - Gemini-native web search with flat schema
    /// - `web_fetch` - Gemini-native URL fetching with flat schema
    /// - `read_file` - Gemini-native file reading with flat schema
    /// - `write_file` - Gemini-native file writing with flat schema
    /// - `replace` - Gemini-native text replacement with flat schema
    ///
    /// # Arguments
    /// * `preamble` - Optional system prompt/preamble for the agent
    /// * `thinking_config` - Optional thinking configuration JSON from ThinkingConfigFacade (TOOL-010).
    ///   Expected format: `{"thinkingConfig": {"includeThoughts": true, "thinkingLevel": "high"}}`
    ///   for Gemini 3, or `{"thinkingConfig": {"includeThoughts": true, "thinkingBudget": 8192}}`
    ///   for Gemini 2.5
    pub fn create_rig_agent(
        &self,
        preamble: Option<&str>,
        thinking_config: Option<serde_json::Value>,
    ) -> rig::agent::Agent<gemini::completion::CompletionModel> {
        use codelet_tools::facade::{
            build_gemini_system_prompt, BashToolFacadeWrapper, FacadeToolWrapper,
            FileToolFacadeWrapper, gemini_fspec_tool, GeminiGlobFacade, GeminiGoogleWebSearchFacade,
            GeminiListDirectoryFacade, GeminiReadFileFacade, GeminiReplaceFacade,
            GeminiRunShellCommandFacade, GeminiSearchFileContentFacade, GeminiWebFetchFacade,
            GeminiWriteFileFacade, LsToolFacadeWrapper, SearchToolFacadeWrapper,
        };
        use codelet_tools::{AstGrepRefactorTool, AstGrepTool};
        use std::sync::Arc;

        // Create Gemini-specific web search facades (TOOL-001)
        // These provide Gemini-native tool names and flat schemas
        let google_web_search = FacadeToolWrapper::new(Arc::new(GeminiGoogleWebSearchFacade));
        let web_fetch = FacadeToolWrapper::new(Arc::new(GeminiWebFetchFacade));

        // Create Gemini-specific file operation facades (TOOL-003)
        // These provide Gemini-native tool names: read_file, write_file, replace
        let read_file = FileToolFacadeWrapper::new(Arc::new(GeminiReadFileFacade));
        let write_file = FileToolFacadeWrapper::new(Arc::new(GeminiWriteFileFacade));
        let replace = FileToolFacadeWrapper::new(Arc::new(GeminiReplaceFacade));

        // Create Gemini-specific bash facade (TOOL-004)
        // Provides Gemini-native tool name: run_shell_command
        let run_shell_command = BashToolFacadeWrapper::new(Arc::new(GeminiRunShellCommandFacade));

        // Create Gemini-specific search facades (TOOL-005)
        // Provides Gemini-native tool names: search_file_content, find_files
        let search_file_content =
            SearchToolFacadeWrapper::new(Arc::new(GeminiSearchFileContentFacade));
        let find_files = SearchToolFacadeWrapper::new(Arc::new(GeminiGlobFacade));

        // Create Gemini-specific directory listing facade (TOOL-006)
        // Provides Gemini-native tool name: list_directory
        let list_directory = LsToolFacadeWrapper::new(Arc::new(GeminiListDirectoryFacade));

        // Build agent with all tools using rig's builder pattern
        // TOOL-001: Use facade wrappers for web search instead of generic WebSearchTool
        // TOOL-003: Use facade wrappers for file operations instead of raw tools
        // TOOL-004: Use facade wrapper for bash instead of raw BashTool
        // TOOL-005: Use facade wrappers for search instead of raw GrepTool/GlobTool
        // TOOL-006: Use facade wrapper for directory listing instead of raw LsTool
        let mut agent_builder = self
            .rig_client
            .agent(&self.model_name)
            .max_tokens(MAX_OUTPUT_TOKENS as u64)
            .tool(read_file) // TOOL-003: Gemini-native read_file
            .tool(write_file) // TOOL-003: Gemini-native write_file
            .tool(replace) // TOOL-003: Gemini-native replace
            .tool(run_shell_command) // TOOL-004: Gemini-native run_shell_command
            .tool(search_file_content) // TOOL-005: Gemini-native search_file_content
            .tool(find_files) // TOOL-005: Gemini-native find_files
            .tool(list_directory) // TOOL-006: Gemini-native list_directory
            .tool(AstGrepTool::new())
            .tool(AstGrepRefactorTool::new())
            .tool(gemini_fspec_tool()) // FspecTool for ACDD workflow management
            .tool(google_web_search) // TOOL-001: Gemini-native google_web_search
            .tool(web_fetch); // TOOL-001: Gemini-native web_fetch

        // Build complete system prompt using model-aware builder
        // This combines:
        // 1. Base Gemini system prompt (from opencode) with examples showing simple questions → simple answers
        // 2. Gemini 3 specific instruction (from gemini-cli) if model is Gemini 3
        // 3. User-provided preamble (e.g., AGENTS.md content)
        let system_prompt = build_gemini_system_prompt(&self.model_name, preamble);
        agent_builder = agent_builder.preamble(&system_prompt);

        // TOOL-010: Apply generation config for Gemini models
        // Based on Gemini CLI and opencode research:
        // - temperature: 1.0 (strongly recommended by Google, other values cause looping)
        // - topP: 0.95 (standard Gemini value)
        // - topK: 64 (standard Gemini value)
        // - thinkingConfig: required for Gemini 3 models
        //
        // The facade returns {"thinkingConfig": {...}} but rig expects {"generationConfig": {...}}
        let thinking_cfg = thinking_config
            .as_ref()
            .and_then(|config| config.get("thinkingConfig").cloned())
            .or_else(|| {
                // Provide default thinking config for Gemini 3 models
                // Gemini 3 uses thinkingLevel enum instead of thinkingBudget
                // Note: You cannot disable thinking for Gemini 3 Pro
                if self.model_name.contains("gemini-3") {
                    tracing::debug!(
                        "Using default thinking level HIGH for Gemini 3 model: {}",
                        self.model_name
                    );
                    Some(serde_json::json!({
                        "includeThoughts": true,
                        "thinkingLevel": "high"
                    }))
                } else {
                    None
                }
            });

        // ALWAYS apply generation config with proper defaults for ALL Gemini models
        // This is critical for Gemini 3 which requires specific parameter values
        let mut gen_config = serde_json::json!({
            "temperature": 1.0,
            "topP": 0.95,
            "topK": 64
        });

        // Add thinking config if present (Gemini 3 or explicitly provided)
        if let Some(thinking_cfg) = thinking_cfg {
            gen_config["thinkingConfig"] = thinking_cfg;
        }

        let generation_config = serde_json::json!({
            "generationConfig": gen_config
        });
        tracing::debug!("Applying Gemini generation config: {:?}", generation_config);
        agent_builder = agent_builder.additional_params(generation_config);

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
