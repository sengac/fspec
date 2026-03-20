//! Anthropic Claude Provider implementation using rig
//!
//! Implements the LlmProvider trait for Claude API communication.
//! Uses rig::providers::anthropic for HTTP communication.
//!
//! # TOOL-008: Uses SystemPromptFacade
//! This module uses the SystemPromptFacade from codelet_tools for provider-specific
//! system prompt formatting, eliminating duplicate code.
//!
//! # PROV-005: Adaptive Thinking Support
//! This module supports model-aware thinking configuration:
//! - Opus 4.6, Sonnet 4.6: Use adaptive thinking (type: "adaptive")
//! - Opus 4.5, Sonnet 4.5, older: Use budgeted thinking (type: "enabled", budget_tokens: N)
//!
//! Beta headers are also model-specific based on official Anthropic documentation.

use crate::{
    claude_oauth::CLAUDE_USER_AGENT,
    claude_refreshing_client::RefreshingClaudeClient,
    convert_assistant_content, convert_tools_to_rig, detect_credential_from_env,
    extract_text_from_content, validate_api_key_static, CompletionResponse, LlmProvider,
    ProviderAdapter, ProviderError, StopReason,
};
use async_trait::async_trait;
use codelet_common::{Message, MessageContent, MessageRole};
// TOOL-008: Import CLAUDE_CODE_PROMPT_PREFIX from facade (single source of truth)
use codelet_tools::facade::CLAUDE_CODE_PROMPT_PREFIX;
// PROV-005: Import model constants and helpers for adaptive thinking
use codelet_tools::facade::is_adaptive_thinking_model;
// CONFIG-007: supports_1m_context will be used when user opt-in toggle is implemented
// use codelet_tools::facade::supports_1m_context;
use codelet_tools::ToolDefinition as OurToolDefinition;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION};
use rig::completion::CompletionRequestBuilder;
use rig::providers::anthropic;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::warn;

/// Claude default context window size
/// NOTE: Some models support 1M context with Tier 4 API access, but we use 200k
/// as the default until CONFIG-007 implements user opt-in for extended context.
pub const CONTEXT_WINDOW: usize = 200_000;

/// Claude Sonnet 4 max output tokens (CTX-002)
pub const MAX_OUTPUT_TOKENS: usize = 8192;

// TOOL-008: CLAUDE_CODE_PROMPT_PREFIX is now imported from codelet_tools::facade
// (single source of truth - see imports at top)

// =============================================================================
// BETA HEADER CONSTANTS (PROV-005)
// =============================================================================

/// Beta header constants for Anthropic API features
pub mod beta_headers {
    /// Prompt caching feature (all models)
    pub const PROMPT_CACHING: &str = "prompt-caching-2024-07-31";
    /// Interleaved thinking for budgeted models (NOT needed for adaptive 4.6 models)
    pub const INTERLEAVED_THINKING: &str = "interleaved-thinking-2025-05-14";
    /// 1M context window (Opus 4.6, Sonnet 4.6, Sonnet 4.5 - NOT Opus 4.5)
    pub const CONTEXT_1M: &str = "context-1m-2025-08-07";
    /// Claude Code identifier for OAuth mode
    pub const CLAUDE_CODE: &str = "claude-code-20250219";
    /// OAuth feature for OAuth mode
    pub const OAUTH: &str = "oauth-2025-04-20";
}

/// Build model-aware beta headers for Claude API requests.
///
/// PROV-005: Beta headers depend on model capabilities:
/// - Adaptive thinking models (4.6): NO interleaved-thinking header needed
/// - Budgeted thinking models: Need interleaved-thinking header
/// - 1M context models: Include context-1m header
///
/// # Arguments
/// * `model` - The Claude model name
/// * `is_oauth` - Whether OAuth authentication is being used
///
/// # Returns
/// Comma-separated string of beta headers for the model
pub fn build_beta_headers(model: &str, is_oauth: bool) -> String {
    let mut headers = Vec::new();

    // OAuth mode headers (must be first for Claude Code compatibility)
    if is_oauth {
        headers.push(beta_headers::CLAUDE_CODE);
        headers.push(beta_headers::OAUTH);
    }

    // Always include prompt caching
    headers.push(beta_headers::PROMPT_CACHING);

    // PROV-005: Interleaved thinking only for non-adaptive models
    // Adaptive thinking models (Opus 4.6, Sonnet 4.6) don't need this header
    if !is_adaptive_thinking_model(model) {
        headers.push(beta_headers::INTERLEAVED_THINKING);
    }

    // CONFIG-007: 1M context header is NOT sent by default.
    // This requires Anthropic API Tier 4 ($400+ cumulative spend) and triggers
    // "Extra usage is required for long context requests" error for non-Tier-4 users.
    // The header will only be sent when CONFIG-007 implements user opt-in toggle.
    // if supports_1m_context(model) {
    //     headers.push(beta_headers::CONTEXT_1M);
    // }

    headers.join(",")
}

/// Legacy: Anthropic beta features header for API key mode (standard features)
/// DEPRECATED: Use build_beta_headers() for model-aware headers
const ANTHROPIC_BETA_HEADER_API_KEY: &str =
    "prompt-caching-2024-07-31,interleaved-thinking-2025-05-14";

/// Legacy: Anthropic beta features header for OAuth mode (must match Claude Code exactly)
/// DEPRECATED: Use build_beta_headers() for model-aware headers
const ANTHROPIC_BETA_HEADER_OAUTH: &str =
    "claude-code-20250219,oauth-2025-04-20,prompt-caching-2024-07-31,interleaved-thinking-2025-05-14";

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

/// Type alias for the Anthropic client parameterized with RefreshingClaudeClient.
/// In OAuth mode, RefreshingClaudeClient intercepts requests for token refresh.
/// In API key mode, it passes through to reqwest unchanged.
type ClaudeClient = anthropic::Client<RefreshingClaudeClient>;

/// Type alias for the Anthropic completion model parameterized with RefreshingClaudeClient.
type ClaudeCompletionModel = anthropic::completion::CompletionModel<RefreshingClaudeClient>;

/// Claude Provider for Anthropic API (using rig)
#[derive(Clone)]
pub struct ClaudeProvider {
    completion_model: ClaudeCompletionModel,
    rig_client: ClaudeClient,
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
    /// Create a new ClaudeProvider with explicit model.
    ///
    /// Model is REQUIRED - will return error if None.
    ///
    /// Auth mode is determined by the **token prefix**, not by which environment
    /// variable the credential came from. This allows OAuth tokens stored in
    /// ANTHROPIC_API_KEY to be correctly detected and use Bearer authentication.
    ///
    /// Token prefixes:
    /// - `sk-ant-oat` → OAuth token → uses `Authorization: Bearer` header
    /// - `sk-ant-api` or other → API key → uses `x-api-key` header
    pub fn new_with_model(model: Option<&str>) -> Result<Self, ProviderError> {
        let model_name = model.ok_or_else(|| {
            ProviderError::config("claude", "Model is required. Please select a model before creating a session.")
        })?;

        // Check both env vars for credentials
        let credential = detect_credential_from_env("claude", &["ANTHROPIC_API_KEY", "CLAUDE_CODE_OAUTH_TOKEN"]);
        
        if let Ok(token) = credential {
            let auth_mode = Self::detect_auth_mode_from_token(&token);
            return Self::from_api_key_with_mode_and_model(&token, auth_mode, model_name);
        }

        Err(ProviderError::auth(
            "claude",
            "No API key found. Set ANTHROPIC_API_KEY or CLAUDE_CODE_OAUTH_TOKEN environment variable",
        ))
    }
    
    /// Detect auth mode from token prefix.
    /// OAuth tokens start with `sk-ant-oat`, API keys use other prefixes.
    fn detect_auth_mode_from_token(token: &str) -> AuthMode {
        if token.starts_with("sk-ant-oat") {
            AuthMode::OAuth
        } else {
            AuthMode::ApiKey
        }
    }

    /// MODEL-001: Create a new ClaudeProvider with explicit model
    ///
    /// Model is REQUIRED.
    pub fn from_api_key_with_model(api_key: &str, model: &str) -> Result<Self, ProviderError> {
        Self::from_api_key_with_mode_and_model(api_key, AuthMode::ApiKey, model)
    }

    /// MODEL-001: Create a new ClaudeProvider with explicit API key, auth mode, and model
    ///
    /// Uses shared validate_api_key_static() helper (REFAC-013).
    /// PROV-005: Uses model-aware beta headers for adaptive thinking support.
    /// PROV-023: Uses RefreshingClaudeClient as HTTP backend for all modes.
    /// In OAuth mode, it intercepts requests for token refresh and auth header injection.
    /// In API key mode, it passes through to reqwest unchanged.
    pub fn from_api_key_with_mode_and_model(
        api_key: &str,
        auth_mode: AuthMode,
        model: &str,
    ) -> Result<Self, ProviderError> {
        validate_api_key_static("claude", api_key)?;

        // PROV-005: Build model-aware beta headers
        let beta_header_string = build_beta_headers(model, auth_mode == AuthMode::OAuth);
        let beta_features: Vec<&str> = beta_header_string.split(',').collect();

        // PROV-023: Create RefreshingClaudeClient as HTTP backend.
        // In API key mode: pass-through (no interception).
        // In OAuth mode: requires from_oauth_tokens() constructor instead — this path
        // uses API key mode pass-through since we don't have refresh tokens here.
        // The OAuth-with-refresh path is from_oauth_tokens().
        let http_client = RefreshingClaudeClient::new_api_key();

        // Build rig client based on auth mode
        // Note: The patched rig-core's AnthropicKey::into_header() automatically skips
        // x-api-key for OAuth tokens (those starting with "sk-ant-oat")
        // PROV-023: Start from reqwest::Client builder, then swap HTTP backend
        // via .http_client() which transforms the type parameter to RefreshingClaudeClient.
        let rig_client: ClaudeClient = if auth_mode == AuthMode::OAuth {
            let mut headers = HeaderMap::new();

            // Authorization: Bearer token (instead of x-api-key)
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {api_key}")).map_err(|e| {
                    ProviderError::auth("claude", format!("Invalid OAuth token: {e}"))
                })?,
            );

            // User-Agent must match Claude Code exactly (DRY: uses shared constant)
            headers.insert(
                reqwest::header::USER_AGENT,
                HeaderValue::from_static(CLAUDE_USER_AGENT),
            );

            // x-app header identifies the application type
            headers.insert(
                HeaderName::from_static("x-app"),
                HeaderValue::from_static("cli"),
            );

            anthropic::Client::<reqwest::Client>::builder()
                .api_key(api_key)
                .anthropic_betas(&beta_features)
                .http_headers(headers)
                .http_client(http_client)
                .build()
                .map_err(|e| {
                    ProviderError::config("claude", format!("Failed to build Anthropic client: {e}"))
                })?
        } else {
            // Standard API key mode - x-api-key header is added automatically
            anthropic::Client::<reqwest::Client>::builder()
                .api_key(api_key)
                .anthropic_betas(&beta_features)
                .http_client(http_client)
                .build()
                .map_err(|e| {
                    ProviderError::config("claude", format!("Failed to build Anthropic client: {e}"))
                })?
        };

        // Create completion model with specified model name and prompt caching enabled
        // OAuth mode supports prompt caching via the prompt-caching-2024-07-31 beta header
        let completion_model =
            anthropic::completion::CompletionModel::new(rig_client.clone(), model)
                .with_prompt_caching();

        Ok(Self {
            completion_model,
            rig_client,
            auth_mode,
            model_name: model.to_string(),
        })
    }

    /// Create a new ClaudeProvider using OAuth tokens with token refresh support
    ///
    /// PROV-023: This is the primary OAuth constructor. Creates a RefreshingClaudeClient
    /// in OAuth mode that will:
    /// - Automatically refresh expired tokens before each request
    /// - Strip stale Authorization headers and inject fresh Bearer tokens
    /// - Persist refreshed tokens to claude_auth.json (best-effort)
    ///
    /// Use `expires_in_secs: Some(0)` when loading tokens from disk to force
    /// immediate refresh on first API request.
    ///
    /// # Arguments
    /// * `access_token` - Current OAuth access token
    /// * `refresh_token` - Refresh token for obtaining new access tokens
    /// * `expires_in_secs` - Token expiry in seconds (Some(0) = force immediate refresh)
    /// * `token_endpoint_base` - Base URL for token refresh (e.g., "https://console.anthropic.com")
    /// * `model` - Model name (e.g., "claude-sonnet-4-20250514")
    pub fn from_oauth_tokens(
        access_token: &str,
        refresh_token: &str,
        expires_in_secs: Option<u64>,
        token_endpoint_base: &str,
        model: &str,
    ) -> Result<Self, ProviderError> {
        // PROV-005: Build model-aware beta headers for OAuth mode
        let beta_header_string = build_beta_headers(model, true);
        let beta_features: Vec<&str> = beta_header_string.split(',').collect();

        // PROV-023: Create RefreshingClaudeClient in OAuth mode with token refresh
        let http_client = RefreshingClaudeClient::new_oauth(
            access_token.to_string(),
            refresh_token.to_string(),
            expires_in_secs,
            token_endpoint_base.to_string(),
        );

        // Build static headers for rig client
        // Note: Authorization header is set statically here but will be REPLACED
        // by RefreshingClaudeClient on each request with the current (possibly refreshed) token
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {access_token}")).map_err(|e| {
                ProviderError::auth("claude", format!("Invalid OAuth token: {e}"))
            })?,
        );
        headers.insert(
            reqwest::header::USER_AGENT,
            HeaderValue::from_static(CLAUDE_USER_AGENT),
        );
        headers.insert(
            HeaderName::from_static("x-app"),
            HeaderValue::from_static("cli"),
        );

        let rig_client: ClaudeClient = anthropic::Client::<reqwest::Client>::builder()
            .api_key(access_token)
            .anthropic_betas(&beta_features)
            .http_headers(headers)
            .http_client(http_client)
            .build()
            .map_err(|e| {
                ProviderError::config("claude", format!("Failed to build Anthropic OAuth client: {e}"))
            })?;

        let completion_model =
            anthropic::completion::CompletionModel::new(rig_client.clone(), model)
                .with_prompt_caching();

        Ok(Self {
            completion_model,
            rig_client,
            auth_mode: AuthMode::OAuth,
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
    /// - PROV-023: RefreshingClaudeClient as HTTP backend for token refresh
    ///
    /// Use this client to create rig agents with tools.
    pub fn client(&self) -> &ClaudeClient {
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

    /// Get the anthropic-beta header value based on auth mode (legacy, non-model-aware)
    ///
    /// DEPRECATED: Use get_anthropic_beta_header_for_model() for model-aware headers.
    /// This method is kept for backward compatibility but doesn't include
    /// adaptive thinking or 1M context headers for 4.6 models.
    pub fn get_anthropic_beta_header(&self) -> &'static str {
        if self.auth_mode == AuthMode::OAuth {
            ANTHROPIC_BETA_HEADER_OAUTH
        } else {
            ANTHROPIC_BETA_HEADER_API_KEY
        }
    }

    /// Get the anthropic-beta header value for the current model (PROV-005)
    ///
    /// Returns model-aware beta headers based on official Anthropic documentation:
    /// - Adaptive thinking models (4.6): NO interleaved-thinking, YES context-1m
    /// - Budgeted thinking models (4.5, older): YES interleaved-thinking
    /// - 1M context models (Opus 4.6, Sonnet 4.6, Sonnet 4.5): YES context-1m
    pub fn get_anthropic_beta_header_for_model(&self) -> String {
        build_beta_headers(&self.model_name, self.auth_mode == AuthMode::OAuth)
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
    /// # TOOL-012 Note
    /// Session ID is now required as the first parameter. This ensures tools know which
    /// session's handler to use at call time, eliminating reliance on thread-local state.
    ///
    /// Returns a fully configured rig::agent::Agent ready for use with RigAgent.
    pub fn create_rig_agent(
        &self,
        session_id: uuid::Uuid,
        preamble: Option<&str>,
        thinking_config: Option<serde_json::Value>,
    ) -> rig::agent::Agent<ClaudeCompletionModel> {
        // TOOL-008: Use facade for system prompt formatting
        use codelet_tools::facade::{
            claude_bridge_tool, claude_fspec_tool, select_claude_facade, ClaudeWebSearchFacade, FacadeToolWrapper,
        };
        use codelet_tools::{
            AstGrepRefactorTool, AstGrepTool, BashTool, EditTool, GlobTool, GrepTool, LsTool,
            ReadTool, WriteTool, ConnectMcpTool, SessionSearchTool, GraphSearchTool, InjectSummaryTool,
            DeepSearchTool, RequestUserInputTool, AgentManagerTool, ScheduleTool,
        };
        use rig::client::CompletionClient;
        use std::sync::Arc;

        // Build agent with all 11 tools using rig's builder pattern (TOOL-007: Uses FacadeToolWrapper for web search)
        // MODEL-001: Use stored model name instead of DEFAULT_MODEL
        // TOOL-012: Pass session_id to fspec and bridge tools
        // TOOL-014: All tools require session_id for worktree isolation
        let mut agent_builder = self
            .rig_client
            .agent(&self.model_name)
            .max_tokens(MAX_OUTPUT_TOKENS as u64)
            .tool(ReadTool::new(session_id))
            .tool(WriteTool::new(session_id))
            .tool(EditTool::new(session_id))
            .tool(BashTool::new(session_id))
            .tool(GrepTool::new(session_id))
            .tool(GlobTool::new(session_id))
            .tool(LsTool::new(session_id))
            .tool(AstGrepTool::new(session_id)) // TOOL-014: AstGrepTool with session_id for worktree isolation
            .tool(AstGrepRefactorTool::new(session_id)) // TOOL-014: AstGrepRefactorTool with session_id for worktree isolation
            .tool(claude_fspec_tool(session_id)) // TOOL-012: FspecTool with explicit session association
            .tool(claude_bridge_tool(session_id)) // TOOL-012: BridgeTool with explicit session association
            .tool(FacadeToolWrapper::new(Arc::new(ClaudeWebSearchFacade), session_id)) // TOOL-007, TOOL-014: Facade with session_id
            .tool(ConnectMcpTool::new(session_id)) // MCP-001: Dynamic MCP connections
            .tool(SessionSearchTool::new(session_id)) // AMGR-001: SessionSearch tool
            .tool(GraphSearchTool::new(session_id)) // KGRAPH-003: GraphSearch tool
            .tool(InjectSummaryTool::new(session_id))
            .tool(DeepSearchTool::new(session_id)) // RLM-001: DeepSearch tool
            .tool(AgentManagerTool::new(session_id)) // AMGR-009: AgentManager tool
            .tool(RequestUserInputTool::new(session_id)) // TOOL-017: HITL tool
            .tool(ScheduleTool::new(session_id)); // SCHED-009: Schedule AI tool

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
        // PROV-005-DEBUG: Log the raw stop_reason for debugging Opus 4.6 compaction
        let raw_stop_reason = response.raw_response.stop_reason.as_deref();
        warn!(
            "[ClaudeProvider] raw stop_reason={:?}, content_len={}",
            raw_stop_reason,
            response.raw_response.content.len()
        );
        
        let stop_reason = match raw_stop_reason {
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
        // CONFIG-007: 1M context opt-in not yet implemented
        // Until then, use 200k for all models (the default for non-Tier-4 users)
        // NOTE: We still send context-1m beta header for Tier 4 users who benefit
        // from it, but compaction uses 200k threshold until opt-in is implemented.
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod oauth_header_tests {
    use super::*;
    use rig::client::Provider;

    #[test]
    fn test_oauth_headers_are_set_correctly() {
        let provider = ClaudeProvider::from_api_key_with_mode_and_model(
            "sk-ant-oat01-test-token",
            AuthMode::OAuth,
            "claude-sonnet-4-20250514",
        ).expect("Should create provider");

        let headers = provider.client().headers();

        println!("\n=== OAuth Headers ===");
        for (k, v) in headers.iter() {
            println!("  {}: {}", k, v.to_str().unwrap_or("<binary>"));
        }
        println!("=====================\n");

        // Check Authorization header
        let auth = headers.get("authorization").expect("Should have authorization");
        assert!(auth.to_str().unwrap().starts_with("Bearer "), "Should use Bearer auth");

        // Check User-Agent
        let ua = headers.get("user-agent").expect("Should have user-agent");
        assert!(ua.to_str().unwrap().contains("claude-cli"), "User-Agent should contain claude-cli");

        // Check x-app
        let xapp = headers.get("x-app").expect("Should have x-app");
        assert_eq!(xapp.to_str().unwrap(), "cli");

        // Check anthropic-beta
        let beta = headers.get("anthropic-beta").expect("Should have anthropic-beta");
        let beta_str = beta.to_str().unwrap();
        assert!(beta_str.contains("claude-code-20250219"), "anthropic-beta should contain claude-code-20250219");
        assert!(beta_str.contains("oauth-2025-04-20"), "anthropic-beta should contain oauth-2025-04-20");

        // Check NO x-api-key
        assert!(headers.get("x-api-key").is_none(), "Should NOT have x-api-key");
    }

    #[test]
    fn test_oauth_url_includes_beta_query_param() {
        let provider = ClaudeProvider::from_api_key_with_mode_and_model(
            "sk-ant-oat01-test-token",
            AuthMode::OAuth,
            "claude-sonnet-4-20250514",
        ).expect("Should create provider");

        // Get the ext from the client to test build_uri
        let ext = provider.client().ext();
        let url = ext.build_uri(
            "https://api.anthropic.com",
            "/v1/messages",
            rig::client::Transport::Http,
        );

        println!("OAuth URL: {url}");
        assert!(
            url.contains("?beta=true"),
            "OAuth URL should contain ?beta=true, got: {url}"
        );
    }

    #[test]
    fn test_api_key_url_does_not_include_beta_query_param() {
        let provider = ClaudeProvider::from_api_key_with_mode_and_model(
            "sk-ant-api03-test-key",
            AuthMode::ApiKey,
            "claude-sonnet-4-20250514",
        ).expect("Should create provider");

        // Get the ext from the client to test build_uri
        let ext = provider.client().ext();
        let url = ext.build_uri(
            "https://api.anthropic.com",
            "/v1/messages",
            rig::client::Transport::Http,
        );

        println!("API Key URL: {url}");
        assert!(
            !url.contains("?beta=true"),
            "API Key URL should NOT contain ?beta=true, got: {url}"
        );
    }
}
