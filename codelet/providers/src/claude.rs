//! Anthropic Claude Provider implementation using rig
//!
//! Implements the LlmProvider trait for Claude API communication.
//! Uses rig::providers::anthropic for HTTP communication.

use crate::{
    convert_assistant_content, convert_tools_to_rig, detect_credential_from_env,
    extract_text_from_content, validate_api_key_static, CompletionResponse, LlmProvider,
    ProviderAdapter, ProviderError, StopReason,
};
use async_trait::async_trait;
use codelet_common::{Message, MessageContent, MessageRole};
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

/// Claude Sonnet 4 max output tokens
const MAX_OUTPUT_TOKENS: usize = 8192;

/// Claude Code system prompt prefix (required for OAuth authentication)
const CLAUDE_CODE_PROMPT_PREFIX: &str = "You are Claude Code, Anthropic's official CLI for Claude.";

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

/// Build a cached system prompt array for Anthropic API (CLI-017)
///
/// Converts a preamble string into the array format required for cache_control:
/// ```json
/// [{ "type": "text", "text": "...", "cache_control": { "type": "ephemeral" } }]
/// ```
///
/// For OAuth mode, the OAuth prefix is added as the first block WITHOUT cache_control,
/// and the preamble is added as the second block WITH cache_control.
///
/// # Arguments
/// * `preamble` - The system prompt text
/// * `is_oauth` - Whether OAuth mode is active
/// * `oauth_prefix` - Optional OAuth prefix (required if is_oauth is true)
///
/// # Returns
/// * `serde_json::Value` array suitable for use in additional_params
pub fn build_cached_system_prompt(
    preamble: &str,
    is_oauth: bool,
    oauth_prefix: Option<&str>,
) -> serde_json::Value {
    if is_oauth {
        // OAuth mode: first block is prefix without cache_control
        // second block is preamble with cache_control
        let prefix = oauth_prefix.unwrap_or(CLAUDE_CODE_PROMPT_PREFIX);
        json!([
            {
                "type": "text",
                "text": prefix
            },
            {
                "type": "text",
                "text": preamble,
                "cache_control": { "type": "ephemeral" }
            }
        ])
    } else {
        // API key mode: single block with cache_control
        json!([
            {
                "type": "text",
                "text": preamble,
                "cache_control": { "type": "ephemeral" }
            }
        ])
    }
}

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
}

impl std::fmt::Debug for ClaudeProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClaudeProvider")
            .field("model", &DEFAULT_MODEL)
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
        // Check for API key first (takes precedence) using shared helper
        if let Ok(api_key) = detect_credential_from_env("claude", &["ANTHROPIC_API_KEY"]) {
            return Self::from_api_key_with_mode(&api_key, AuthMode::ApiKey);
        }

        // Fall back to OAuth token using shared helper
        if let Ok(oauth_token) = detect_credential_from_env("claude", &["CLAUDE_CODE_OAUTH_TOKEN"])
        {
            return Self::from_api_key_with_mode(&oauth_token, AuthMode::OAuth);
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

    /// Create a new ClaudeProvider with an explicit API key and auth mode
    ///
    /// Uses shared validate_api_key_static() helper (REFAC-013).
    pub fn from_api_key_with_mode(
        api_key: &str,
        auth_mode: AuthMode,
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

        // Create completion model
        let completion_model =
            anthropic::completion::CompletionModel::new(rig_client.clone(), DEFAULT_MODEL);

        Ok(Self {
            completion_model,
            rig_client,
            auth_mode,
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

    /// Create a rig Agent with all 9 tools configured for this provider
    ///
    /// This method encapsulates all Claude-specific configuration:
    /// - Model name (claude-sonnet-4-20250514)
    /// - Max tokens (8192)
    /// - System prompt with cache_control metadata (PROV-006)
    /// - All 9 tools (Read, Write, Edit, Bash, Grep, Glob, Ls, AstGrep, WebSearchTool)
    /// - Prompt caching via cache_control metadata (PROV-006)
    ///
    /// # Arguments
    /// * `preamble` - Optional system prompt/preamble. For API key mode, this should
    ///   contain CLAUDE.md content and other context. For OAuth mode, this is combined
    ///   with the required Claude Code prefix.
    ///
    /// # Cache Control Behavior (PROV-006)
    /// - OAuth mode: 2 blocks - Claude Code prefix WITHOUT cache_control, preamble WITH cache_control
    /// - API key mode: 1 block - preamble WITH cache_control
    ///
    /// # WEB-001 Note
    /// WebSearchTool is now included as a rig tool that provides web search capabilities
    /// with Search, OpenPage, and FindInPage actions.
    ///
    /// Returns a fully configured rig::agent::Agent ready for use with RigAgent.
    pub fn create_rig_agent(
        &self,
        preamble: Option<&str>,
    ) -> rig::agent::Agent<anthropic::completion::CompletionModel> {
        use crate::caching_client::transform_system_prompt;
        use codelet_tools::{
            AstGrepTool, BashTool, EditTool, GlobTool, GrepTool, LsTool, ReadTool, WebSearchTool,
            WriteTool,
        };
        use rig::client::CompletionClient;

        // Build agent with all 9 tools using rig's builder pattern (WEB-001: Added WebSearchTool)
        let mut agent_builder = self
            .rig_client
            .agent(DEFAULT_MODEL)
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

        // PROV-006: Apply cache_control to system prompt for BOTH auth modes
        // OAuth mode: built-in prefix + optional additional preamble
        // API key mode: just the provided preamble
        let is_oauth = self.is_oauth_mode();

        // Determine the effective preamble based on auth mode
        let effective_preamble = if is_oauth {
            // OAuth mode: Claude Code prefix is required, optionally combined with additional preamble
            match preamble {
                Some(p) if !p.is_empty() => {
                    // Combine prefix with additional preamble
                    Some(format!("{CLAUDE_CODE_PROMPT_PREFIX}\n\n{p}"))
                }
                _ => {
                    // Just the required prefix
                    Some(CLAUDE_CODE_PROMPT_PREFIX.to_string())
                }
            }
        } else {
            // API key mode: use provided preamble as-is
            preamble.map(str::to_string)
        };

        // Apply cache_control transformation if we have a preamble
        if let Some(ref preamble_text) = effective_preamble {
            // Set preamble for rig's internal handling
            agent_builder = agent_builder.preamble(preamble_text);

            // Override system field with array format containing cache_control (PROV-006)
            let cached_system = transform_system_prompt(
                preamble_text,
                is_oauth,
                if is_oauth {
                    Some(CLAUDE_CODE_PROMPT_PREFIX)
                } else {
                    None
                },
            );
            agent_builder = agent_builder.additional_params(json!({
                "system": cached_system
            }));
        }

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
        DEFAULT_MODEL
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
