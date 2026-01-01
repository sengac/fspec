//! Compaction Hook for Rig Streaming
//!
//! Implements rig's StreamingPromptHook to capture per-request token usage
//! and check compaction thresholds before each API call.
//!
//! This matches TypeScript's approach in runner.ts:829-836 where compaction
//! is checked BEFORE each LLM call using actual API-reported token values.

use crate::token_usage::ApiTokenUsage;
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
    /// Last per-request output tokens from API (CTX-002)
    pub output_tokens: u64,
    /// Whether compaction was triggered (caller should check this)
    pub compaction_needed: bool,
}

impl TokenState {
    /// Create TokenState from ApiTokenUsage (PROV-001)
    ///
    /// Use this to ensure consistent token tracking across the codebase.
    pub fn from_usage(usage: ApiTokenUsage) -> Self {
        Self {
            input_tokens: usage.input_tokens,
            cache_read_input_tokens: usage.cache_read_input_tokens,
            cache_creation_input_tokens: usage.cache_creation_input_tokens,
            output_tokens: usage.output_tokens,
            compaction_needed: false,
        }
    }

    /// Convert to ApiTokenUsage for calculations
    pub fn as_usage(&self) -> ApiTokenUsage {
        ApiTokenUsage::new(
            self.input_tokens,
            self.cache_read_input_tokens,
            self.cache_creation_input_tokens,
            self.output_tokens,
        )
    }

    /// Calculate total token count (CTX-002: simple sum, no discounting)
    ///
    /// PROV-001: With Anthropic caching, total input = input + cache_read + cache_creation
    /// These are three DISJOINT sets:
    /// - input_tokens: fresh tokens not from cache and not being cached
    /// - cache_read_input_tokens: tokens read from existing cache
    /// - cache_creation_input_tokens: tokens being written to new cache
    ///
    /// Algorithm: count = input + cache_read + cache_creation + output
    #[inline]
    pub fn total(&self) -> u64 {
        self.as_usage().total_context()
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
    /// * `threshold` - Token threshold that triggers compaction (usable context after output reservation)
    pub fn new(state: Arc<Mutex<TokenState>>, threshold: u64) -> Self {
        Self { state, threshold }
    }

    /// Check if compaction is needed based on current state (for testing)
    /// This is the core logic used by on_completion_call
    ///
    /// CTX-002: Uses simple token sum (input + cache_read + output) instead of cache discount
    #[cfg(test)]
    pub fn check_compaction(&self, cancel_sig: &CancelSignal) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };

        // CTX-002: Use total() for simple sum without cache discount
        let total = state.total();

        if total > self.threshold {
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
    /// If the last API call's total tokens exceed the threshold,
    /// we cancel to prevent the next API call from failing with "prompt too long".
    ///
    /// CTX-002: Uses simple token sum (input + cache_read + output) instead of cache discount
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

        // CTX-002: Use total() for simple sum without cache discount
        let total = state.total();

        // Check threshold
        if total > self.threshold {
            // Signal that compaction is needed
            state.compaction_needed = true;
            cancel_sig.cancel();
        }
    }

    /// Called AFTER each API call - capture per-request usage
    ///
    /// This stores the actual token usage from the API response,
    /// which will be checked by the next on_completion_call.
    ///
    /// CTX-002: Now also captures output_tokens for accurate total calculation
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
            // CTX-002: Capture output tokens for accurate total calculation
            state.output_tokens = usage.output_tokens;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rig::agent::CancelSignal;

    #[test]
    fn test_token_state_total() {
        // PROV-001: Test simple sum (input + cache_read + cache_creation + output)
        let state = TokenState {
            input_tokens: 100_000,
            cache_read_input_tokens: 50_000,
            cache_creation_input_tokens: 5_000,
            output_tokens: 10_000,
            compaction_needed: false,
        };
        // Simple sum: 100,000 + 50,000 + 5,000 + 10,000 = 165,000
        assert_eq!(state.total(), 165_000);
    }

    #[test]
    fn test_token_state_total_no_cache() {
        // CTX-002: No cache tokens
        let state = TokenState {
            input_tokens: 100_000,
            cache_read_input_tokens: 0,
            cache_creation_input_tokens: 0,
            output_tokens: 5_000,
            compaction_needed: false,
        };
        assert_eq!(state.total(), 105_000);
    }

    #[test]
    fn test_token_state_default() {
        let state = TokenState::default();
        assert_eq!(state.input_tokens, 0);
        assert_eq!(state.cache_read_input_tokens, 0);
        assert_eq!(state.output_tokens, 0);
        assert!(!state.compaction_needed);
        assert_eq!(state.total(), 0);
    }

    /// Test Scenario 1: Token count under threshold - no compaction
    /// CTX-002: Uses simple sum (input + cache_read + output)
    #[test]
    fn test_under_threshold_no_compaction() {
        let state = Arc::new(Mutex::new(TokenState {
            input_tokens: 100_000,
            cache_read_input_tokens: 20_000,
            cache_creation_input_tokens: 0,
            output_tokens: 10_000,
            // Total: 100k + 20k + 10k = 130k < 180k threshold
            compaction_needed: false,
        }));
        let hook = CompactionHook::new(Arc::clone(&state), 180_000);

        let cancel_sig = CancelSignal::new();

        // Call hook check logic
        hook.check_compaction(&cancel_sig);

        // Should NOT cancel (130k < 180k)
        assert!(!cancel_sig.is_cancelled());
        assert!(!state.lock().unwrap().compaction_needed);
    }

    /// Test Scenario 2: Token count over threshold - triggers compaction
    /// CTX-002: Uses simple sum (input + cache_read + output)
    #[test]
    fn test_over_threshold_triggers_compaction() {
        let state = Arc::new(Mutex::new(TokenState {
            input_tokens: 150_000,
            cache_read_input_tokens: 20_000,
            cache_creation_input_tokens: 0,
            output_tokens: 15_000,
            // Total: 150k + 20k + 15k = 185k > 180k threshold
            compaction_needed: false,
        }));
        let hook = CompactionHook::new(Arc::clone(&state), 180_000);

        let cancel_sig = CancelSignal::new();

        hook.check_compaction(&cancel_sig);

        // Should cancel and set compaction_needed (185k > 180k)
        assert!(cancel_sig.is_cancelled());
        assert!(state.lock().unwrap().compaction_needed);
    }

    /// Test Scenario 3: CTX-002 - Cache tokens are NOT discounted
    /// Unlike old logic, cache_read tokens count at full value
    #[test]
    fn test_cache_tokens_not_discounted() {
        // CTX-002: Simple sum means cache tokens add to total
        // 100k input + 50k cache_read + 35k output = 185k total
        // 185k > 180k threshold = TRIGGERS compaction
        // (Old logic would discount cache: 100k - 45k = 55k < 180k = no compaction)
        let state = Arc::new(Mutex::new(TokenState {
            input_tokens: 100_000,
            cache_read_input_tokens: 50_000,
            cache_creation_input_tokens: 0,
            output_tokens: 35_000,
            compaction_needed: false,
        }));
        let hook = CompactionHook::new(Arc::clone(&state), 180_000);

        let cancel_sig = CancelSignal::new();

        hook.check_compaction(&cancel_sig);

        // CTX-002: Cache adds to total, so 185k > 180k triggers compaction
        assert!(cancel_sig.is_cancelled());
        assert!(state.lock().unwrap().compaction_needed);
    }

    /// Test Scenario 4: Multi-turn simulation - token state updates between calls
    /// CTX-002: Uses simple sum (input + cache_read + output)
    #[test]
    fn test_multi_turn_token_state_update() {
        let state = Arc::new(Mutex::new(TokenState {
            input_tokens: 100_000, // Initial: under threshold
            cache_read_input_tokens: 20_000,
            cache_creation_input_tokens: 0,
            output_tokens: 10_000,
            // Total: 130k < 180k threshold
            compaction_needed: false,
        }));
        let hook = CompactionHook::new(Arc::clone(&state), 180_000);

        // First API call - under threshold (130k < 180k)
        let cancel_sig1 = CancelSignal::new();
        hook.check_compaction(&cancel_sig1);
        assert!(
            !cancel_sig1.is_cancelled(),
            "First call should not trigger compaction"
        );

        // Simulate API response updating token state (this is what on_stream_completion_response_finish does)
        // Tool results added significant tokens
        {
            let mut s = state.lock().unwrap();
            s.input_tokens = 150_000;
            s.cache_read_input_tokens = 30_000;
            s.output_tokens = 10_000;
            // Total: 150k + 30k + 10k = 190k > 180k
        }

        // Second API call - should trigger compaction (190k > 180k)
        let cancel_sig2 = CancelSignal::new();
        hook.check_compaction(&cancel_sig2);
        assert!(
            cancel_sig2.is_cancelled(),
            "Second call should trigger compaction"
        );
        assert!(
            state.lock().unwrap().compaction_needed,
            "compaction_needed should be set"
        );
    }

    /// Test Scenario 5: Simulates real-world context growth
    /// CTX-002: Uses simple sum (input + cache_read + output)
    #[test]
    fn test_context_growth_scenario() {
        // Initial state: under threshold
        let state = Arc::new(Mutex::new(TokenState {
            input_tokens: 100_000,
            cache_read_input_tokens: 15_000,
            cache_creation_input_tokens: 0,
            output_tokens: 10_000,
            // Total: 100k + 15k + 10k = 125k < 180k
            compaction_needed: false,
        }));
        let hook = CompactionHook::new(Arc::clone(&state), 180_000);

        // First API call check - should pass (125k < 180k)
        let cancel_sig1 = CancelSignal::new();
        hook.check_compaction(&cancel_sig1);
        assert!(
            !cancel_sig1.is_cancelled(),
            "First call should NOT trigger compaction (125k < 180k)"
        );

        // Simulate context growth after tool calls
        {
            let mut s = state.lock().unwrap();
            s.input_tokens = 150_000;
            s.cache_read_input_tokens = 25_000;
            s.output_tokens = 15_000;
            // Total: 150k + 25k + 15k = 190k > 180k
        }

        // Second API call check - should trigger compaction (190k > 180k)
        let cancel_sig2 = CancelSignal::new();
        hook.check_compaction(&cancel_sig2);
        assert!(
            cancel_sig2.is_cancelled(),
            "Second call MUST trigger compaction (190k > 180k)"
        );
        assert!(
            state.lock().unwrap().compaction_needed,
            "compaction_needed must be set for stream_loop to run compaction"
        );
    }

    /// Test Scenario 6: Exactly at threshold - should NOT trigger (need to exceed)
    /// CTX-002: Uses simple sum, strictly greater than (>)
    #[test]
    fn test_exactly_at_threshold_no_trigger() {
        let state = Arc::new(Mutex::new(TokenState {
            input_tokens: 150_000,
            cache_read_input_tokens: 20_000,
            cache_creation_input_tokens: 0,
            output_tokens: 10_000,
            // Total: 150k + 20k + 10k = 180k (exactly at threshold)
            compaction_needed: false,
        }));
        let hook = CompactionHook::new(Arc::clone(&state), 180_000);

        let cancel_sig = CancelSignal::new();
        hook.check_compaction(&cancel_sig);

        // Should NOT cancel (need to EXCEED threshold, not equal)
        assert!(!cancel_sig.is_cancelled());
    }

    /// Test Scenario 7: One token over threshold - should trigger
    /// CTX-002: Uses simple sum, strictly greater than (>)
    #[test]
    fn test_one_over_threshold_triggers() {
        let state = Arc::new(Mutex::new(TokenState {
            input_tokens: 150_001, // One extra token
            cache_read_input_tokens: 20_000,
            cache_creation_input_tokens: 0,
            output_tokens: 10_000,
            // Total: 150,001 + 20k + 10k = 180,001 > 180k
            compaction_needed: false,
        }));
        let hook = CompactionHook::new(Arc::clone(&state), 180_000);

        let cancel_sig = CancelSignal::new();
        hook.check_compaction(&cancel_sig);

        // Should cancel (180,001 > 180,000)
        assert!(cancel_sig.is_cancelled());
    }
}
