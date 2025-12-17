//! Anthropic Claude Provider implementation using rig
//!
//! Implements the LlmProvider trait for Claude API communication.
//! Uses rig::providers::anthropic for HTTP communication.

use crate::{CompletionResponse, LlmProvider, StopReason};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use codelet_common::{ContentPart, Message, MessageContent, MessageRole};
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

impl ClaudeProvider {
    /// Create a new ClaudeProvider using API key from environment
    ///
    /// Checks for API key in order of preference:
    /// 1. ANTHROPIC_API_KEY (uses standard x-api-key header)
    /// 2. CLAUDE_CODE_OAUTH_TOKEN (uses Bearer auth with special headers)
    pub fn new() -> Result<Self> {
        // Check for API key first (takes precedence)
        if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
            return Self::from_api_key_with_mode(&api_key, AuthMode::ApiKey);
        }

        // Fall back to OAuth token
        if let Ok(oauth_token) = std::env::var("CLAUDE_CODE_OAUTH_TOKEN") {
            return Self::from_api_key_with_mode(&oauth_token, AuthMode::OAuth);
        }

        Err(anyhow!(
            "No API key found. Set ANTHROPIC_API_KEY or CLAUDE_CODE_OAUTH_TOKEN environment variable"
        ))
    }

    /// Create a new ClaudeProvider with an explicit API key (defaults to ApiKey mode)
    pub fn from_api_key(api_key: &str) -> Result<Self> {
        Self::from_api_key_with_mode(api_key, AuthMode::ApiKey)
    }

    /// Create a new ClaudeProvider with an explicit API key and auth mode
    pub fn from_api_key_with_mode(api_key: &str, auth_mode: AuthMode) -> Result<Self> {
        if api_key.is_empty() {
            return Err(anyhow!("API key cannot be empty"));
        }

        // Parse beta headers for rig client
        let beta_features: Vec<&str> = ANTHROPIC_BETA_HEADER.split(',').collect();

        // Build rig client with beta headers (using default reqwest::Client)
        let mut rig_client: anthropic::Client = anthropic::Client::builder()
            .api_key(api_key)
            .anthropic_betas(&beta_features)
            .build()
            .map_err(|e| anyhow!("Failed to build Anthropic client: {e}"))?;

        // For OAuth mode, replace x-api-key header with Bearer auth
        if auth_mode == AuthMode::OAuth {
            let mut headers = rig_client.headers().clone();

            // Remove x-api-key header
            headers.remove("x-api-key");

            // Add Authorization: Bearer header
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {api_key}"))
                    .map_err(|e| anyhow!("Invalid OAuth token: {e}"))?,
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

    /// Create a rig Agent with all 8 tools configured for this provider
    ///
    /// This method encapsulates all Claude-specific configuration:
    /// - Model name (claude-sonnet-4-20250514)
    /// - Max tokens (8192)
    /// - System prompt with cache_control metadata (PROV-006)
    /// - All 8 tools (Read, Write, Edit, Bash, Grep, Glob, Ls, AstGrep)
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
    /// Returns a fully configured rig::agent::Agent ready for use with RigAgent.
    pub fn create_rig_agent(
        &self,
        preamble: Option<&str>,
    ) -> rig::agent::Agent<anthropic::completion::CompletionModel> {
        use crate::caching_client::transform_system_prompt;
        use codelet_tools::{
            AstGrepTool, BashTool, EditTool, GlobTool, GrepTool, LsTool, ReadTool, WriteTool,
        };
        use rig::client::CompletionClient;

        // Build agent with all 8 tools using rig's builder pattern
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
            .tool(AstGrepTool::new());

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

    /// Extract text content from a message (DRY helper)
    fn extract_text_from_content(content: &MessageContent) -> String {
        match content {
            MessageContent::Text(t) => t.clone(),
            MessageContent::Parts(parts) => parts
                .iter()
                .filter_map(|p| {
                    if let ContentPart::Text { text } = p {
                        Some(text.as_str())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }

    /// Extract preamble and last user prompt from messages
    fn extract_prompt_data(&self, messages: &[Message]) -> (Option<String>, String) {
        let mut system_prompt: Option<String> = None;
        let mut user_messages: Vec<String> = Vec::new();

        for msg in messages {
            match msg.role {
                MessageRole::System => {
                    let text = Self::extract_text_from_content(&msg.content);

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
                    let text = Self::extract_text_from_content(&msg.content);
                    user_messages.push(text);
                }
                MessageRole::Assistant => {
                    // Multi-turn conversation history (including assistant messages) is out of
                    // scope for REFAC-003. Multi-turn support will be added in REFAC-004 using
                    // rig::agent::Agent which handles conversation history automatically.
                    // For now, we only extract the last user message for single-turn completions.
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
    ) -> Result<CompletionResponse> {
        // Convert rig AssistantContent to our ContentPart format
        let mut content_parts: Vec<ContentPart> = vec![];

        for content in response.choice.into_iter() {
            match content {
                rig::completion::AssistantContent::Text(text) => {
                    content_parts.push(ContentPart::Text { text: text.text });
                }
                rig::completion::AssistantContent::ToolCall(tool_call) => {
                    content_parts.push(ContentPart::ToolUse {
                        id: tool_call.id,
                        name: tool_call.function.name,
                        input: tool_call.function.arguments,
                    });
                }
                rig::completion::AssistantContent::Reasoning(reasoning) => {
                    // Rig returns reasoning separately, we can include it as text
                    // or handle it specially if needed
                    for r in reasoning.reasoning {
                        content_parts.push(ContentPart::Text { text: r });
                    }
                }
                rig::completion::AssistantContent::Image(_) => {
                    // Anthropic doesn't support assistant images
                    return Err(anyhow!("Assistant images not supported"));
                }
            }
        }

        // Map Anthropic's stop_reason to our StopReason enum
        let stop_reason = match response.raw_response.stop_reason.as_deref() {
            Some("tool_use") => StopReason::ToolUse,
            Some("max_tokens") => StopReason::MaxTokens,
            Some("end_turn") | Some("stop_sequence") | None => StopReason::EndTurn,
            Some(other) => {
                // Unknown stop reason - log and default to EndTurn
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

    async fn complete(&self, messages: &[Message]) -> Result<String> {
        // Reuse complete_with_tools with no tools to avoid code duplication
        let response = self.complete_with_tools(messages, &[]).await?;

        // Extract text using helper method to maintain DRY
        Ok(Self::extract_text_from_content(&response.content))
    }

    async fn complete_with_tools(
        &self,
        messages: &[Message],
        tools: &[OurToolDefinition],
    ) -> Result<CompletionResponse> {
        // Extract prompt data
        let (preamble, prompt) = self.extract_prompt_data(messages);

        // Convert tools to rig format
        let rig_tools: Vec<rig::completion::ToolDefinition> = tools
            .iter()
            .map(|t| rig::completion::ToolDefinition {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters: t.input_schema.clone(),
            })
            .collect();

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
            .map_err(|e| anyhow!("Rig completion with tools failed: {e}"))?;

        // Convert rig response to our CompletionResponse format
        self.rig_response_to_completion(response)
    }
}
