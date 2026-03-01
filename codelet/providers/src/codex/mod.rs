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

/// Type alias for the OpenAI Responses API client parameterized with our RefreshingCodexClient.
/// The Responses API uses `input` + `instructions` (from preamble) instead of `messages`,
/// which is required by the Codex endpoint (chatgpt.com/backend-api/codex/responses).
type CodexResponsesClient = openai::Client<RefreshingCodexClient>;

/// Type alias for the Responses API completion model.
type CodexResponsesModel = openai::responses_api::ResponsesCompletionModel<RefreshingCodexClient>;

pub mod codex_auth;
pub mod codex_device_auth;
pub mod codex_oauth;
pub mod codex_oauth_server;
pub mod refreshing_client;

use refreshing_client::RefreshingCodexClient;

/// Base instructions for the Codex Responses API.
/// This is the exact system prompt from the official Codex CLI
/// (codex-rs/protocol/src/prompts/base_instructions/default.md).
/// The Codex backend API **requires** a non-empty `instructions` field;
/// omitting it returns 400 `{"detail":"Instructions are required"}`.
pub const CODEX_BASE_INSTRUCTIONS: &str = include_str!("codex_base_instructions.md");

/// GPT-5.1 Codex context window size (272K tokens)
pub const CONTEXT_WINDOW: usize = 272_000;

/// GPT-5.1 Codex max output tokens (CTX-002) (assumption: same as GPT-4)
pub const MAX_OUTPUT_TOKENS: usize = 4096;

/// Authentication mode for the Codex provider
#[derive(Clone, Debug)]
pub enum CodexAuthMode {
    /// Legacy mode: uses OpenAI API key from token exchange
    ApiKey,
    /// Direct Codex API mode: uses Bearer access_token to chatgpt.com/backend-api/codex
    OAuthDirect {
        account_id: String,
    },
}

/// Codex Provider for ChatGPT Backend API (using rig with OAuth)
///
/// Uses the OpenAI Responses API (not Chat Completions API) because the Codex
/// endpoint requires the `instructions` field and `input` array format.
/// The preamble is mapped to `instructions` by rig's Responses API implementation.
#[derive(Clone)]
pub struct CodexProvider {
    completion_model: CodexResponsesModel,
    rig_client: CodexResponsesClient,
    model_name: String,
    auth_mode: CodexAuthMode,
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
    /// Two modes:
    /// 1. **Direct Codex API** (preferred): If OAuth tokens with account_id exist,
    ///    route requests to chatgpt.com/backend-api/codex with Bearer token.
    /// 2. **Legacy token-exchange**: If cached OPENAI_API_KEY exists, use standard
    ///    OpenAI API. If not, refresh tokens and exchange for API key.
    ///
    /// Model MUST be provided via CODEX_MODEL environment variable.
    pub fn new() -> Result<Self, ProviderError> {
        // Model is REQUIRED via CODEX_MODEL env var
        let model_name = std::env::var("CODEX_MODEL").map_err(|_| {
            ProviderError::config(
                "codex",
                "Model is required. Set CODEX_MODEL environment variable.",
            )
        })?;

        // Read existing auth data
        let auth = codex_auth::read_codex_auth()
            .map_err(|e| ProviderError::auth("codex", format!("{e}")))?;

        if let Some(auth_data) = &auth {
            // Mode 1: Direct Codex API - use OAuth tokens with access_token + account_id
            if let Some(tokens) = &auth_data.tokens {
                if !tokens.access_token.is_empty() && !tokens.account_id.is_empty() {
                    return Self::from_oauth_tokens(
                        &tokens.access_token,
                        &tokens.refresh_token,
                        &tokens.account_id,
                        Some(0), // Force immediate refresh — tokens from disk are of unknown age
                        codex_oauth::CODEX_ISSUER,
                        &model_name,
                    );
                }
            }

            // Mode 2: Legacy - use cached OpenAI API key
            if let Some(api_key) = &auth_data.openai_api_key {
                return Self::from_api_key(api_key, &model_name);
            }
        }

        // Fallback: try the full refresh + exchange flow (legacy mode)
        let api_key = codex_auth::get_codex_api_key_sync()
            .map_err(|e| ProviderError::auth("codex", format!("{e}")))?;

        Self::from_api_key(&api_key, &model_name)
    }

    /// Create a new CodexProvider with an explicit API key and model
    ///
    /// Uses shared validate_api_key_static() helper (REFAC-013).
    /// PROV-016: Uses RefreshingCodexClient in ApiKey pass-through mode.
    /// Uses the Responses API client so requests go to /responses (rewritten
    /// to the Codex endpoint) with `instructions` from preamble.
    pub fn from_api_key(api_key: &str, model: &str) -> Result<Self, ProviderError> {
        // Use shared validation helper (REFAC-013)
        validate_api_key_static("codex", api_key)?;

        // PROV-016: Create RefreshingCodexClient in API key pass-through mode
        let http_client = RefreshingCodexClient::new_api_key();

        // Build rig Responses API client with RefreshingCodexClient as HTTP backend
        let rig_client = CodexResponsesClient::builder()
            .api_key(api_key)
            .http_client(http_client)
            .build()
            .map_err(|e| {
                ProviderError::config("codex", format!("Failed to build Codex client: {e}"))
            })?;

        // Create Responses API completion model using the client
        let completion_model = CodexResponsesModel::new(rig_client.clone(), model);

        Ok(Self {
            completion_model,
            rig_client,
            model_name: model.to_string(),
            auth_mode: CodexAuthMode::ApiKey,
        })
    }

    /// Create a new CodexProvider using OAuth tokens (Direct Codex API mode)
    ///
    /// This mode routes requests directly to chatgpt.com/backend-api/codex/responses
    /// using Bearer access_token and ChatGPT-Account-Id header.
    ///
    /// PROV-016: Uses RefreshingCodexClient in OAuth mode for dynamic token refresh,
    /// URL rewriting, and header injection. No static base_url or headers needed —
    /// RefreshingCodexClient handles everything dynamically per-request.
    ///
    /// Uses the Responses API client so the request body contains `instructions`
    /// (mapped from preamble) and `input` (mapped from chat history), which is
    /// the format required by the Codex endpoint.
    ///
    /// # Arguments
    /// * `access_token` - OAuth access token (used as Bearer token)
    /// * `refresh_token` - OAuth refresh token (used for token refresh)
    /// * `account_id` - ChatGPT account ID (sent as ChatGPT-Account-Id header)
    /// * `expires_in_secs` - Token expiry in seconds (None defaults to 3600s)
    /// * `issuer_url` - OAuth issuer URL for token refresh (e.g., CODEX_ISSUER)
    /// * `model` - Model name (e.g., "gpt-5.1-codex")
    pub fn from_oauth_tokens(
        access_token: &str,
        refresh_token: &str,
        account_id: &str,
        expires_in_secs: Option<u64>,
        issuer_url: &str,
        model: &str,
    ) -> Result<Self, ProviderError> {
        // PROV-016: Create RefreshingCodexClient in OAuth mode with token refresh support
        let http_client = RefreshingCodexClient::new_oauth(
            access_token.to_string(),
            refresh_token.to_string(),
            account_id.to_string(),
            expires_in_secs,
            issuer_url.to_string(),
        );

        // Build rig Responses API client with RefreshingCodexClient as HTTP backend.
        // - .api_key("dummy") is required by rig but RefreshingCodexClient strips and replaces it
        // - No .base_url() needed — RefreshingCodexClient rewrites URLs dynamically
        // - No .http_headers() needed — RefreshingCodexClient injects headers dynamically
        let rig_client = CodexResponsesClient::builder()
            .api_key("dummy")
            .http_client(http_client)
            .build()
            .map_err(|e| {
                ProviderError::config("codex", format!("Failed to build Codex OAuth client: {e}"))
            })?;

        let completion_model = CodexResponsesModel::new(rig_client.clone(), model);

        Ok(Self {
            completion_model,
            rig_client,
            model_name: model.to_string(),
            auth_mode: CodexAuthMode::OAuthDirect {
                account_id: account_id.to_string(),
            },
        })
    }

    /// Get the configured rig openai::Client (Responses API)
    pub fn client(&self) -> &CodexResponsesClient {
        &self.rig_client
    }

    /// Get the authentication mode (ApiKey or OAuthDirect)
    pub fn auth_mode(&self) -> &CodexAuthMode {
        &self.auth_mode
    }

    /// Create a rig Agent with all tools configured for this provider using Codex-native facades (TOOL-015)
    ///
    /// This method encapsulates all Codex-specific configuration:
    /// - Model name (GPT or compatible)
    /// - Codex-native tool names (shell_command, read_file, list_dir, grep_files) via facade wrappers
    /// - Non-Codex tools kept with standard naming (Write, Edit, Glob, AstGrep, AstGrepRefactor)
    /// - Fspec/Bridge tools reusing OpenAI facades
    ///
    /// # Arguments
    /// * `session_id` - Session UUID for tool handler lookup (TOOL-012, TOOL-014)
    /// * `preamble` - Optional system prompt/preamble for the agent
    /// * `_thinking_config` - Optional thinking configuration JSON (TOOL-010, currently unused for Codex)
    ///
    /// Returns a fully configured rig::agent::Agent ready for use with RigAgent.
    pub fn create_rig_agent(
        &self,
        session_id: uuid::Uuid,
        preamble: Option<&str>,
        _thinking_config: Option<serde_json::Value>,
    ) -> rig::agent::Agent<CodexResponsesModel> {
        use codelet_tools::{
            AstGrepRefactorTool, AstGrepTool, EditTool, GlobTool, WebSearchTool, WriteTool,
        };
        use codelet_tools::facade::{
            BashToolFacadeWrapper, CodexGrepFilesFacade, CodexListDirFacade, CodexReadFileFacade,
            CodexShellCommandFacade, FileToolFacadeWrapper, LsToolFacadeWrapper,
            SearchToolFacadeWrapper, codex_bridge_tool, codex_fspec_tool,
        };
        use rig::client::CompletionClient;
        use std::sync::Arc;

        // Create Codex-native tool facades (TOOL-015)
        // These provide Codex-native tool names and schemas that GPT-5.1-codex was trained on

        // @step Given a CodexProvider with create_rig_agent configured
        // @step When the agent is built with session_id

        // @step Then shell_command uses BashToolFacadeWrapper with CodexShellCommandFacade
        // Shell command facade: Bash → shell_command
        let shell_command = BashToolFacadeWrapper::new(Arc::new(CodexShellCommandFacade), session_id);

        // @step And read_file uses FileToolFacadeWrapper with CodexReadFileFacade
        // File read facade: Read → read_file
        let read_file = FileToolFacadeWrapper::new(Arc::new(CodexReadFileFacade), session_id);

        // @step And list_dir uses LsToolFacadeWrapper with CodexListDirFacade
        // Directory listing facade: Ls → list_dir
        let list_dir = LsToolFacadeWrapper::new(Arc::new(CodexListDirFacade), session_id);

        // @step And grep_files uses SearchToolFacadeWrapper with CodexGrepFilesFacade
        // Grep facade: Grep → grep_files
        let grep_files = SearchToolFacadeWrapper::new(Arc::new(CodexGrepFilesFacade), session_id);

        // Build agent with Codex-native tools using rig's builder pattern
        // TOOL-015: Uses FacadeToolWrapper for tools with Codex equivalents
        // NOTE: Do NOT set .max_tokens() — the Codex backend API rejects
        // `max_output_tokens` with 400: {"detail":"Unsupported parameter: max_output_tokens"}
        let mut agent_builder = self
            .rig_client
            .agent(&self.model_name)
            // Codex-native facade tools
            .tool(shell_command)   // Codex-native shell_command
            .tool(read_file)       // Codex-native read_file
            .tool(list_dir)        // Codex-native list_dir
            .tool(grep_files)      // Codex-native grep_files
            // @step And Write, Edit, Glob, AstGrep, AstGrepRefactor remain as direct tool registrations
            // Tools without Codex equivalent - keep standard naming
            .tool(WriteTool::new(session_id))
            .tool(EditTool::new(session_id))
            .tool(GlobTool::new(session_id))
            .tool(AstGrepTool::new(session_id))
            .tool(AstGrepRefactorTool::new(session_id))
            .tool(WebSearchTool::new(session_id))
            // Fspec/Bridge tools reusing OpenAI facades (TOOL-012)
            .tool(codex_fspec_tool(session_id))
            .tool(codex_bridge_tool(session_id));

        // The Codex backend API REQUIRES non-empty `instructions` in every
        // Responses API request.  The rig layer maps preamble → instructions,
        // so we must ALWAYS set a preamble.  Use the caller's value if given,
        // otherwise fall back to the official Codex base instructions.
        let effective_preamble = preamble.unwrap_or(CODEX_BASE_INSTRUCTIONS);
        agent_builder = agent_builder.preamble(effective_preamble);

        // The Codex backend API REQUIRES `store: false` in the request body.
        // Omitting it or setting it to true returns 400:
        //   {"detail":"Store must be set to false"}
        // Both the official codex-cli and opencode set this explicitly.
        agent_builder = agent_builder.additional_params(serde_json::json!({"store": false}));

        agent_builder.build()
    }

    /// Convert rig Responses API response to our CompletionResponse format
    fn rig_response_to_completion(
        &self,
        response: rig::completion::CompletionResponse<openai::responses_api::CompletionResponse>,
    ) -> Result<CompletionResponse, ProviderError> {
        // Convert rig AssistantContent to our ContentPart format using shared helper (REFAC-013)
        let content_parts = convert_assistant_content(response.choice, "codex")?;

        // Map Responses API status to our StopReason enum
        let stop_reason = match response.raw_response.status {
            openai::responses_api::ResponseStatus::Completed => {
                // Check if any output contains a function call (tool use)
                let has_tool_calls = response.raw_response.output.iter().any(|o| {
                    matches!(o, openai::responses_api::Output::FunctionCall(_))
                });
                if has_tool_calls {
                    StopReason::ToolUse
                } else {
                    StopReason::EndTurn
                }
            }
            openai::responses_api::ResponseStatus::Incomplete => StopReason::MaxTokens,
            openai::responses_api::ResponseStatus::Failed => {
                let err_msg = response
                    .raw_response
                    .error
                    .as_ref()
                    .map(|e| e.message.as_str())
                    .unwrap_or("unknown error");
                warn!(error = %err_msg, "Codex API response failed");
                StopReason::EndTurn
            }
            other => {
                warn!(status = ?other, "Unexpected response status from Codex API");
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
        // PROV-019: Codex backend API REQUIRES non-empty `instructions`.
        // Always set preamble — use extracted preamble or fall back to base instructions.
        let effective_preamble = preamble.unwrap_or_else(|| CODEX_BASE_INSTRUCTIONS.to_string());
        // NOTE: Do NOT set .max_tokens() — the Codex backend API rejects
        // `max_output_tokens` with 400: {"detail":"Unsupported parameter: max_output_tokens"}
        // Both codex-cli and opencode omit this parameter for Codex endpoints.
        let builder = CompletionRequestBuilder::new(self.completion_model.clone(), prompt)
            .tools(rig_tools)
            .preamble(effective_preamble)
            // Codex backend API REQUIRES `store: false`; omitting it returns 400:
            //   {"detail":"Store must be set to false"}
            .additional_params(serde_json::json!({"store": false}));

        // Send request and get response
        let response = builder
            .send()
            .await
            .map_err(|e| ProviderError::api("codex", format!("Rig completion failed: {e}")))?;

        // Convert rig response to our CompletionResponse format
        self.rig_response_to_completion(response)
    }
}
