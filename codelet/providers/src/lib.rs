//! Provider Management bounded context
//!
//! Multi-provider LLM abstraction.

pub mod adapter;
pub mod cache_token_extractor;
pub mod caching_client;
pub mod claude;
pub mod codex;
mod credentials;
pub mod error;
pub mod gemini;
mod manager;
pub mod openai;

pub use adapter::{
    convert_assistant_content, convert_tools_to_rig, detect_credential_from_env,
    extract_prompt_data, extract_text_from_content, validate_api_key_static, ProviderAdapter,
};
pub use error::ProviderError;

use async_trait::async_trait;

pub use cache_token_extractor::{extract_cache_tokens_from_sse, CacheTokenExtractor};
pub use caching_client::{
    should_transform_request, transform_request_body, transform_system_prompt,
    transform_user_message_cache_control,
};
pub use claude::{build_cached_system_prompt, AuthMode, CacheControl, ClaudeProvider};
pub use codex::CodexProvider;
pub use credentials::ProviderCredentials;
pub use gemini::GeminiProvider;
pub use manager::{ProviderManager, ProviderType};
pub use openai::OpenAIProvider;

use codelet_common::MessageContent;
use codelet_tools::ToolDefinition;

/// Response from an LLM completion with tool support
#[derive(Debug, Clone)]
pub struct CompletionResponse {
    /// Content of the response (text or tool calls)
    pub content: MessageContent,
    /// Reason the model stopped generating
    pub stop_reason: StopReason,
}

/// Reason why the model stopped generating
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// Natural end of response
    EndTurn,
    /// Model wants to use tools
    ToolUse,
    /// Hit maximum token limit
    MaxTokens,
}

/// LLM Provider trait for multi-provider abstraction
///
/// All methods return `Result<_, ProviderError>` for typed error handling.
/// This enables automatic retry logic for rate limit errors and proper
/// error categorization (authentication, API, rate limit, etc.).
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Provider name
    fn name(&self) -> &str;

    /// Model identifier
    fn model(&self) -> &str;

    /// Context window size in tokens
    fn context_window(&self) -> usize;

    /// Maximum output tokens
    fn max_output_tokens(&self) -> usize;

    /// Whether this provider supports prompt caching
    fn supports_caching(&self) -> bool;

    /// Whether this provider supports streaming responses
    fn supports_streaming(&self) -> bool;

    /// Send a completion request
    async fn complete(
        &self,
        messages: &[codelet_common::Message],
    ) -> Result<String, ProviderError>;

    /// Send a completion request with tool definitions
    async fn complete_with_tools(
        &self,
        messages: &[codelet_common::Message],
        tools: &[ToolDefinition],
    ) -> Result<CompletionResponse, ProviderError>;
}
