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

    /// Build additional_params JSON with reasoning config matching codex-rs format.
    ///
    /// PROV-037: This is the fix for the missing reasoning configuration that prevented
    /// GPT-5.x Codex models from performing multi-step agentic reasoning and tool use.
    ///
    /// If `thinking_config` contains a `reasoning` object, it is merged into the params.
    /// If `thinking_config` is None or doesn't contain `reasoning`, default reasoning
    /// (effort: "high", summary: "auto") is applied, because Codex models always need
    /// reasoning enabled for agentic tool use.
    ///
    /// Reference: codex-rs core/src/client.rs lines 500-553
    /// Wire format matches: reasoning, include, store
    fn build_reasoning_params(thinking_config: Option<&serde_json::Value>) -> serde_json::Value {
        let mut params = serde_json::json!({
            "store": false,
            "include": ["reasoning.encrypted_content"],
        });

        // Merge reasoning config from thinking_config if present
        let has_reasoning = if let Some(config) = thinking_config {
            if let Some(reasoning) = config.get("reasoning") {
                params["reasoning"] = reasoning.clone();
                true
            } else {
                false
            }
        } else {
            false
        };

        // Default reasoning if none provided (Codex models always need reasoning)
        if !has_reasoning {
            params["reasoning"] = serde_json::json!({
                "effort": "high",
                "summary": "auto"
            });
        }

        params
    }

    /// Create a rig Agent with all tools configured for this provider using Codex-native facades (TOOL-015)
    ///
    /// This method encapsulates all Codex-specific configuration:
    /// - Model name (GPT or compatible)
    /// - Codex-native tool names (shell_command, read_file, list_dir, grep_files) via facade wrappers
    /// - Codex-native apply_patch for all file modifications (BUG-105: replaces Write/Edit)
    /// - Non-Codex tools kept with standard naming (AstGrep, AstGrepRefactor)
    /// - Fspec/Bridge tools reusing OpenAI facades
    ///
    /// # Arguments
    /// * `session_id` - Session UUID for tool handler lookup (TOOL-012, TOOL-014)
    /// * `preamble` - Optional system prompt/preamble for the agent
    /// * `thinking_config` - Optional thinking configuration JSON (PROV-037: now consumed for reasoning)
    ///
    /// Returns a fully configured rig::agent::Agent ready for use with RigAgent.
    pub fn create_rig_agent(
        &self,
        session_id: uuid::Uuid,
        preamble: Option<&str>,
        thinking_config: Option<serde_json::Value>,
    ) -> rig::agent::Agent<CodexResponsesModel> {
        use codelet_tools::{
            AstGrepRefactorTool, AstGrepTool, ApplyPatchTool, WebSearchTool,
            ConnectMcpTool, SessionSearchTool, InjectSummaryTool, DeepSearchTool,
        };
        use codelet_tools::facade::{
            BashToolFacadeWrapper, CodexExecCommandFacade, CodexGrepFilesFacade, CodexListDirFacade,
            CodexReadFileFacade, CodexRequestUserInputFacade, CodexShellCommandFacade,
            CodexShellFacade, CodexViewImageFacade, CodexWriteStdinFacade,
            ExecToolFacadeWrapper, FileToolFacadeWrapper, HitlToolFacadeWrapper,
            LsToolFacadeWrapper, SearchToolFacadeWrapper, codex_bridge_tool, codex_fspec_tool,
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

        // @step And view_image uses FileToolFacadeWrapper with CodexViewImageFacade
        // View image facade: Read → view_image (maps Codex-native view_image to ReadTool)
        let view_image = FileToolFacadeWrapper::new(Arc::new(CodexViewImageFacade), session_id);

        // @step And list_dir uses LsToolFacadeWrapper with CodexListDirFacade
        // Directory listing facade: Ls → list_dir
        let list_dir = LsToolFacadeWrapper::new(Arc::new(CodexListDirFacade), session_id);

        // @step And grep_files uses SearchToolFacadeWrapper with CodexGrepFilesFacade
        // Grep facade: Grep → grep_files
        let grep_files = SearchToolFacadeWrapper::new(Arc::new(CodexGrepFilesFacade), session_id);

        // BUG-114: Exec facades: UnifiedExec → shell, exec_command
        // shell: execvp-style (argv array, no shell interpretation)
        let shell = ExecToolFacadeWrapper::new(Arc::new(CodexShellFacade), session_id);
        // exec_command: PTY-capable with yield-and-resume
        let exec_command = ExecToolFacadeWrapper::new(Arc::new(CodexExecCommandFacade), session_id);
        // BUG-115: write_stdin: send input to running PTY session or poll for output
        let write_stdin = ExecToolFacadeWrapper::new(Arc::new(CodexWriteStdinFacade), session_id);
        // BUG-116: request_user_input: Codex-native HITL facade
        let request_user_input = HitlToolFacadeWrapper::new(Arc::new(CodexRequestUserInputFacade), session_id);

        // Build agent with Codex-native tools using rig's builder pattern
        // TOOL-015: Uses FacadeToolWrapper for tools with Codex equivalents
        // BUG-107: glob removed — not part of native Codex CLI tool set
        // NOTE: Do NOT set .max_tokens() — the Codex backend API rejects
        // `max_output_tokens` with 400: {"detail":"Unsupported parameter: max_output_tokens"}
        let mut agent_builder = self
            .rig_client
            .agent(&self.model_name)
            // Codex-native facade tools
            .tool(shell_command)   // Codex-native shell_command
            .tool(read_file)       // Codex-native read_file
            .tool(view_image)      // Codex-native view_image (BUG-112: facade, not standalone)
            .tool(list_dir)        // Codex-native list_dir
            .tool(grep_files)      // Codex-native grep_files
            .tool(shell)           // BUG-114: Codex-native shell (execvp argv)
            .tool(exec_command)    // BUG-114: Codex-native exec_command (PTY)
            .tool(write_stdin)     // BUG-115: Codex-native write_stdin (send input to PTY session)
            // @step And AstGrep, AstGrepRefactor remain as direct tool registrations
            // BUG-105: Replace WriteTool + EditTool with Codex-native apply_patch.
            // Codex models are trained to use apply_patch for all file modifications.
            // Keeping Write/Edit would cause the model to choose between competing interfaces.
            .tool(ApplyPatchTool::new(session_id))  // Codex-native apply_patch
            .tool(AstGrepTool::new(session_id))
            .tool(AstGrepRefactorTool::new(session_id))
            .tool(WebSearchTool::new(session_id))
            // Fspec/Bridge tools reusing OpenAI facades (TOOL-012)
            .tool(codex_fspec_tool(session_id))
            .tool(codex_bridge_tool(session_id))
            .tool(ConnectMcpTool::new(session_id)) // MCP-001: Dynamic MCP connections
            .tool(SessionSearchTool::new(session_id)) // AMGR-001: SessionSearch tool
            .tool(InjectSummaryTool::new(session_id))
            .tool(DeepSearchTool::new(session_id)) // RLM-001: DeepSearch tool
            .tool(request_user_input); // BUG-116: Codex-native HITL facade

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
        //
        // PROV-037: Build additional_params with reasoning config matching codex-rs format.
        // Without reasoning, GPT-5.x Codex models cannot perform multi-step agentic
        // reasoning and tool use (the model produces ~33 tokens of commentary with
        // 0 reasoning tokens and no function_call output items).
        //
        // Reference: codex-rs core/src/client.rs lines 500-553
        let additional = Self::build_reasoning_params(thinking_config.as_ref());
        agent_builder = agent_builder.additional_params(additional);

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

#[cfg(test)]
mod tests {
    use super::CodexProvider;
    use uuid::Uuid;

    // =========================================================================
    // Unit tests for build_reasoning_params — the core function that constructs
    // the reasoning JSON matching codex-rs wire format.
    // Feature: spec/features/codex-provider-reasoning-configuration.feature
    // =========================================================================

    #[test]
    fn build_reasoning_params_with_high_effort_config() {
        // @step Given I have a thinking_config with reasoning effort "high" and summary "auto"
        let config = serde_json::json!({"reasoning": {"effort": "high", "summary": "auto"}});

        // @step When I call build_reasoning_params with the thinking_config
        let params = CodexProvider::build_reasoning_params(Some(&config));

        // @step Then the params should contain reasoning.effort "high"
        assert_eq!(params["reasoning"]["effort"], "high");
        // @step And the params should contain reasoning.summary "auto"
        assert_eq!(params["reasoning"]["summary"], "auto");
        // @step And the params should contain store as false
        assert_eq!(params["store"], false);
        // @step And the params should contain include with "reasoning.encrypted_content"
        assert_eq!(params["include"][0], "reasoning.encrypted_content");
    }

    #[test]
    fn build_reasoning_params_with_medium_effort_config() {
        let config = serde_json::json!({"reasoning": {"effort": "medium", "summary": "auto"}});
        let params = CodexProvider::build_reasoning_params(Some(&config));

        assert_eq!(params["reasoning"]["effort"], "medium");
        assert_eq!(params["reasoning"]["summary"], "auto");
        assert_eq!(params["store"], false);
        assert_eq!(params["include"][0], "reasoning.encrypted_content");
    }

    #[test]
    fn build_reasoning_params_with_low_effort_config() {
        let config = serde_json::json!({"reasoning": {"effort": "low", "summary": "auto"}});
        let params = CodexProvider::build_reasoning_params(Some(&config));

        assert_eq!(params["reasoning"]["effort"], "low");
        assert_eq!(params["reasoning"]["summary"], "auto");
    }

    #[test]
    fn build_reasoning_params_defaults_to_high_when_none() {
        // @step When I call build_reasoning_params with None as thinking_config
        let params = CodexProvider::build_reasoning_params(None);

        // @step Then the params should contain default reasoning.effort "high"
        assert_eq!(params["reasoning"]["effort"], "high");
        // @step And the params should contain default reasoning.summary "auto"
        assert_eq!(params["reasoning"]["summary"], "auto");
        // @step And store and include should still be present
        assert_eq!(params["store"], false);
        assert_eq!(params["include"][0], "reasoning.encrypted_content");
    }

    #[test]
    fn build_reasoning_params_defaults_to_high_when_empty_config() {
        // Empty config (from ThinkingLevel::Off flowing through as Some({}))
        // should still get default reasoning because Codex models always need it
        let config = serde_json::json!({});
        let params = CodexProvider::build_reasoning_params(Some(&config));

        assert_eq!(params["reasoning"]["effort"], "high");
        assert_eq!(params["reasoning"]["summary"], "auto");
    }

    /// Feature: spec/features/codex-native-tool-facades.feature
    ///
    /// Scenario: Codex agent does not expose non-native glob tool
    /// BUG-107: glob is NOT part of the native Codex CLI tool set
    #[tokio::test]
    async fn create_rig_agent_does_not_expose_non_native_glob_tool() {
        // @step Given a Codex agent built with create_rig_agent
        let provider = CodexProvider::from_api_key("sk-proj-test-key-12345", "gpt-5.1-codex")
            .expect("Provider should initialize");

        let session_id = Uuid::new_v4();
        let agent = provider.create_rig_agent(session_id, None, None);

        // @step When the agent tool definitions are inspected
        let tool_defs = agent
            .tool_server_handle
            .get_tool_defs(None)
            .await
            .expect("Tool definitions should be available");

        let tool_names: Vec<&str> = tool_defs.iter().map(|def| def.name.as_str()).collect();

        // @step Then the tool list does not contain "glob"
        assert!(!tool_names.contains(&"glob"), "Codex agent should NOT expose non-native 'glob' tool, but found: {:?}", tool_names);
        assert!(!tool_names.contains(&"Glob"), "Codex agent should NOT expose 'Glob' tool either");

        // @step And the tool list contains "shell_command"
        assert!(tool_names.contains(&"shell_command"));
        // @step And the tool list contains "read_file"
        assert!(tool_names.contains(&"read_file"));
        // @step And the tool list contains "list_dir"
        assert!(tool_names.contains(&"list_dir"));
        // @step And the tool list contains "grep_files"
        assert!(tool_names.contains(&"grep_files"));
        // @step And the tool list contains "apply_patch"
        assert!(tool_names.contains(&"apply_patch"));

        // @step And the agent does not have "Write" or "Edit" tools registered
        // BUG-105 Rule [6]: WriteTool and EditTool removed to prevent competing edit interfaces
        assert!(!tool_names.contains(&"Write"), "Codex agent should NOT expose 'Write' tool (BUG-105), but found: {tool_names:?}");
        assert!(!tool_names.contains(&"Edit"), "Codex agent should NOT expose 'Edit' tool (BUG-105), but found: {tool_names:?}");

        // @step Then the agent's tool list includes a tool named "view_image"
        // BUG-112: Codex-native view_image tool for viewing local image files
        assert!(tool_names.contains(&"view_image"), "Codex agent should expose 'view_image' tool (BUG-112), but found: {tool_names:?}");
    }

    /// Feature: spec/features/codex-shell-exec-facades.feature
    ///
    /// Scenario: Both facades are registered in Codex create_rig_agent
    #[tokio::test]
    async fn create_rig_agent_exposes_shell_and_exec_command_tools() {
        // @step Given a CodexProvider with create_rig_agent configured
        let provider = CodexProvider::from_api_key("sk-proj-test-key-12345", "gpt-5.1-codex")
            .expect("Provider should initialize");

        // @step When the agent is built with session_id
        let session_id = Uuid::new_v4();
        let agent = provider.create_rig_agent(session_id, None, None);

        let tool_defs = agent
            .tool_server_handle
            .get_tool_defs(None)
            .await
            .expect("Tool definitions should be available");

        let tool_names: Vec<&str> = tool_defs.iter().map(|def| def.name.as_str()).collect();

        // @step Then the tool list contains "shell"
        assert!(tool_names.contains(&"shell"), "Codex agent should expose 'shell' tool (BUG-114), but found: {tool_names:?}");

        // @step And the tool list contains "exec_command"
        assert!(tool_names.contains(&"exec_command"), "Codex agent should expose 'exec_command' tool (BUG-114), but found: {tool_names:?}");

        // @step And shell uses ExecToolFacadeWrapper with CodexShellFacade
        // Verify shell tool has the correct schema (array command)
        let shell_def = tool_defs.iter().find(|d| d.name == "shell").unwrap();
        assert_eq!(shell_def.parameters["properties"]["command"]["type"], "array");
        assert_eq!(shell_def.parameters["required"][0], "command");

        // @step And exec_command uses ExecToolFacadeWrapper with CodexExecCommandFacade
        // Verify exec_command tool has the correct schema (string cmd)
        let exec_def = tool_defs.iter().find(|d| d.name == "exec_command").unwrap();
        assert_eq!(exec_def.parameters["properties"]["cmd"]["type"], "string");
        assert_eq!(exec_def.parameters["required"][0], "cmd");

        // @step And write_stdin uses ExecToolFacadeWrapper with CodexWriteStdinFacade (BUG-115)
        assert!(tool_names.contains(&"write_stdin"), "Codex agent should expose 'write_stdin' tool (BUG-115), but found: {tool_names:?}");
        let write_def = tool_defs.iter().find(|d| d.name == "write_stdin").unwrap();
        assert_eq!(write_def.parameters["properties"]["session_id"]["type"], "number");
        assert_eq!(write_def.parameters["required"][0], "session_id");
    }

    #[test]
    fn build_reasoning_params_preserves_custom_effort_from_thinking_config() {
        // Verify that user's thinking level preference is respected when provided
        let config = serde_json::json!({"reasoning": {"effort": "low", "summary": "auto"}});
        let params = CodexProvider::build_reasoning_params(Some(&config));

        // Should use the provided effort, NOT the default "high"
        assert_eq!(
            params["reasoning"]["effort"], "low",
            "User's effort level should be preserved, not overridden with default"
        );
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
            // PROV-037: Include reasoning config in complete_with_tools, not just create_rig_agent.
            // Without reasoning, GPT-5.x Codex models produce ~33 tokens of commentary
            // with 0 reasoning tokens and no function_call output items.
            .additional_params(Self::build_reasoning_params(None));

        // Send request and get response
        let response = builder
            .send()
            .await
            .map_err(|e| ProviderError::api("codex", format!("Rig completion failed: {e}")))?;

        // Convert rig response to our CompletionResponse format
        self.rig_response_to_completion(response)
    }
}
