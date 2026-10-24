//! Codex Provider implementation using rig
//!
//! Implements the LlmProvider trait for Codex (ChatGPT backend API) communication.
//! Uses OAuth authentication via ~/.codex/auth.json or macOS keychain.

use crate::ProviderAdapter;
use rig::providers::openai;

/// Type alias for the OpenAI Responses API client parameterized with our RefreshingCodexClient.
/// The Responses API uses `input` + `instructions` (from preamble) instead of `messages`,
/// which is required by the Codex endpoint (chatgpt.com/backend-api/codex/responses).
type CodexResponsesClient = openai::Client<RefreshingCodexClient>;

pub use model::{CodexResponsesModel, NativeCapabilities};

mod agent;
pub mod auth_lifecycle;
mod auth_refresh;
mod auth_storage;
pub mod codex_auth;
pub mod codex_device_auth;
pub mod codex_oauth;
pub mod codex_oauth_server;
mod completion;
mod constructors;
mod credential_refresh;
pub mod custom_tools;
mod device_session;
mod diagnostic_redaction;
pub mod diagnostics;
pub mod history;
mod legacy_auth;
pub mod memory_policy;
pub mod metadata;
mod model;
mod network_error;
#[cfg(feature = "test-support")]
pub mod oauth_test_support;
pub mod presentation;
pub mod reconnect;
mod recovery;
mod recovery_auth;
pub mod refreshing_client;
pub mod request_options;
pub mod routing;
pub mod session;
pub mod websocket;

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
    OAuthDirect { account_id: String },
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
    fn with_session(self, state: &crate::session::ProviderSession) -> Self {
        if let Some(extension) = state.extension::<session::CodexSession>("codex") {
            self.with_history(extension.history.clone())
                .with_routing_handle(extension.routing.clone())
                .with_metadata_handle(extension.metadata.clone())
                .with_recovery_handle(extension.recovery.clone())
        } else {
            self
        }
    }

    fn provider_name(&self) -> &'static str {
        "codex"
    }
}

// ---------------------------------------------------------------------------
// LIMITS-003: ModelLimitsResolver for Codex
// ---------------------------------------------------------------------------

impl crate::model_limits::ModelLimitsResolver for CodexProvider {
    /// Codex trusts registry values — no hard ceiling.
    fn max_context_window(&self) -> Option<usize> {
        None
    }

    /// Codex trusts registry values — no hard ceiling.
    fn max_output_tokens_limit(&self) -> Option<usize> {
        None
    }

    /// Default context window when no registry data is available.
    fn default_context_window(&self) -> usize {
        0
    }

    /// Default max output tokens when no registry data is available.
    fn default_max_output_tokens(&self) -> usize {
        0
    }

    /// The Codex backend API **rejects** `max_output_tokens` with HTTP 400:
    /// `{"detail":"Unsupported parameter: max_output_tokens"}`.
    fn should_send_max_output_tokens(&self) -> bool {
        false
    }
}

impl CodexProvider {
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
    #[cfg(test)]
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
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests;
