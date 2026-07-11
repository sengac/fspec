//! OpenAI Provider implementation using rig
//!
//! Implements the LlmProvider trait for OpenAI API communication.
//! Uses rig::providers::openai for HTTP communication.
//!
//! PROV-006: Supports custom base URLs for local model servers (vLLM, Ollama, LM Studio)
//! via OPENAI_BASE_URL environment variable.

use crate::{
    cache_optimization::{CacheOptimizationFacade, SessionAffinityConfig},
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

/// Default context window size (fallback when not configured)
/// For local servers, use OPENAI_CONTEXT_WINDOW to match your model's actual limit
const DEFAULT_CONTEXT_WINDOW: usize = 128_000;

/// Default max output tokens (fallback when not configured)
/// For local servers, use OPENAI_MAX_OUTPUT_TOKENS to match your model's actual limit
const DEFAULT_MAX_OUTPUT_TOKENS: usize = 4096;

/// Context window size constant (for backwards compatibility)
pub const CONTEXT_WINDOW: usize = DEFAULT_CONTEXT_WINDOW;

/// Max output tokens constant (for backwards compatibility)
pub const MAX_OUTPUT_TOKENS: usize = DEFAULT_MAX_OUTPUT_TOKENS;

/// Normalize base URL to ensure it includes /v1 suffix
///
/// The rig library's default OpenAI base URL is "https://api.openai.com/v1",
/// which includes the /v1 prefix. When using custom base URLs for local servers
/// (vLLM, Ollama, etc.), the URL typically doesn't include /v1, but the rig
/// client expects it because it appends paths like /chat/completions directly.
///
/// This function ensures consistency by adding /v1 if not present.
fn normalize_base_url(url: &str) -> String {
    let trimmed = url.trim_end_matches('/');
    if trimmed.ends_with("/v1") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/v1")
    }
}

/// OpenAI Provider for OpenAI API (using rig)
///
/// PROV-006: Now supports custom base URLs for local model servers via:
/// - OPENAI_BASE_URL: Custom API endpoint (e.g., http://localhost:8888)
/// - OPENAI_CONTEXT_WINDOW: Custom context window size
/// - OPENAI_MAX_OUTPUT_TOKENS: Custom max output tokens
#[derive(Clone)]
pub struct OpenAIProvider {
    completion_model: openai::completion::CompletionModel,
    rig_client: openai::CompletionsClient,
    model_name: String,
    /// Custom base URL if set (None for default OpenAI API)
    base_url: Option<String>,
    /// Context window size (configurable via OPENAI_CONTEXT_WINDOW)
    context_window: usize,
    /// Max output tokens (configurable via OPENAI_MAX_OUTPUT_TOKENS)
    max_output_tokens: usize,
    /// PROV-140: per-profile streaming toggle sourced from `OPENAI_STREAMING`
    /// (default `true`; only an explicit "false" disables). Drives
    /// `supports_streaming()` and the rig model's request path.
    streaming: bool,
}

impl std::fmt::Debug for OpenAIProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAIProvider")
            .field("model", &self.model_name)
            .field("base_url", &self.base_url)
            .field("context_window", &self.context_window)
            .field("max_output_tokens", &self.max_output_tokens)
            .finish()
    }
}

impl ProviderAdapter for OpenAIProvider {
    fn provider_name(&self) -> &'static str {
        "openai"
    }
}

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

    /// Default context window, reading `OPENAI_CONTEXT_WINDOW` env var at
    /// construction time (already stored in `self.context_window`).
    fn default_context_window(&self) -> usize {
        self.context_window
    }

    /// Default max output tokens, reading `OPENAI_MAX_OUTPUT_TOKENS` env var
    /// at construction time (already stored in `self.max_output_tokens`).
    fn default_max_output_tokens(&self) -> usize {
        self.max_output_tokens
    }
}

impl OpenAIProvider {
    /// Create a new OpenAIProvider using API key from environment
    ///
    /// Checks for API key in OPENAI_API_KEY environment variable.
    /// Model MUST be provided via OPENAI_MODEL environment variable.
    ///
    /// PROV-006: Also checks OPENAI_BASE_URL for custom endpoint,
    /// OPENAI_CONTEXT_WINDOW and OPENAI_MAX_OUTPUT_TOKENS for custom limits.
    ///
    /// Uses shared detect_credential_from_env() helper (REFAC-013).
    pub fn new() -> Result<Self, ProviderError> {
        // Use shared credential detection helper (REFAC-013)
        let api_key = detect_credential_from_env("openai", &["OPENAI_API_KEY"])?;

        // Model is REQUIRED via OPENAI_MODEL env var
        let model_name = std::env::var("OPENAI_MODEL").map_err(|_| {
            ProviderError::config(
                "openai",
                "Model is required. Set OPENAI_MODEL environment variable.",
            )
        })?;

        // PROV-006: Check for custom base URL and normalize it
        let base_url = std::env::var("OPENAI_BASE_URL")
            .ok()
            .map(|url| normalize_base_url(&url));

        Self::from_api_key_with_options(&api_key, &model_name, base_url.as_deref(), None)
    }

    /// Create a new OpenAIProvider with an explicit API key and model
    ///
    /// Uses shared validate_api_key_static() helper (REFAC-013).
    /// PROV-051: Delegates to from_api_key_with_options with no session_id.
    /// Use from_api_key_with_session for session-aware construction.
    pub fn from_api_key(api_key: &str, model: &str) -> Result<Self, ProviderError> {
        // Check for custom base URL from environment and normalize it
        let base_url = std::env::var("OPENAI_BASE_URL")
            .ok()
            .map(|url| normalize_base_url(&url));
        Self::from_api_key_with_options(api_key, model, base_url.as_deref(), None)
    }

    /// Create a new OpenAIProvider with explicit API key, model, and session ID
    ///
    /// PROV-051: Session-aware constructor that reads OPENAI_BASE_URL from env
    /// and applies cache optimization headers when a custom base URL is set.
    pub fn from_api_key_with_session(
        api_key: &str,
        model: &str,
        session_id: uuid::Uuid,
    ) -> Result<Self, ProviderError> {
        let base_url = std::env::var("OPENAI_BASE_URL")
            .ok()
            .map(|url| normalize_base_url(&url));
        Self::from_api_key_with_options(api_key, model, base_url.as_deref(), Some(session_id))
    }

    /// Create a new OpenAIProvider with explicit API key, model, and optional base URL
    ///
    /// PROV-006: New constructor that supports custom base URLs for local model servers.
    /// PROV-051: Accepts optional session_id for cache optimization (session affinity headers).
    ///
    /// # Arguments
    /// * `api_key` - The API key (any non-empty value for local servers)
    /// * `model` - The model name
    /// * `base_url` - Optional custom base URL (e.g., "http://localhost:8888")
    /// * `session_id` - Optional session UUID for cache optimization headers
    pub fn from_api_key_with_options(
        api_key: &str,
        model: &str,
        base_url: Option<&str>,
        session_id: Option<uuid::Uuid>,
    ) -> Result<Self, ProviderError> {
        // Use shared validation helper (REFAC-013)
        validate_api_key_static("openai", api_key)?;

        // Read configurable context window and max output tokens from environment
        let context_window = std::env::var("OPENAI_CONTEXT_WINDOW")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(DEFAULT_CONTEXT_WINDOW);

        let max_output_tokens = std::env::var("OPENAI_MAX_OUTPUT_TOKENS")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(DEFAULT_MAX_OUTPUT_TOKENS);

        // PROV-140: read the per-profile streaming toggle. Streaming stays
        // enabled unless OPENAI_STREAMING is explicitly "false".
        let streaming = std::env::var("OPENAI_STREAMING")
            .map(|value| !value.trim().eq_ignore_ascii_case("false"))
            .unwrap_or(true);

        // Build rig completions client
        // PROV-006: Use custom base_url if provided (for vLLM, Ollama, etc.)
        // PROV-051: Apply cache optimization headers via facade when session_id is available
        let cache_headers = session_id
            .map(|sid| {
                let config = SessionAffinityConfig::from_env(sid, base_url.is_some());
                CacheOptimizationFacade::build_headers(&config)
            })
            .unwrap_or_default();

        let rig_client = if let Some(url) = base_url {
            let mut builder = openai::CompletionsClient::builder()
                .api_key(api_key)
                .base_url(url);
            if !cache_headers.is_empty() {
                builder = builder.http_headers(cache_headers);
            }
            builder.build().map_err(|e| {
                ProviderError::config(
                    "openai",
                    format!("Failed to build OpenAI client with custom base URL: {e}"),
                )
            })?
        } else {
            openai::CompletionsClient::builder()
                .api_key(api_key)
                .build()
                .map_err(|e| {
                    ProviderError::config("openai", format!("Failed to build OpenAI client: {e}"))
                })?
        };

        // Create completion model using the client
        // PROV-140: thread the per-profile streaming flag onto the model so its
        // `stream()` picks the streaming vs non-streaming request path.
        let completion_model = openai::completion::CompletionModel::new(rig_client.clone(), model)
            .with_stream(streaming);

        Ok(Self {
            completion_model,
            rig_client,
            model_name: model.to_string(),
            base_url: base_url.map(String::from),
            context_window,
            max_output_tokens,
            streaming,
        })
    }

    /// Get the custom base URL if set
    ///
    /// PROV-006: Returns the custom base URL for local model servers.
    /// Returns None if using the default OpenAI API endpoint.
    pub fn base_url(&self) -> Option<&str> {
        self.base_url.as_deref()
    }

    /// Check if this provider is using a custom (local) endpoint
    ///
    /// PROV-006: Returns true if OPENAI_BASE_URL was set.
    pub fn is_local_endpoint(&self) -> bool {
        self.base_url.is_some()
    }

    /// Fetch available models from a local OpenAI-compatible server
    ///
    /// Makes HTTP GET request to {base_url}/v1/models and parses the response.
    /// Returns a list of model IDs available on the server.
    ///
    /// PROV-006: This method is used by the TUI to populate the model selection dialog
    /// when OPENAI_BASE_URL is set.
    ///
    /// PROV-040: Added optional api_key parameter for servers that require authentication
    /// (e.g., Fireworks AI, Together AI). Local servers (vLLM, Ollama) typically don't
    /// require auth for the /models endpoint.
    ///
    /// # Arguments
    /// * `base_url` - The base URL of the local server (e.g., "http://localhost:8888")
    /// * `api_key` - Optional API key for servers requiring authentication
    ///
    /// # Returns
    /// * `Ok(Vec<String>)` - List of model IDs
    /// * `Err(ProviderError)` - If server is unreachable or response is invalid
    pub async fn list_local_models(base_url: &str) -> Result<Vec<String>, ProviderError> {
        Self::list_local_models_with_auth(base_url, None).await
    }

    /// Fetch available models from an OpenAI-compatible server with optional authentication
    ///
    /// PROV-040: Same as list_local_models but accepts an optional API key for servers
    /// that require authentication (Fireworks AI, Together AI, etc.).
    ///
    /// # Arguments
    /// * `base_url` - The base URL of the server (e.g., "https://api.fireworks.ai/inference")
    /// * `api_key` - Optional API key for servers requiring authentication
    ///
    /// # Returns
    /// * `Ok(Vec<String>)` - List of model IDs
    /// * `Err(ProviderError)` - If server is unreachable or response is invalid
    pub async fn list_local_models_with_auth(
        base_url: &str,
        api_key: Option<&str>,
    ) -> Result<Vec<String>, ProviderError> {
        use reqwest::Client;
        use std::time::Duration;

        // Create HTTP client with 5-second timeout
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| {
                ProviderError::api("openai", format!("Failed to create HTTP client: {e}"))
            })?;

        // Normalize base URL: strip trailing slash and /v1 suffix if present
        // This handles both "https://api.fireworks.ai/inference" and
        // "https://api.fireworks.ai/inference/v1" formats
        let trimmed = base_url.trim_end_matches('/');
        let normalized = if let Some(stripped) = trimmed.strip_suffix("/v1") {
            stripped
        } else {
            trimmed
        };

        // Build URL for /v1/models endpoint
        let url = format!("{normalized}/v1/models");

        // Build GET request, optionally with Authorization header (PROV-040)
        let mut request = client.get(&url);
        if let Some(key) = api_key {
            request = request.header("Authorization", format!("Bearer {key}"));
        }

        // Make GET request
        let response = request.send().await.map_err(|e| {
            ProviderError::api(
                "openai",
                format!("Failed to connect to local server at {base_url}: {e}"),
            )
        })?;

        // Check for successful response
        if !response.status().is_success() {
            return Err(ProviderError::api(
                "openai",
                format!(
                    "Local server at {} returned error status: {}",
                    base_url,
                    response.status()
                ),
            ));
        }

        // Parse JSON response
        let body: serde_json::Value = response.json().await.map_err(|e| {
            ProviderError::api(
                "openai",
                format!("Failed to parse models response from {base_url}: {e}"),
            )
        })?;

        // Extract model IDs from data array
        let models = body
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| {
                ProviderError::api(
                    "openai",
                    format!("Invalid models response from {base_url}: missing 'data' array"),
                )
            })?
            .iter()
            .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(String::from))
            .collect();

        Ok(models)
    }

    /// Get the configured rig openai::CompletionsClient
    ///
    /// This client is fully configured with:
    /// - API key authentication
    /// - Custom base URL (if OPENAI_BASE_URL is set)
    ///
    /// Use this client to create rig agents with tools.
    pub fn client(&self) -> &openai::CompletionsClient {
        &self.rig_client
    }

    /// Create a rig Agent with all 10 tools configured for this provider
    ///
    /// This method encapsulates all OpenAI-specific configuration:
    /// - Model name (gpt-4-turbo or custom)
    /// - Max tokens (configurable via OPENAI_MAX_OUTPUT_TOKENS)
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
    /// # TOOL-012 Note
    /// Session ID is now required as the first parameter. This ensures tools know which
    /// session's handler to use at call time, eliminating reliance on thread-local state.
    ///
    /// Returns a fully configured rig::agent::Agent ready for use with RigAgent.
    pub fn create_rig_agent(
        &self,
        session_id: uuid::Uuid,
        preamble: Option<&str>,
        _thinking_config: Option<serde_json::Value>,
    ) -> rig::agent::Agent<openai::completion::CompletionModel> {
        use codelet_tools::facade::{openai_bridge_tool, openai_fspec_tool};
        use codelet_tools::{
            AgentManagerTool, AstGrepRefactorTool, AstGrepTool, BashTool, ConnectMcpTool,
            DeepSearchTool, EditTool, GlobTool, GraphSearchTool, GrepTool, InjectSummaryTool,
            LsTool, ReadTool, RequestUserInputTool, ScheduleTool, SessionSearchTool, WebSearchTool,
            WriteTool,
        };

        // Build agent with all 11 tools using rig's builder pattern (WEB-001: Added WebSearchTool)
        // TOOL-012: Pass session_id to fspec and bridge tools
        // TOOL-014: All tools require session_id for worktree isolation
        // PROV-140: build the completion model explicitly so the per-profile
        // streaming flag is applied to the agent's model (not just re-read from
        // env), keeping streaming vs non-streaming selection deterministic.
        let mut agent_builder =
            openai::completion::CompletionModel::new(self.rig_client.clone(), &self.model_name)
                .with_stream(self.streaming)
                .into_agent_builder()
                .max_tokens(self.max_output_tokens as u64)
            .tool(ReadTool::new(session_id))
            .tool(WriteTool::new(session_id))
            .tool(EditTool::new(session_id))
            .tool(BashTool::new(session_id))
            .tool(GrepTool::new(session_id))
            .tool(GlobTool::new(session_id))
            .tool(LsTool::new(session_id))
            .tool(AstGrepTool::new(session_id)) // TOOL-014: AstGrepTool with session_id for worktree isolation
            .tool(AstGrepRefactorTool::new(session_id)) // TOOL-014: AstGrepRefactorTool with session_id for worktree isolation
            .tool(openai_fspec_tool(session_id)) // TOOL-012: FspecTool with explicit session association
            .tool(openai_bridge_tool(session_id)) // TOOL-012: BridgeTool with explicit session association
            .tool(WebSearchTool::new(session_id)) // WEB-001, TOOL-014: WebSearchTool with session_id
            .tool(ConnectMcpTool::new(session_id)) // MCP-001: Dynamic MCP connections
            .tool(SessionSearchTool::new(session_id)) // AMGR-001: SessionSearch tool
            .tool(GraphSearchTool::new(session_id)) // KGRAPH-003: GraphSearch tool
            .tool(InjectSummaryTool::new(session_id))
            .tool(DeepSearchTool::new(session_id)) // RLM-001: DeepSearch tool
            .tool(AgentManagerTool::new(session_id)) // AMGR-009: AgentManager tool
            .tool(RequestUserInputTool::new(session_id)) // TOOL-017: HITL tool
            .tool(ScheduleTool::new(session_id)); // SCHED-009: Schedule AI tool

        // CONT-002: done() is registered only while auto-continue is armed
        if codelet_tools::is_continue_armed(session_id) {
            agent_builder = agent_builder.tool(codelet_tools::DoneTool::new(session_id));
        }

        // BUG-120: Always set a system prompt with fspec guidance.
        // When a role is provided, it is appended after fspec guidance.
        // Uses the same transform as OpenAISystemPromptFacade for consistency.
        {
            use codelet_tools::facade::prepend_fspec_guidance;
            let preamble_text = preamble.unwrap_or("");
            let effective_preamble = prepend_fspec_guidance(preamble_text);
            agent_builder = agent_builder.preamble(&effective_preamble);
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
        self.context_window
    }

    fn max_output_tokens(&self) -> usize {
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
        let rig_tools = convert_tools_to_rig(tools);

        // Build and send completion request using rig's builder pattern
        let mut builder = CompletionRequestBuilder::new(self.completion_model.clone(), prompt)
            .max_tokens(self.max_output_tokens as u64)
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_default_constants() {
        assert_eq!(CONTEXT_WINDOW, DEFAULT_CONTEXT_WINDOW);
        assert_eq!(MAX_OUTPUT_TOKENS, DEFAULT_MAX_OUTPUT_TOKENS);
    }

    // =========================================================================
    // PROV-039: OpenAI max_output_tokens must read runtime env var
    // Feature: spec/features/stop-reason-lost-in-streaming-output-truncation-silently-treated-as-normal-completion.feature
    // =========================================================================

    /// Scenario: OpenAI max_output_tokens reads runtime environment variable
    #[test]
    #[serial_test::serial]
    fn test_openai_provider_reads_max_output_tokens_env_var() {
        // @step Given the OPENAI_MAX_OUTPUT_TOKENS environment variable is set to "16384"
        std::env::set_var("OPENAI_MAX_OUTPUT_TOKENS", "16384");

        // @step When an OpenAIProvider is created
        let provider =
            OpenAIProvider::from_api_key_with_options("test-key", "gpt-4o", None, None).unwrap();

        // @step Then the instance max_output_tokens is 16384
        assert_eq!(provider.max_output_tokens(), 16384);

        // @step And the returned value is not the compile-time constant 4096
        assert_ne!(provider.max_output_tokens(), DEFAULT_MAX_OUTPUT_TOKENS);

        // Clean up
        std::env::remove_var("OPENAI_MAX_OUTPUT_TOKENS");
    }

    /// Scenario: OpenAI max_output_tokens defaults when env var not set
    #[test]
    #[serial_test::serial]
    fn test_openai_provider_default_max_output_tokens() {
        // Ensure env var is not set
        std::env::remove_var("OPENAI_MAX_OUTPUT_TOKENS");

        let provider =
            OpenAIProvider::from_api_key_with_options("test-key", "gpt-4o", None, None).unwrap();

        // Should use default
        assert_eq!(provider.max_output_tokens(), DEFAULT_MAX_OUTPUT_TOKENS);
    }
}
