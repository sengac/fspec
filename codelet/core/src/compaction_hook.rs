//! Compaction Hook for Rig Streaming
//!
//! Implements rig's StreamingPromptHook to capture per-request token usage
//! and check compaction thresholds before each API call.
//!
//! This matches TypeScript's approach in runner.ts:829-836 where compaction
//! is checked BEFORE each LLM call using actual API-reported token values.

use rig::agent::{CancelSignal, StreamingPromptHook};
use rig::completion::{CompletionModel, GetTokenUsage};
use rig::message::Message;
use std::sync::{Arc, Mutex};

/// Shared state for tracking token usage across API calls
#[derive(Debug, Clone)]
pub struct TokenState {
    /// Last per-request input tokens from API
    pub input_tokens: u64,
    /// Last per-request cache read tokens from API
    pub cache_read_input_tokens: u64,
    /// Last per-request cache creation tokens from API
    pub cache_creation_input_tokens: u64,
    /// Whether compaction was triggered (caller should check this)
    pub compaction_needed: bool,
}

impl Default for TokenState {
    fn default() -> Self {
        Self {
            input_tokens: 0,
            cache_read_input_tokens: 0,
            cache_creation_input_tokens: 0,
            compaction_needed: false,
        }
    }
}

/// Hook for capturing per-request usage and checking compaction thresholds
///
/// This hook is called by rig's multi-turn streaming agent:
/// - `on_completion_call`: Called BEFORE each API call - checks if compaction needed
/// - `on_stream_completion_response_finish`: Called AFTER each API call - captures usage
#[derive(Clone)]
pub struct CompactionHook {
    /// Shared state for per-request usage (shared with caller)
    pub state: Arc<Mutex<TokenState>>,
    /// Compaction threshold in tokens
    threshold: u64,
}

impl CompactionHook {
    /// Create a new CompactionHook
    ///
    /// # Arguments
    /// * `state` - Shared state for token tracking (caller retains access)
    /// * `threshold` - Token threshold that triggers compaction (e.g., context_window * 0.9)
    pub fn new(state: Arc<Mutex<TokenState>>, threshold: u64) -> Self {
        Self { state, threshold }
    }

    /// Calculate effective tokens with cache discount (matches TypeScript AGENT-028)
    ///
    /// Formula: effectiveTokens = inputTokens - (cacheReadInputTokens * 0.9)
    fn calculate_effective_tokens(&self, input_tokens: u64, cache_read_tokens: u64) -> u64 {
        let cache_discount = (cache_read_tokens as f64 * 0.9) as u64;
        input_tokens.saturating_sub(cache_discount)
    }
}

impl<M> StreamingPromptHook<M> for CompactionHook
where
    M: CompletionModel,
    <M as CompletionModel>::StreamingResponse: GetTokenUsage,
{
    /// Called BEFORE each API call - check if compaction is needed
    ///
    /// If the last API call's effective tokens exceed the threshold,
    /// we cancel to prevent the next API call from failing with "prompt too long".
    fn on_completion_call(
        &self,
        _prompt: &Message,
        _history: &[Message],
        cancel_sig: CancelSignal,
    ) -> impl std::future::Future<Output = ()> + Send {
        async move {
            let mut state = self.state.lock().unwrap();

            // Calculate effective tokens with cache discount
            let effective = self.calculate_effective_tokens(
                state.input_tokens,
                state.cache_read_input_tokens,
            );

            // Check threshold
            if effective > self.threshold {
                // Signal that compaction is needed
                state.compaction_needed = true;
                cancel_sig.cancel();
            }
        }
    }

    /// Called AFTER each API call - capture per-request usage
    ///
    /// This stores the actual token usage from the API response,
    /// which will be checked by the next on_completion_call.
    fn on_stream_completion_response_finish(
        &self,
        _prompt: &Message,
        response: &<M as CompletionModel>::StreamingResponse,
        _cancel_sig: CancelSignal,
    ) -> impl std::future::Future<Output = ()> + Send {
        async move {
            if let Some(usage) = response.token_usage() {
                let mut state = self.state.lock().unwrap();
                state.input_tokens = usage.input_tokens;
                // Cache tokens are stored in the Usage struct if available
                state.cache_read_input_tokens = usage.cache_read_input_tokens.unwrap_or(0);
                state.cache_creation_input_tokens = usage.cache_creation_input_tokens.unwrap_or(0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effective_tokens_calculation() {
        let state = Arc::new(Mutex::new(TokenState::default()));
        let hook = CompactionHook::new(state, 180_000);

        // No cache: effective = input
        assert_eq!(hook.calculate_effective_tokens(100_000, 0), 100_000);

        // With cache: effective = input - (cache * 0.9)
        // 100k - (80k * 0.9) = 100k - 72k = 28k
        assert_eq!(hook.calculate_effective_tokens(100_000, 80_000), 28_000);

        // Edge case: cache discount larger than input (saturating sub)
        assert_eq!(hook.calculate_effective_tokens(10_000, 20_000), 0);
    }

    #[test]
    fn test_token_state_default() {
        let state = TokenState::default();
        assert_eq!(state.input_tokens, 0);
        assert_eq!(state.cache_read_input_tokens, 0);
        assert!(!state.compaction_needed);
    }
}
