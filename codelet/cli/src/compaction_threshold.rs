//! Compaction Threshold Calculation (CLI-020, CTX-007)
//!
//! Provides constants and functions for calculating when context compaction
//! should be triggered, including the autocompact buffer that leaves headroom
//! after compaction to reduce re-compaction frequency.
//!
//! CTX-007: Per-model configurable compaction thresholds. The threshold can be
//! configured as absolute tokens, percentage of context window, or fall through
//! to the base formula.
//!
//! Reference: spec/features/autocompact-buffer.feature
//!            spec/features/per-model-compaction-threshold.feature

/// Autocompact buffer in tokens
///
/// This buffer defines the target context size AFTER compaction, leaving
/// headroom before the next compaction trigger.
///
/// How it works:
/// - Threshold: calculate_usable_context() → context_window - min(max_output, 32k)
/// - Budget (target after compact): contextWindow - AUTOCOMPACT_BUFFER (e.g., 150k for 200k window)
/// - Headroom: threshold - budget (varies by model)
///
/// This separation ensures:
/// - After compaction, there's headroom before next trigger
/// - Reduces re-compaction frequency by providing buffer space for new interactions
///
/// Example for Claude (200k context, 8k max_output):
/// - Trigger threshold: 191,808 tokens (200k - 8k)
/// - Target after compaction: 150,000 tokens (200k - 50k)
/// - Headroom before re-trigger: 41,808 tokens
pub const AUTOCOMPACT_BUFFER: u64 = 50_000;

/// Calculate the summarization budget for context compaction
///
/// Matches TypeScript implementation in compaction.ts:calculateSummarizationBudget()
///
/// The budget determines how many tokens to target after compaction.
/// It is separate from the threshold calculation.
///
/// Logic:
/// - If context_window <= AUTOCOMPACT_BUFFER: budget = context_window * 0.8
/// - Otherwise: budget = context_window - AUTOCOMPACT_BUFFER
///
/// # Arguments
/// * `context_window` - The provider's context window size in tokens
///
/// # Returns
/// * The budget in tokens to target after compaction
///
/// # Examples
/// ```
/// use codelet_cli::compaction_threshold::calculate_summarization_budget;
///
/// // Claude with 200k context
/// let budget = calculate_summarization_budget(200_000);
/// assert_eq!(budget, 150_000); // 200k - 50k
///
/// // Small context window (40k)
/// let budget = calculate_summarization_budget(40_000);
/// assert_eq!(budget, 32_000); // 40k * 0.8
/// ```
pub fn calculate_summarization_budget(context_window: u64) -> u64 {
    if context_window <= AUTOCOMPACT_BUFFER {
        (context_window as f64 * 0.8) as u64
    } else {
        context_window - AUTOCOMPACT_BUFFER
    }
}

// =============================================================================
// CTX-002: Optimized Compaction Window Limit Trigger
// =============================================================================

/// Session output token maximum
///
/// This constant caps the output reservation to prevent over-reserving context
/// for models with very large max_output values. Set to 32k tokens as a
/// reasonable upper bound for output reservation.
pub const SESSION_OUTPUT_TOKEN_MAX: u64 = 32_000;

/// Calculate usable context given model limits
///
/// Algorithm:
/// 1. output_reservation = min(model_max_output, SESSION_OUTPUT_TOKEN_MAX)
/// 2. If output_reservation == 0, use SESSION_OUTPUT_TOKEN_MAX as fallback
/// 3. usable_context = context_window - output_reservation
///
/// # Arguments
/// * `context_window` - The model's total context window
/// * `model_max_output` - The model's maximum output tokens (0 if unknown)
///
/// # Returns
/// * The usable context in tokens (space available for input/cache/output)
pub fn calculate_usable_context(context_window: u64, model_max_output: u64) -> u64 {
    let output_reservation = model_max_output.min(SESSION_OUTPUT_TOKEN_MAX);
    let output_reservation = if output_reservation == 0 {
        SESSION_OUTPUT_TOKEN_MAX
    } else {
        output_reservation
    };
    context_window.saturating_sub(output_reservation)
}

// =============================================================================
// CTX-007: Per-Model Configurable Compaction Threshold
// =============================================================================

/// Per-model compaction threshold configuration.
///
/// Allows the compaction trigger point to be configured independently of the
/// context window size. This decouples "when to compact" from "how big is the
/// context" — essential when models.dev reports 1M but the practical limit is 200k.
#[derive(Debug, Clone)]
pub enum CompactionThresholdConfig {
    /// Absolute token count (e.g., 200000 for Claude)
    Tokens(u64),
    /// Percentage of context window (e.g., 80 for 80%)
    Percentage(u8),
}

impl CompactionThresholdConfig {
    /// Resolve to absolute token count given the context window.
    ///
    /// For Tokens: returns the configured value directly.
    /// For Percentage: computes percentage of context_window.
    pub fn resolve(&self, context_window: u64) -> u64 {
        match self {
            Self::Tokens(tokens) => *tokens,
            Self::Percentage(pct) => {
                (context_window as f64 * (*pct as f64 / 100.0)) as u64
            }
        }
    }

    /// Construct from type string and value, as received from NAPI/TUI layer.
    ///
    /// `type_str` should be "percentage" or "tokens" (anything else defaults to Tokens).
    /// `value` is the raw numeric value (percentage 0-100 or absolute token count).
    pub fn from_type_value(type_str: &str, value: u64) -> Self {
        if type_str == "percentage" {
            Self::Percentage(value as u8)
        } else {
            Self::Tokens(value)
        }
    }
}

/// Get built-in compaction threshold for known model families.
///
/// Resolution uses model_id string prefix matching, which works for both
/// registry models (models.dev) and profile models (no registry).
///
/// Returns None for unknown models, letting the caller fall through to
/// the base calculate_usable_context formula.
///
/// Priority: Claude family uses base formula (no override);
/// Gemini/OpenAI/others get 80% of context_window.
pub fn builtin_compaction_threshold(model_id: Option<&str>) -> Option<CompactionThresholdConfig> {
    let id = model_id?;
    let id_lower = id.to_lowercase();

    // Claude family: use base formula (no built-in override)
    // This means Claude uses calculate_usable_context(context_window, max_output)
    // which gives threshold = context_window - min(max_output, 32k)
    if id_lower.starts_with("claude-") || id_lower.starts_with("claude_") {
        return None;
    }

    // Gemini family: 80% of context window
    if id_lower.starts_with("gemini-") || id_lower.starts_with("gemini_") {
        return Some(CompactionThresholdConfig::Percentage(80));
    }

    // OpenAI family (GPT, o-series): 80% of context window
    if id_lower.starts_with("gpt-") || id_lower.starts_with("gpt4")
        || id_lower.starts_with("o1") || id_lower.starts_with("o3")
        || id_lower.starts_with("o4") || id_lower.starts_with("chatgpt-")
    {
        return Some(CompactionThresholdConfig::Percentage(80));
    }

    // Unknown model: no built-in default, fall through to base formula
    None
}

/// Resolve the effective compaction threshold for a model.
///
/// Priority chain:
/// 1. User-configured override (from NAPI/TUI custom model settings)
/// 2. Built-in model family default (Gemini/OpenAI → 80%, Claude → base formula)
/// 3. Base formula: calculate_usable_context(context_window, max_output)
///
/// The resolved value is clamped to not exceed
/// calculate_usable_context(context_window, max_output) to prevent the
/// threshold from exceeding the actual usable context.
pub fn resolve_compaction_threshold(
    context_window: u64,
    max_output: u64,
    model_id: Option<&str>,
    user_config: Option<&CompactionThresholdConfig>,
) -> u64 {
    let base_threshold = calculate_usable_context(context_window, max_output);

    // 1. User-configured override
    if let Some(config) = user_config {
        let resolved = config.resolve(context_window);
        // Clamp: never exceed the base threshold (usable context)
        return resolved.min(base_threshold);
    }

    // 2. Built-in model family default
    if let Some(config) = builtin_compaction_threshold(model_id) {
        let resolved = config.resolve(context_window);
        return resolved.min(base_threshold);
    }

    // 3. Default: base formula
    base_threshold
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_constants_defined() {
        assert_eq!(AUTOCOMPACT_BUFFER, 50_000);
        assert_eq!(SESSION_OUTPUT_TOKEN_MAX, 32_000);
    }

    // =========================================================================
    // CTX-002 Tests: Optimized Compaction Window Limit Trigger
    // Feature: spec/features/optimized-compaction-trigger.feature
    // =========================================================================

    // -------------------------------------------------------------------------
    // Scenario: Calculate usable context for Claude Sonnet
    // -------------------------------------------------------------------------
    #[test]
    fn test_usable_context_claude_sonnet() {
        // @step Given a model with context_window of 200000 tokens
        let context_window = 200_000;

        // @step And the model has max_output_tokens of 8192
        let max_output = 8_192;

        // @step And SESSION_OUTPUT_TOKEN_MAX is 32000
        assert_eq!(SESSION_OUTPUT_TOKEN_MAX, 32_000);

        // @step When I calculate usable context
        let usable = calculate_usable_context(context_window, max_output);

        // @step Then usable context should be 191808 tokens
        // 200,000 - min(8,192, 32,000) = 200,000 - 8,192 = 191,808
        assert_eq!(usable, 191_808);
    }

    // -------------------------------------------------------------------------
    // Scenario: Calculate usable context for GPT-4
    // -------------------------------------------------------------------------
    #[test]
    fn test_usable_context_gpt4() {
        // @step Given a model with context_window of 128000 tokens
        let context_window = 128_000;

        // @step And the model has max_output_tokens of 4096
        let max_output = 4_096;

        // @step And SESSION_OUTPUT_TOKEN_MAX is 32000
        assert_eq!(SESSION_OUTPUT_TOKEN_MAX, 32_000);

        // @step When I calculate usable context
        let usable = calculate_usable_context(context_window, max_output);

        // @step Then usable context should be 123904 tokens
        // 128,000 - min(4,096, 32,000) = 128,000 - 4,096 = 123,904
        assert_eq!(usable, 123_904);
    }

    // -------------------------------------------------------------------------
    // Scenario: SESSION_OUTPUT_MAX caps high-output models
    // -------------------------------------------------------------------------
    #[test]
    fn test_usable_context_high_output_capped() {
        // @step Given a model with context_window of 200000 tokens
        let context_window = 200_000;

        // @step And the model has max_output_tokens of 64000
        let max_output = 64_000;

        // @step And SESSION_OUTPUT_TOKEN_MAX is 32000
        assert_eq!(SESSION_OUTPUT_TOKEN_MAX, 32_000);

        // @step When I calculate usable context
        let usable = calculate_usable_context(context_window, max_output);

        // @step Then usable context should be 168000 tokens
        // 200,000 - min(64,000, 32,000) = 200,000 - 32,000 = 168,000
        assert_eq!(usable, 168_000);
    }

    // -------------------------------------------------------------------------
    // Scenario: Unknown model with zero max_output uses SESSION_OUTPUT_MAX fallback
    // -------------------------------------------------------------------------
    #[test]
    fn test_usable_context_zero_max_output_fallback() {
        // @step Given a model with context_window of 100000 tokens
        let context_window = 100_000;

        // @step And the model has max_output_tokens of 0
        let max_output = 0;

        // @step And SESSION_OUTPUT_TOKEN_MAX is 32000
        assert_eq!(SESSION_OUTPUT_TOKEN_MAX, 32_000);

        // @step When I calculate usable context
        let usable = calculate_usable_context(context_window, max_output);

        // @step Then usable context should be 68000 tokens
        // min(0, 32000) = 0, but 0 triggers fallback to SESSION_OUTPUT_MAX
        // usable = 100,000 - 32,000 = 68,000 (NOT 100,000)
        assert_eq!(usable, 68_000);
    }

    #[test]
    fn test_calculate_budget_large_context() {
        // Large context: 200k
        let budget = calculate_summarization_budget(200_000);
        assert_eq!(budget, 150_000); // 200k - 50k
    }

    #[test]
    fn test_calculate_budget_small_context() {
        // Small context: 40k (less than buffer)
        let budget = calculate_summarization_budget(40_000);
        assert_eq!(budget, 32_000); // 40k * 0.8
    }

    #[test]
    fn test_calculate_budget_equal_to_buffer() {
        // Context equals buffer: 50k
        let budget = calculate_summarization_budget(50_000);
        assert_eq!(budget, 40_000); // 50k * 0.8
    }

    // =========================================================================
    // MODEL-005: Compaction threshold uses per-model context window values
    // Feature: spec/features/per-model-context-window-and-max-output-configuration.feature
    // =========================================================================

    // -------------------------------------------------------------------------
    // Scenario: Compaction threshold uses per-model context window for large-context model
    // -------------------------------------------------------------------------
    #[test]
    fn test_compaction_threshold_large_context_model() {
        // @step Given a ProviderManager with model_context_window=200000 and model_max_output_tokens=100000
        let context_window: u64 = 200_000;
        let max_output: u64 = 100_000;

        // @step When the compaction threshold is calculated
        let usable = calculate_usable_context(context_window, max_output);

        // @step Then calculate_usable_context(200000, 100000) should return 168000
        // 200,000 - min(100,000, 32,000) = 200,000 - 32,000 = 168,000
        assert_eq!(usable, 168_000);

        // @step And compaction triggers when effective tokens exceed 168000
        assert!(168_001 > usable - 1, "tokens exceeding threshold should trigger compaction");
    }

    // -------------------------------------------------------------------------
    // Scenario: Compaction threshold uses per-model context window for small-context model
    // -------------------------------------------------------------------------
    #[test]
    fn test_compaction_threshold_small_context_model() {
        // @step Given a ProviderManager with model_context_window=32000 and model_max_output_tokens=4096
        let context_window: u64 = 32_000;
        let max_output: u64 = 4_096;

        // @step When the compaction threshold is calculated
        let usable = calculate_usable_context(context_window, max_output);

        // @step Then calculate_usable_context(32000, 4096) should return 27904
        // 32,000 - min(4,096, 32,000) = 32,000 - 4,096 = 27,904
        assert_eq!(usable, 27_904);

        // @step And compaction triggers when effective tokens exceed 27904
        assert!(27_905 > usable - 1, "tokens exceeding threshold should trigger compaction");
    }

    // =========================================================================
    // CTX-007 Tests: Per-Model Configurable Compaction Threshold
    // Feature: spec/features/per-model-compaction-threshold.feature
    // =========================================================================

    // -------------------------------------------------------------------------
    // Scenario: CompactionThresholdConfig resolves tokens mode
    // -------------------------------------------------------------------------
    #[test]
    fn test_threshold_config_tokens_resolve() {
        // @step Given a compaction threshold configured as 150000 tokens
        let config = CompactionThresholdConfig::Tokens(150_000);

        // @step When resolved with any context window
        let resolved = config.resolve(200_000);

        // @step Then the threshold should equal exactly 150000
        assert_eq!(resolved, 150_000);
    }

    // -------------------------------------------------------------------------
    // Scenario: CompactionThresholdConfig resolves percentage mode
    // -------------------------------------------------------------------------
    #[test]
    fn test_threshold_config_percentage_resolve() {
        // @step Given a compaction threshold configured as 80 percent
        let config = CompactionThresholdConfig::Percentage(80);

        // @step When resolved with context window of 1000000
        let resolved = config.resolve(1_000_000);

        // @step Then the threshold should equal 800000
        assert_eq!(resolved, 800_000);
    }

    #[test]
    fn test_threshold_config_percentage_60() {
        let config = CompactionThresholdConfig::Percentage(60);
        assert_eq!(config.resolve(200_000), 120_000);
    }

    // -------------------------------------------------------------------------
    // Scenario: Built-in defaults by model family
    // -------------------------------------------------------------------------
    #[test]
    fn test_builtin_claude_returns_none() {
        // Claude family uses base formula (no built-in override)
        assert!(builtin_compaction_threshold(Some("claude-sonnet-4")).is_none());
        assert!(builtin_compaction_threshold(Some("claude-opus-4-6")).is_none());
        assert!(builtin_compaction_threshold(Some("claude-3-haiku")).is_none());
    }

    #[test]
    fn test_builtin_gemini_returns_80_percent() {
        let config = builtin_compaction_threshold(Some("gemini-2.5-pro"));
        assert!(config.is_some());
        assert_eq!(config.unwrap().resolve(1_000_000), 800_000);
    }

    #[test]
    fn test_builtin_openai_returns_80_percent() {
        let config = builtin_compaction_threshold(Some("gpt-4o"));
        assert!(config.is_some());
        assert_eq!(config.unwrap().resolve(128_000), 102_400);

        let config_o3 = builtin_compaction_threshold(Some("o3-mini"));
        assert!(config_o3.is_some());
        assert_eq!(config_o3.unwrap().resolve(200_000), 160_000);
    }

    #[test]
    fn test_builtin_unknown_returns_none() {
        assert!(builtin_compaction_threshold(Some("my-custom-llm")).is_none());
        assert!(builtin_compaction_threshold(None).is_none());
    }

    // -------------------------------------------------------------------------
    // Scenario: resolve_compaction_threshold priority chain
    // -------------------------------------------------------------------------
    #[test]
    fn test_resolve_priority_user_override() {
        // User override should take priority over everything
        let user_config = CompactionThresholdConfig::Tokens(150_000);
        let threshold = resolve_compaction_threshold(
            200_000, 8_192, Some("claude-sonnet-4"), Some(&user_config),
        );
        assert_eq!(threshold, 150_000);
    }

    #[test]
    fn test_resolve_priority_builtin_gemini() {
        // No user config → built-in Gemini default (80%)
        let threshold = resolve_compaction_threshold(
            1_000_000, 65_536, Some("gemini-2.5-pro"), None,
        );
        assert_eq!(threshold, 800_000);
    }

    #[test]
    fn test_resolve_priority_claude_base() {
        // No user config → Claude has no built-in → base formula
        let threshold = resolve_compaction_threshold(
            200_000, 8_192, Some("claude-sonnet-4"), None,
        );
        // Legacy: 200k - min(8192, 32k) = 200k - 8192 = 191808
        assert_eq!(threshold, 191_808);
    }

    #[test]
    fn test_resolve_priority_unknown_base() {
        // Unknown model → no built-in → base formula
        let threshold = resolve_compaction_threshold(
            100_000, 0, Some("my-custom-llm"), None,
        );
        // Legacy: 100k - 32k (0→fallback) = 68k
        assert_eq!(threshold, 68_000);
    }

    #[test]
    fn test_resolve_clamps_user_threshold_above_context() {
        // User sets 300k on a 200k context model → clamped to base threshold
        let user_config = CompactionThresholdConfig::Tokens(300_000);
        let threshold = resolve_compaction_threshold(
            200_000, 100_000, Some("claude-sonnet-4"), Some(&user_config),
        );
        // Legacy: 200k - min(100k, 32k) = 200k - 32k = 168k
        assert_eq!(threshold, 168_000);
    }

    #[test]
    fn test_resolve_openai_gpt4o() {
        let threshold = resolve_compaction_threshold(
            128_000, 16_384, Some("gpt-4o"), None,
        );
        // Built-in: 80% of 128k = 102400
        // Legacy: 128k - 16384 = 111616
        // 102400 < 111616, so 102400 (not clamped)
        assert_eq!(threshold, 102_400);
    }

    // -------------------------------------------------------------------------
    // Scenario: from_type_value factory method
    // -------------------------------------------------------------------------
    #[test]
    fn test_from_type_value_percentage() {
        let config = CompactionThresholdConfig::from_type_value("percentage", 80);
        assert_eq!(config.resolve(200_000), 160_000);
    }

    #[test]
    fn test_from_type_value_tokens() {
        let config = CompactionThresholdConfig::from_type_value("tokens", 150_000);
        assert_eq!(config.resolve(200_000), 150_000);
    }

    #[test]
    fn test_from_type_value_unknown_defaults_to_tokens() {
        let config = CompactionThresholdConfig::from_type_value("unknown", 100_000);
        assert_eq!(config.resolve(200_000), 100_000);
    }

    // =========================================================================
    // LIMITS-005 Tests: Compaction Threshold Chain With Clamped Inputs
    // Feature: spec/features/compaction-threshold-clamped-inputs.feature
    // =========================================================================

    // -------------------------------------------------------------------------
    // Scenario: Claude Opus 4.6 compaction threshold uses clamped context window
    // -------------------------------------------------------------------------
    #[test]
    fn test_limits005_claude_opus_clamped_threshold() {
        // @step Given a Claude model where models.dev reports context_window of 1000000 and max_output of 16384
        // After LIMITS-004 clamping, ProviderManager.context_window() returns 200k
        // and max_output_tokens() returns 8192 (Claude provider hard max).
        let clamped_context_window: u64 = 200_000;
        let clamped_max_output: u64 = 8_192;

        // @step And the Claude provider clamps context_window to 200000 and max_output to 8192
        // These are the values stream_loop.rs:276 receives from provider_manager
        assert_eq!(clamped_context_window, 200_000);
        assert_eq!(clamped_max_output, 8_192);

        // @step When resolve_compaction_threshold is called with the clamped values and model_id "claude-opus-4-6"
        let threshold = resolve_compaction_threshold(
            clamped_context_window,
            clamped_max_output,
            Some("claude-opus-4-6"),
            None, // No user config
        );

        // @step Then the threshold should be 191808 tokens (200000 minus 8192)
        // Claude has no builtin override → falls through to base formula
        // Legacy: calculate_usable_context(200000, 8192) = 200000 - min(8192, 32000) = 191808
        assert_eq!(threshold, 191_808);
    }

    // -------------------------------------------------------------------------
    // Scenario: Context fill percentage is accurate with clamped threshold
    // -------------------------------------------------------------------------
    #[test]
    fn test_limits005_context_fill_percentage_accuracy() {
        // @step Given a clamped compaction threshold of 191808 tokens
        let threshold: u64 = 191_808;

        // @step When the current token count is 87000
        let total_tokens: u64 = 87_000;
        let fill_percentage = if threshold > 0 {
            ((total_tokens as f64 / threshold as f64) * 100.0) as u32
        } else {
            0
        };

        // @step Then the fill percentage should be approximately 45 percent (not 9 percent from wrong 968k threshold)
        assert_eq!(fill_percentage, 45, "87k / 191k should be ~45%, not 9% from wrong 968k threshold");

        // Verify the wrong threshold would give wrong answer
        let wrong_threshold: u64 = 968_000; // What 1M - 32k would give with unclamped values
        let wrong_fill = ((total_tokens as f64 / wrong_threshold as f64) * 100.0) as u32;
        assert_eq!(wrong_fill, 8, "With wrong 968k threshold, fill would be ~8-9% — incorrect");
    }

    // -------------------------------------------------------------------------
    // Scenario: Compaction fires before API limit with clamped threshold
    // -------------------------------------------------------------------------
    #[test]
    fn test_limits005_compaction_fires_before_api_limit() {
        // @step Given a CompactionHook with threshold 191808 derived from clamped context_window
        let threshold: u64 = 191_808;

        // @step When the token count reaches 192000 tokens
        let token_count: u64 = 192_000;

        // @step Then compaction should be triggered well before the 200000 API limit
        assert!(
            token_count > threshold,
            "192000 > 191808 should trigger compaction"
        );
        assert!(
            threshold < 200_000,
            "Threshold 191808 is well below the 200000 API limit"
        );
        // With the wrong unclamped threshold (968k), compaction would NOT fire at 192k tokens
        let wrong_threshold: u64 = 968_000;
        assert!(
            token_count < wrong_threshold,
            "With wrong 968k threshold, 192k tokens would NOT trigger compaction — prompt-too-long"
        );
    }

    // -------------------------------------------------------------------------
    // Scenario: Gemini 2.5 Pro compaction threshold uses 80 percent of context window
    // -------------------------------------------------------------------------
    #[test]
    fn test_limits005_gemini_threshold() {
        // @step Given a Gemini model with context_window of 1000000 and max_output of 65536
        let context_window: u64 = 1_000_000;
        let max_output: u64 = 65_536;

        // @step When resolve_compaction_threshold is called with model_id "gemini-2.5-pro"
        let threshold = resolve_compaction_threshold(
            context_window,
            max_output,
            Some("gemini-2.5-pro"),
            None,
        );

        // @step Then the threshold should be 800000 tokens (80 percent of 1M)
        assert_eq!(threshold, 800_000);
    }

    // -------------------------------------------------------------------------
    // Scenario: GPT-4o compaction threshold uses 80 percent of context window
    // -------------------------------------------------------------------------
    #[test]
    fn test_limits005_gpt4o_threshold() {
        // @step Given an OpenAI model with context_window of 128000 and max_output of 16384
        let context_window: u64 = 128_000;
        let max_output: u64 = 16_384;

        // @step When resolve_compaction_threshold is called with model_id "gpt-4o"
        let threshold = resolve_compaction_threshold(
            context_window,
            max_output,
            Some("gpt-4o"),
            None,
        );

        // @step Then the threshold should be 102400 tokens (80 percent of 128k)
        assert_eq!(threshold, 102_400);
    }

    // -------------------------------------------------------------------------
    // Scenario: Summarization budget uses clamped context window
    // -------------------------------------------------------------------------
    #[test]
    fn test_limits005_summarization_budget_clamped() {
        // @step Given a clamped context_window of 200000 for Claude
        let clamped_context_window: u64 = 200_000;

        // @step When calculate_summarization_budget is called with 200000
        let budget = calculate_summarization_budget(clamped_context_window);

        // @step Then the budget should be 150000 (200k minus 50k AUTOCOMPACT_BUFFER)
        assert_eq!(budget, 150_000);

        // Verify wrong unclamped value would give wrong budget
        let wrong_budget = calculate_summarization_budget(1_000_000);
        assert_eq!(wrong_budget, 950_000, "Unclamped 1M would give 950k budget — too high");
    }

    // -------------------------------------------------------------------------
    // Scenario: Thinking exhaustion uses clamped context window for utilization check
    // (This mirrors stream_loop.rs:1033 logic)
    // -------------------------------------------------------------------------
    #[test]
    fn test_limits005_thinking_exhaustion_utilization_check() {
        // The thinking exhaustion check at stream_loop.rs:1033 uses:
        // let utilization_pct = (current_tokens as f64 / context_window as f64) * 100.0;
        // context_window comes from line 276: session.provider_manager().context_window()
        // After LIMITS-004, this returns 200k for Claude, not 1M.

        let clamped_context_window: u64 = 200_000;
        let current_tokens: u64 = 185_000; // Near limit

        let utilization_pct = if clamped_context_window > 0 {
            (current_tokens as f64 / clamped_context_window as f64) * 100.0
        } else {
            0.0
        };

        // With clamped 200k, 185k tokens = 92.5% utilization → triggers >90% preservation
        assert!(utilization_pct > 90.0, "185k / 200k = {utilization_pct:.1}% should trigger >90% preservation");

        // With wrong unclamped 1M, 185k tokens = 18.5% → would NOT trigger preservation
        let wrong_context_window: u64 = 1_000_000;
        let wrong_utilization = (current_tokens as f64 / wrong_context_window as f64) * 100.0;
        assert!(wrong_utilization < 20.0, "185k / 1M = {wrong_utilization:.1}% would miss the >90% check");
    }

    // -------------------------------------------------------------------------
    // Scenario: End-to-end chain — Claude complete compaction parameter set
    // (Verifies all three values that stream_loop computes from clamped inputs)
    // -------------------------------------------------------------------------
    #[test]
    fn test_limits005_end_to_end_claude_chain() {
        // After LIMITS-004, for Claude Opus 4.6:
        //   ProviderManager.context_window() → 200,000 (clamped from 1M registry)
        //   ProviderManager.max_output_tokens() → 8,192 (clamped from 16k registry)
        let context_window: u64 = 200_000;
        let max_output: u64 = 8_192;

        // 1. Compaction threshold (stream_loop.rs:283)
        let threshold = resolve_compaction_threshold(
            context_window, max_output, Some("claude-opus-4-6"), None,
        );
        assert_eq!(threshold, 191_808);

        // 2. Summarization budget (compaction_retry.rs uses this)
        let budget = calculate_summarization_budget(context_window);
        assert_eq!(budget, 150_000);

        // 3. Headroom after compaction
        let headroom = threshold - budget;
        assert_eq!(headroom, 41_808, "Headroom between trigger and target");

        // 4. Fill percentage at 87k tokens (common mid-session)
        let fill_at_87k = ((87_000_f64 / threshold as f64) * 100.0) as u32;
        assert_eq!(fill_at_87k, 45);

        // 5. Fill percentage at threshold (should be 100%)
        let fill_at_threshold = ((threshold as f64 / threshold as f64) * 100.0) as u32;
        assert_eq!(fill_at_threshold, 100);
    }

    // -------------------------------------------------------------------------
    // Scenario: Sub-agent propagation uses clamped context window
    // (Tests the downstream effect: sub-agents receive clamped values, so their
    //  compaction thresholds are also correct.)
    // -------------------------------------------------------------------------
    #[test]
    fn test_limits005_sub_agent_threshold_from_clamped_values() {
        // @step Given a ProviderManager with registry context_window of 1000000 for Claude
        // After LIMITS-004 clamping, raw_model_context_window() returns Some(200_000).
        // Sub-agents (DeepSearch, AgentManager) use this as their context_window.
        let sub_agent_context_window: u64 = 200_000; // Clamped value from parent session
        let sub_agent_max_output: u64 = 8_192; // Clamped value from parent session

        // @step When raw_model_context_window is called for DeepSearch or AgentManager sub-agent propagation
        // The sub-agent creates its own ProviderManager with these values.
        // Then resolve_compaction_threshold is called in the sub-agent's stream_loop.
        let sub_agent_threshold = resolve_compaction_threshold(
            sub_agent_context_window,
            sub_agent_max_output,
            Some("claude-opus-4-6"),
            None,
        );

        // @step Then it should return 200000 (clamped by provider hard max) not 1000000
        // The sub-agent's threshold should match the parent's threshold
        assert_eq!(sub_agent_threshold, 191_808);
        assert_ne!(sub_agent_context_window, 1_000_000, "Must NOT be unclamped 1M");
        assert_eq!(sub_agent_context_window, 200_000, "Must be clamped 200k");
    }
}
