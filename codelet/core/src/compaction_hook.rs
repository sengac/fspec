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
#[derive(Debug, Clone, Default)]
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

    /// Check if compaction is needed based on current state (for testing)
    /// This is the core logic used by on_completion_call
    #[cfg(test)]
    pub fn check_compaction(&self, cancel_sig: &CancelSignal) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };

        let effective =
            self.calculate_effective_tokens(state.input_tokens, state.cache_read_input_tokens);

        if effective > self.threshold {
            state.compaction_needed = true;
            cancel_sig.cancel();
        }
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
    async fn on_completion_call(
        &self,
        _prompt: &Message,
        _history: &[Message],
        cancel_sig: CancelSignal,
    ) {
        // Handle mutex lock gracefully - if poisoned, skip the check
        // A poisoned mutex indicates another thread panicked while holding it
        let Ok(mut state) = self.state.lock() else {
            tracing::warn!("CompactionHook state mutex poisoned, skipping compaction check");
            return;
        };

        // Calculate effective tokens with cache discount
        let effective =
            self.calculate_effective_tokens(state.input_tokens, state.cache_read_input_tokens);

        // Check threshold
        if effective > self.threshold {
            // Signal that compaction is needed
            state.compaction_needed = true;
            cancel_sig.cancel();
        }
    }

    /// Called AFTER each API call - capture per-request usage
    ///
    /// This stores the actual token usage from the API response,
    /// which will be checked by the next on_completion_call.
    async fn on_stream_completion_response_finish(
        &self,
        _prompt: &Message,
        response: &<M as CompletionModel>::StreamingResponse,
        _cancel_sig: CancelSignal,
    ) {
        if let Some(usage) = response.token_usage() {
            // Handle mutex lock gracefully - if poisoned, skip the update
            let Ok(mut state) = self.state.lock() else {
                tracing::warn!("CompactionHook state mutex poisoned, skipping token update");
                return;
            };
            state.input_tokens = usage.input_tokens;
            // Cache tokens are stored in the Usage struct if available
            state.cache_read_input_tokens = usage.cache_read_input_tokens.unwrap_or(0);
            state.cache_creation_input_tokens = usage.cache_creation_input_tokens.unwrap_or(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rig::agent::CancelSignal;

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

    /// Test Scenario 1: Token count under threshold - no compaction
    #[test]
    fn test_under_threshold_no_compaction() {
        let state = Arc::new(Mutex::new(TokenState {
            input_tokens: 100_000, // Under 180k threshold
            cache_read_input_tokens: 0,
            cache_creation_input_tokens: 0,
            compaction_needed: false,
        }));
        let hook = CompactionHook::new(Arc::clone(&state), 180_000);

        let cancel_sig = CancelSignal::new();

        // Call hook check logic
        hook.check_compaction(&cancel_sig);

        // Should NOT cancel
        assert!(!cancel_sig.is_cancelled());
        assert!(!state.lock().unwrap().compaction_needed);
    }

    /// Test Scenario 2: Token count over threshold - triggers compaction
    #[test]
    fn test_over_threshold_triggers_compaction() {
        let state = Arc::new(Mutex::new(TokenState {
            input_tokens: 200_000, // Over 180k threshold
            cache_read_input_tokens: 0,
            cache_creation_input_tokens: 0,
            compaction_needed: false,
        }));
        let hook = CompactionHook::new(Arc::clone(&state), 180_000);

        let cancel_sig = CancelSignal::new();

        hook.check_compaction(&cancel_sig);

        // Should cancel and set compaction_needed
        assert!(cancel_sig.is_cancelled());
        assert!(state.lock().unwrap().compaction_needed);
    }

    /// Test Scenario 3: Cache discount brings effective tokens under threshold
    #[test]
    fn test_cache_discount_prevents_compaction() {
        // 200k input, 100k cache read
        // effective = 200k - (100k * 0.9) = 200k - 90k = 110k
        // 110k < 180k threshold = no compaction
        let state = Arc::new(Mutex::new(TokenState {
            input_tokens: 200_000,
            cache_read_input_tokens: 100_000,
            cache_creation_input_tokens: 0,
            compaction_needed: false,
        }));
        let hook = CompactionHook::new(Arc::clone(&state), 180_000);

        let cancel_sig = CancelSignal::new();

        hook.check_compaction(&cancel_sig);

        // Should NOT cancel due to cache discount
        assert!(!cancel_sig.is_cancelled());
        assert!(!state.lock().unwrap().compaction_needed);
    }

    /// Test Scenario 4: Multi-turn simulation - token state updates between calls
    /// This tests the fix for tool-only responses where on_stream_completion_response_finish
    /// was not being called
    #[test]
    fn test_multi_turn_token_state_update() {
        let state = Arc::new(Mutex::new(TokenState {
            input_tokens: 100_000, // Initial: under threshold
            cache_read_input_tokens: 0,
            cache_creation_input_tokens: 0,
            compaction_needed: false,
        }));
        let hook = CompactionHook::new(Arc::clone(&state), 180_000);

        // First API call - under threshold
        let cancel_sig1 = CancelSignal::new();
        hook.check_compaction(&cancel_sig1);
        assert!(!cancel_sig1.is_cancelled(), "First call should not trigger compaction");

        // Simulate API response updating token state (this is what on_stream_completion_response_finish does)
        // Tool results added significant tokens
        {
            let mut s = state.lock().unwrap();
            s.input_tokens = 250_000; // Now over threshold
        }

        // Second API call - should trigger compaction
        let cancel_sig2 = CancelSignal::new();
        hook.check_compaction(&cancel_sig2);
        assert!(cancel_sig2.is_cancelled(), "Second call should trigger compaction");
        assert!(
            state.lock().unwrap().compaction_needed,
            "compaction_needed should be set"
        );
    }

    /// Test Scenario 5: Simulates the exact bug from debug log
    /// - Initial: 161k tokens (under 180k threshold)
    /// - After tool calls: 484k tokens (way over threshold)
    /// - Without the rig fix, on_stream_completion_response_finish was not called for tool-only responses
    /// - With the fix, token state should be updated and second API call should trigger compaction
    #[test]
    fn test_tool_call_context_growth_scenario() {
        // Matches debug log: initial tokens = 161,826
        let state = Arc::new(Mutex::new(TokenState {
            input_tokens: 161_826,
            cache_read_input_tokens: 14_964,
            cache_creation_input_tokens: 0,
            compaction_needed: false,
        }));
        let hook = CompactionHook::new(Arc::clone(&state), 180_000);

        // First API call check - should pass (effective = 161,826 - 13,467 = 148,359 < 180k)
        let cancel_sig1 = CancelSignal::new();
        hook.check_compaction(&cancel_sig1);
        assert!(
            !cancel_sig1.is_cancelled(),
            "First call should NOT trigger compaction (148k effective < 180k)"
        );

        // Simulate what happens after tool calls complete (the rig fix ensures this is called)
        // API response shows 484k input tokens
        {
            let mut s = state.lock().unwrap();
            s.input_tokens = 484_732;
            s.cache_read_input_tokens = 27_434;
        }

        // Second API call check - should trigger compaction
        // effective = 484,732 - (27,434 * 0.9) = 484,732 - 24,690 = 460,042 > 180k
        let cancel_sig2 = CancelSignal::new();
        hook.check_compaction(&cancel_sig2);
        assert!(
            cancel_sig2.is_cancelled(),
            "Second call MUST trigger compaction (460k effective > 180k)"
        );
        assert!(
            state.lock().unwrap().compaction_needed,
            "compaction_needed must be set for stream_loop to run compaction"
        );
    }

    /// Test Scenario 6: Exactly at threshold - should NOT trigger (need to exceed)
    #[test]
    fn test_exactly_at_threshold_no_trigger() {
        let state = Arc::new(Mutex::new(TokenState {
            input_tokens: 180_000, // Exactly at threshold
            cache_read_input_tokens: 0,
            cache_creation_input_tokens: 0,
            compaction_needed: false,
        }));
        let hook = CompactionHook::new(Arc::clone(&state), 180_000);

        let cancel_sig = CancelSignal::new();
        hook.check_compaction(&cancel_sig);

        // Should NOT cancel (need to EXCEED threshold)
        assert!(!cancel_sig.is_cancelled());
    }

    /// Test Scenario 7: One token over threshold - should trigger
    #[test]
    fn test_one_over_threshold_triggers() {
        let state = Arc::new(Mutex::new(TokenState {
            input_tokens: 180_001, // One over threshold
            cache_read_input_tokens: 0,
            cache_creation_input_tokens: 0,
            compaction_needed: false,
        }));
        let hook = CompactionHook::new(Arc::clone(&state), 180_000);

        let cancel_sig = CancelSignal::new();
        hook.check_compaction(&cancel_sig);

        // Should cancel
        assert!(cancel_sig.is_cancelled());
    }
}
