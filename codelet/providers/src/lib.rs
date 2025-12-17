//! Provider Management bounded context
//!
//! Multi-provider LLM abstraction.

pub mod claude;
pub mod codex;
mod credentials;
pub mod gemini;
mod manager;
pub mod openai;

use anyhow::Result;
use async_trait::async_trait;

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
    async fn complete(&self, messages: &[codelet_common::Message]) -> Result<String>;

    /// Send a completion request with tool definitions
    async fn complete_with_tools(
        &self,
        messages: &[codelet_common::Message],
        tools: &[ToolDefinition],
    ) -> Result<CompletionResponse>;
}
