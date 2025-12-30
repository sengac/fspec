//! Anthropic Claude Provider implementation using rig
//!
//! Implements the LlmProvider trait for Claude API communication.
//! Uses rig::providers::anthropic for HTTP communication.
//!
//! # TOOL-008: Uses SystemPromptFacade
//! This module uses the SystemPromptFacade from codelet_tools for provider-specific
//! system prompt formatting, eliminating duplicate code.

use crate::{
    convert_assistant_content, convert_tools_to_rig, detect_credential_from_env,
    extract_text_from_content, validate_api_key_static, CompletionResponse, LlmProvider,
    ProviderAdapter, ProviderError, StopReason,
};
use async_trait::async_trait;
use codelet_common::{Message, MessageContent, MessageRole};
// TOOL-008: Import CLAUDE_CODE_PROMPT_PREFIX from facade (single source of truth)
use codelet_tools::facade::CLAUDE_CODE_PROMPT_PREFIX;
use codelet_tools::ToolDefinition as OurToolDefinition;
use reqwest::header::{HeaderValue, AUTHORIZATION};
use rig::completion::CompletionRequestBuilder;
use rig::providers::anthropic;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::warn;

/// Default Claude model
const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";

/// Claude Sonnet 4 context window size
pub const CONTEXT_WINDOW: usize = 200_000;

/// Claude Sonnet 4 max output tokens (CTX-002)
pub const MAX_OUTPUT_TOKENS: usize = 8192;

// TOOL-008: CLAUDE_CODE_PROMPT_PREFIX is now imported from codelet_tools::facade
// (single source of truth - see imports at top)

/// Anthropic beta features header for OAuth and prompt caching
const ANTHROPIC_BETA_HEADER: &str =
    "prompt-caching-2024-07-31,oauth-2025-04-20,interleaved-thinking-2025-05-14,tool-examples-2025-10-29";

/// Cache control metadata for Anthropic prompt caching (CLI-017)
///
/// Used to mark content blocks for caching with `type: ephemeral`.
/// The ephemeral cache type allows Anthropic to cache the content
/// for the duration of the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheControl {
    #[serde(rename = "type")]
    pub cache_type: String,
}

impl CacheControl {
    /// Create an ephemeral cache control marker
    pub fn ephemeral() -> Self {
        Self {
            cache_type: "ephemeral".to_string(),
        }
    }
}

// TOOL-008: build_cached_system_prompt has been removed
// Use codelet_tools::facade::select_claude_facade(is_oauth).format_for_api(preamble) instead
// Or use crate::caching_client::transform_system_prompt(preamble, is_oauth)

/// Authentication mode for Claude API
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMode {
    /// Standard API key authentication (x-api-key header)
    ApiKey,
    /// OAuth token authentication (Authorization: Bearer header)
    OAuth,
}

/// Claude Provider for Anthropic API (using rig)
#[derive(Clone)]
pub struct ClaudeProvider {
    completion_model: anthropic::completion::CompletionModel,
    rig_client: anthropic::Client,
    auth_mode: AuthMode,
    /// MODEL-001: Store model name for dynamic model selection
    model_name: String,
}

impl std::fmt::Debug for ClaudeProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClaudeProvider")
            .field("model", &self.model_name)
            .field("auth_mode", &self.auth_mode)
            .finish()
    }
}

impl ProviderAdapter for ClaudeProvider {
    fn provider_name(&self) -> &'static str {
        "claude"
    }
}

impl ClaudeProvider {
    /// Create a new ClaudeProvider using API key from environment
    ///
    /// Checks for API key in order of preference:
    /// 1. ANTHROPIC_API_KEY (uses standard x-api-key header)
    /// 2. CLAUDE_CODE_OAUTH_TOKEN (uses Bearer auth with special headers)
    ///
    /// Uses shared detect_credential_from_env() helper (REFAC-013).
    pub fn new() -> Result<Self, ProviderError> {
        Self::new_with_model(None)
    }

    /// MODEL-001: Create a new ClaudeProvider with optional custom model
    ///
    /// If model is None, uses DEFAULT_MODEL.
    pub fn new_with_model(model: Option<&str>) -> Result<Self, ProviderError> {
        let model_name = model.unwrap_or(DEFAULT_MODEL);

        // Check for API key first (takes precedence) using shared helper
        if let Ok(api_key) = detect_credential_from_env("claude", &["ANTHROPIC_API_KEY"]) {
            return Self::from_api_key_with_mode_and_model(&api_key, AuthMode::ApiKey, model_name);
        }

        // Fall back to OAuth token using shared helper
        if let Ok(oauth_token) = detect_credential_from_env("claude", &["CLAUDE_CODE_OAUTH_TOKEN"])
        {
            return Self::from_api_key_with_mode_and_model(&oauth_token, AuthMode::OAuth, model_name);
        }

        Err(ProviderError::auth(
            "claude",
            "No API key found. Set ANTHROPIC_API_KEY or CLAUDE_CODE_OAUTH_TOKEN environment variable",
        ))
    }

    /// Create a new ClaudeProvider with an explicit API key (defaults to ApiKey mode)
    pub fn from_api_key(api_key: &str) -> Result<Self, ProviderError> {
        Self::from_api_key_with_mode(api_key, AuthMode::ApiKey)
    }

    /// MODEL-001: Create a new ClaudeProvider with explicit model
    ///
    /// Uses the specified model instead of DEFAULT_MODEL.
    pub fn from_api_key_with_model(api_key: &str, model: &str) -> Result<Self, ProviderError> {
        Self::from_api_key_with_mode_and_model(api_key, AuthMode::ApiKey, model)
    }

    /// Create a new ClaudeProvider with an explicit API key and auth mode
    ///
    /// Uses shared validate_api_key_static() helper (REFAC-013).
    pub fn from_api_key_with_mode(
        api_key: &str,
        auth_mode: AuthMode,
    ) -> Result<Self, ProviderError> {
        Self::from_api_key_with_mode_and_model(api_key, auth_mode, DEFAULT_MODEL)
    }

    /// MODEL-001: Create a new ClaudeProvider with explicit API key, auth mode, and model
    ///
    /// Uses shared validate_api_key_static() helper (REFAC-013).
    pub fn from_api_key_with_mode_and_model(
        api_key: &str,
        auth_mode: AuthMode,
        model: &str,
    ) -> Result<Self, ProviderError> {
        // Use shared validation helper (REFAC-013)
        validate_api_key_static("claude", api_key)?;

        // Parse beta headers for rig client
        let beta_features: Vec<&str> = ANTHROPIC_BETA_HEADER.split(',').collect();

        // Build rig client with beta headers (using default reqwest::Client)
        let mut rig_client: anthropic::Client = anthropic::Client::builder()
            .api_key(api_key)
            .anthropic_betas(&beta_features)
            .build()
            .map_err(|e| {
                ProviderError::config("claude", format!("Failed to build Anthropic client: {e}"))
            })?;

        // For OAuth mode, replace x-api-key header with Bearer auth
        if auth_mode == AuthMode::OAuth {
            let mut headers = rig_client.headers().clone();

            // Remove x-api-key header
            headers.remove("x-api-key");

            // Add Authorization: Bearer header
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {api_key}")).map_err(|e| {
                    ProviderError::auth("claude", format!("Invalid OAuth token: {e}"))
                })?,
            );

            // Rebuild client with modified headers
            rig_client = anthropic::Client::from_parts(
                rig_client.base_url().to_string(),
                headers,
                rig_client.http_client().clone(),
                rig_client.ext().clone(),
            );
        }

        // Create completion model with specified model name
        let completion_model =
            anthropic::completion::CompletionModel::new(rig_client.clone(), model);

        Ok(Self {
            completion_model,
            rig_client,
            auth_mode,
            model_name: model.to_string(),
        })
    }

    /// Check if the provider is using OAuth mode
    pub fn is_oauth_mode(&self) -> bool {
        self.auth_mode == AuthMode::OAuth
    }

    /// Get the configured rig anthropic::Client
    ///
    /// This client is fully configured with:
    /// - API key or OAuth token authentication
    /// - Anthropic beta features (prompt caching, OAuth, thinking, tool examples)
    /// - Proper headers (x-api-key for API key mode, Authorization: Bearer for OAuth)
    ///
    /// Use this client to create rig agents with tools.
    pub fn client(&self) -> &anthropic::Client {
        &self.rig_client
    }

    /// Get the system prompt prefix for this provider
    ///
    /// Returns Some() with the Claude Code prefix if OAuth mode is active,
    /// otherwise returns None. OAuth mode requires this prefix for authentication.
    pub fn system_prompt(&self) -> Option<&'static str> {
        if self.auth_mode == AuthMode::OAuth {
            Some(CLAUDE_CODE_PROMPT_PREFIX)
        } else {
            None
        }
    }

    /// Get the Claude Code system prompt prefix (required for OAuth)
    pub fn get_system_prompt_prefix(&self) -> &'static str {
        CLAUDE_CODE_PROMPT_PREFIX
    }

    /// Get the anthropic-beta header value
    pub fn get_anthropic_beta_header(&self) -> &'static str {
        ANTHROPIC_BETA_HEADER
    }

    /// Create a rig Agent with all 10 tools configured for this provider
    ///
    /// This method encapsulates all Claude-specific configuration:
    /// - Model name (claude-sonnet-4-20250514)
    /// - Max tokens (8192)
    /// - System prompt with cache_control metadata (PROV-006)
    /// - All 10 tools (Read, Write, Edit, Bash, Grep, Glob, Ls, AstGrep, AstGrepRefactor, WebSearchTool)
    /// - Prompt caching via cache_control metadata (PROV-006)
    /// - Extended thinking configuration (TOOL-010)
    ///
    /// # Arguments
    /// * `preamble` - Optional system prompt/preamble. For API key mode, this should
    ///   contain CLAUDE.md content and other context. For OAuth mode, this is the
    ///   ADDITIONAL content (facade adds the required Claude Code prefix).
    /// * `thinking_config` - Optional thinking configuration JSON (TOOL-010). When provided,
    ///   enables extended thinking with the specified budget. Format: `{"thinking": {"type": "enabled", "budget_tokens": N}}`
    ///
    /// # Cache Control Behavior (PROV-006, TOOL-008)
    /// - OAuth mode: 2 blocks - Claude Code prefix WITHOUT cache_control, preamble WITH cache_control
    /// - API key mode: 1 block - preamble WITH cache_control
    ///
    /// # TOOL-008: Uses SystemPromptFacade
    /// System prompt formatting now uses select_claude_facade() from codelet_tools,
    /// ensuring consistent formatting across the codebase.
    ///
    /// # TOOL-007 Note
    /// Web search uses FacadeToolWrapper(ClaudeWebSearchFacade) for consistent tool interfaces.
    /// This provides web search capabilities with Search, OpenPage, and FindInPage actions.
    ///
    /// Returns a fully configured rig::agent::Agent ready for use with RigAgent.
    pub fn create_rig_agent(
        &self,
        preamble: Option<&str>,
        thinking_config: Option<serde_json::Value>,
    ) -> rig::agent::Agent<anthropic::completion::CompletionModel> {
        // TOOL-008: Use facade for system prompt formatting
        use codelet_tools::facade::{
            select_claude_facade, ClaudeWebSearchFacade, FacadeToolWrapper,
        };
        use codelet_tools::{AstGrepTool, AstGrepRefactorTool, BashTool, EditTool, GlobTool, GrepTool, LsTool, ReadTool, WriteTool};
        use rig::client::CompletionClient;
        use std::sync::Arc;

        // Build agent with all 10 tools using rig's builder pattern (TOOL-007: Uses FacadeToolWrapper for web search)
        // MODEL-001: Use stored model name instead of DEFAULT_MODEL
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
            .tool(FacadeToolWrapper::new(Arc::new(ClaudeWebSearchFacade))); // TOOL-007: Use facade for consistent tool interfaces

        // PROV-006, TOOL-008: Apply cache_control to system prompt using facade
        let is_oauth = self.is_oauth_mode();
        let facade = select_claude_facade(is_oauth);

        // Get the preamble text (or empty string for OAuth-only prefix case)
        let preamble_text = preamble.unwrap_or("");

        // Build effective preamble for rig's internal handling
        // (rig needs the combined text, facade handles the structured format)
        let effective_preamble = facade.transform_preamble(preamble_text);

        // Set preamble for rig's internal handling
        agent_builder = agent_builder.preamble(&effective_preamble);

        // Override system field with array format containing cache_control (PROV-006, TOOL-008)
        // Facade handles OAuth vs API key formatting automatically
        let cached_system = facade.format_for_api(preamble_text);

        // TOOL-010: Merge thinking config with system prompt in additional_params
        let mut additional = json!({
            "system": cached_system
        });

        // If thinking config is provided, merge it into additional_params
        if let Some(thinking) = thinking_config {
            if let Some(obj) = additional.as_object_mut() {
                if let Some(thinking_obj) = thinking.as_object() {
                    for (key, value) in thinking_obj {
                        obj.insert(key.clone(), value.clone());
                    }
                }
            }
        }

        agent_builder = agent_builder.additional_params(additional);

        agent_builder.build()
    }

    /// Extract preamble and last user prompt from messages
    ///
    /// Note: Claude requires custom handling for OAuth mode prefix, so this
    /// method doesn't use the shared extract_prompt_data() helper.
    /// It does use the shared extract_text_from_content() helper (REFAC-013).
    fn extract_prompt_data(&self, messages: &[Message]) -> (Option<String>, String) {
        let mut system_prompt: Option<String> = None;
        let mut user_messages: Vec<String> = Vec::new();

        for msg in messages {
            match msg.role {
                MessageRole::System => {
                    // Use shared helper (REFAC-013)
                    let text = extract_text_from_content(&msg.content);

                    // OAuth mode requires prefix (but don't duplicate if already present)
                    system_prompt = if self.auth_mode == AuthMode::OAuth {
                        if text.starts_with(CLAUDE_CODE_PROMPT_PREFIX) {
                            // Already has prefix, use as-is
                            Some(text)
                        } else {
                            // Add prefix
                            Some(format!("{CLAUDE_CODE_PROMPT_PREFIX}\n\n{text}"))
                        }
                    } else {
                        Some(text)
                    };
                }
                MessageRole::User => {
                    // Use shared helper (REFAC-013)
                    let text = extract_text_from_content(&msg.content);
                    user_messages.push(text);
                }
                MessageRole::Assistant => {
                    // Multi-turn conversation history handled by rig
                }
            }
        }

        let prompt = if user_messages.is_empty() {
            String::new()
        } else {
            user_messages.join("\n\n")
        };

        (system_prompt, prompt)
    }

    /// Convert rig response to our CompletionResponse format
    fn rig_response_to_completion(
        &self,
        response: rig::completion::CompletionResponse<anthropic::completion::CompletionResponse>,
    ) -> Result<CompletionResponse, ProviderError> {
        // Convert rig AssistantContent to our ContentPart format using shared helper (REFAC-013)
        let content_parts = convert_assistant_content(response.choice, "claude")?;

        // Map Anthropic's stop_reason to our StopReason enum (provider-specific)
        let stop_reason = match response.raw_response.stop_reason.as_deref() {
            Some("tool_use") => StopReason::ToolUse,
            Some("max_tokens") => StopReason::MaxTokens,
            Some("end_turn") | Some("stop_sequence") | None => StopReason::EndTurn,
            Some(other) => {
                warn!(stop_reason = %other, "Unknown stop_reason from Anthropic API");
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
impl LlmProvider for ClaudeProvider {
    fn name(&self) -> &str {
        "claude"
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
        true // Claude supports prompt caching
    }

    fn supports_streaming(&self) -> bool {
        true // Streaming support via rig (REFAC-003)
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
        // Extract prompt data
        let (preamble, prompt) = self.extract_prompt_data(messages);

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
            .map_err(|e| ProviderError::api("claude", format!("Rig completion failed: {e}")))?;

        // Convert rig response to our CompletionResponse format
        self.rig_response_to_completion(response)
    }
}
