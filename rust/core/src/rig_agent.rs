//! Rig Agent integration for automatic multi-turn tool execution
//!
//! Replaces the manual Runner loop with rig::agent::Agent which handles
//! multi-turn tool calling automatically with configurable depth control.

use anyhow::Result;
use rig::agent::{Agent, StreamingPromptHook};
use rig::completion::{CompletionModel, GetTokenUsage, Prompt};
use rig::streaming::StreamingPrompt;
use rig::wasm_compat::WasmCompatSend;
use tracing::debug;

/// Default maximum depth for multi-turn tool execution
/// Set to usize::MAX - 1 to effectively disable the limit
/// (rig checks: current_max_depth > self.max_depth + 1, so this prevents overflow)
pub const DEFAULT_MAX_DEPTH: usize = usize::MAX - 1;

/// Rig-based agent with automatic multi-turn tool execution
///
/// Generic over the completion model type M to support multiple providers
/// (Claude, OpenAI, Google, etc.)
pub struct RigAgent<M>
where
    M: CompletionModel + 'static,
{
    /// The underlying rig agent with tools
    agent: Agent<M>,
    /// Maximum depth for multi-turn execution
    max_depth: usize,
}

impl<M> RigAgent<M>
where
    M: CompletionModel + 'static,
{
    /// Create a new RigAgent with a pre-built rig Agent
    ///
    /// This is the provider-agnostic constructor. Providers are responsible
    /// for building the rig::agent::Agent with their specific client, model,
    /// and configuration. This ensures RigAgent stays decoupled from any
    /// specific provider implementation.
    pub fn new(agent: Agent<M>, max_depth: usize) -> Self {
        Self { agent, max_depth }
    }

    /// Create a new RigAgent with default max depth (10)
    pub fn with_default_depth(agent: Agent<M>) -> Self {
        Self::new(agent, DEFAULT_MAX_DEPTH)
    }

    /// Get the maximum depth setting
    pub fn max_depth(&self) -> usize {
        self.max_depth
    }

    /// Execute a prompt in non-streaming mode with automatic multi-turn tool execution
    ///
    /// Returns the final response as a string after all tool calls complete.
    /// Tools are executed automatically up to max_depth.
    pub async fn prompt(&self, prompt: &str) -> Result<String> {
        debug!(prompt = %prompt, "Starting agent execution");

        let response = self
            .agent
            .prompt(prompt)
            .multi_turn(self.max_depth)
            .await
            .map_err(anyhow::Error::from)?;

        debug!(
            response_length = response.len(),
            "Agent execution completed"
        );

        Ok(response)
    }

    /// Execute a prompt in streaming mode with automatic multi-turn tool execution
    ///
    /// Returns a stream of MultiTurnStreamItem containing:
    /// - StreamAssistantItem for text chunks and tool calls from the model
    /// - StreamUserItem for tool results
    /// - FinalResponse with the final text and aggregated usage
    ///
    /// The stream items and errors come from rig's multi-turn streaming implementation.
    ///
    /// NOTE: This version does NOT accept conversation history. Use `prompt_streaming_with_history`
    /// for persistent context across REPL iterations (CLI-008).
    pub async fn prompt_streaming(
        &self,
        prompt: &str,
    ) -> impl futures::Stream<
        Item = Result<rig::agent::MultiTurnStreamItem<M::StreamingResponse>, anyhow::Error>,
    > + '_ {
        use futures::StreamExt;

        debug!(prompt = %prompt, "Starting streaming agent execution");

        self.agent
            .stream_prompt(prompt)
            .multi_turn(self.max_depth)
            .await
            .map(|result| result.map_err(anyhow::Error::from))
    }

    /// Execute a prompt in streaming mode WITH conversation history (CLI-008)
    ///
    /// This is the persistent context version that passes message history to rig
    /// using `.with_history()` for multi-turn conversations across REPL iterations.
    ///
    /// NOTE: Rig's .with_history() takes ownership of the Vec, so we clone it.
    /// The caller is responsible for manually updating the history vector by
    /// adding messages from the stream (user prompt, assistant responses, tool calls, tool results).
    pub async fn prompt_streaming_with_history(
        &self,
        prompt: &str,
        history: &mut [rig::message::Message],
    ) -> impl futures::Stream<
        Item = Result<rig::agent::MultiTurnStreamItem<M::StreamingResponse>, anyhow::Error>,
    > + '_ {
        use futures::StreamExt;

        debug!(
            prompt = %prompt,
            history_len = history.len(),
            "Starting streaming agent execution with history"
        );

        // Clone history for rig (rig takes ownership and manages it internally)
        let history_for_rig = history.to_vec();

        self.agent
            .stream_prompt(prompt)
            .with_history(history_for_rig)
            .multi_turn(self.max_depth)
            .await
            .map(|result| result.map_err(anyhow::Error::from))
    }

    /// Execute a prompt in streaming mode WITH conversation history AND a hook
    ///
    /// This version accepts a StreamingPromptHook that is called:
    /// - `on_completion_call`: BEFORE each API call (for compaction checks)
    /// - `on_stream_completion_response_finish`: AFTER each API call (to capture per-request usage)
    ///
    /// This matches TypeScript's approach where compaction is checked before each
    /// API call using actual token values from the previous call.
    pub async fn prompt_streaming_with_history_and_hook<P>(
        &self,
        prompt: &str,
        history: &mut [rig::message::Message],
        hook: P,
    ) -> impl futures::Stream<
        Item = Result<rig::agent::MultiTurnStreamItem<M::StreamingResponse>, anyhow::Error>,
    > + '_
    where
        P: StreamingPromptHook<M> + 'static,
        M::StreamingResponse: WasmCompatSend + GetTokenUsage,
    {
        use futures::StreamExt;

        debug!(
            prompt = %prompt,
            history_len = history.len(),
            "Starting streaming agent execution with history and hook"
        );

        // Clone history for rig (rig takes ownership and manages it internally)
        let history_for_rig = history.to_vec();

        self.agent
            .stream_prompt(prompt)
            .with_history(history_for_rig)
            .with_hook(hook)
            .multi_turn(self.max_depth)
            .await
            .map(|result| result.map_err(anyhow::Error::from))
    }
}

impl<M> std::fmt::Debug for RigAgent<M>
where
    M: CompletionModel + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RigAgent")
            .field("max_depth", &self.max_depth)
            .field("tools", &9) // We have 9 tools including WebSearchTool (WEB-001)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_max_depth() {
        assert_eq!(
            DEFAULT_MAX_DEPTH,
            usize::MAX - 1,
            "Default max depth should be usize::MAX - 1"
        );
    }

    #[test]
    fn no_anyhow_macro_in_error_conversion_sites() {
        // @step Given RigAgent streaming error conversion sites in rig_agent.rs use anyhow::Error::from(e)
        // @step When the streaming error conversion is verified against the source
        // @step Then all sites use anyhow::Error::from(e) instead of anyhow::anyhow!("Streaming error: {e}")
        // @step And the typed error chain is preserved for downstream downcast_ref extraction
        //
        // This is a compile-time guarantee: the file was edited so all four
        // map_err/map sites call `anyhow::Error::from(e)`. If someone reverts
        // to `anyhow!("...")`, the `anyhow` macro import will be re-added or
        // the string pattern will re-appear. We enforce this by a negative
        // source scan.
        let source = include_str!("rig_agent.rs");
        assert!(
            !source.contains("map_err(|e| anyhow!(\"Prompt failed:"),
            "prompt() must NOT use anyhow!() — it destroys the typed error chain"
        );
        // Count occurrences outside the test module: there should be zero
        // production usages of anyhow::anyhow!("Streaming error:
        let test_module_start = source.rfind("#[cfg(test)]").unwrap_or(source.len());
        let production_source = &source[..test_module_start];
        assert!(
            !production_source.contains("map_err(|e| anyhow::anyhow!(\"Streaming error:"),
            "streaming methods must NOT use anyhow::anyhow!() in production code — it destroys the typed error chain"
        );
        assert!(
            production_source.contains("anyhow::Error::from"),
            "must use anyhow::Error::from to preserve typed error chain"
        );
    }
}
