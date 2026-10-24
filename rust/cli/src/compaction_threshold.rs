//! Compaction threshold calculation and post-compaction budgets.
//!
//! Every model defaults to 80% of its context window, bounded by usable
//! context after output reservation. Explicit token or percentage overrides
//! take priority over that default.
//!
//! Reference: spec/features/universal-compaction-default.feature
//!            spec/features/per-model-compaction-threshold.feature

/// Buffer subtracted when calculating a post-compaction summarization budget.
///
/// This target budget is separate from the compaction trigger. For a 200k
/// context, the default trigger is 160k and the target budget is 150k.
pub const AUTOCOMPACT_BUFFER: u64 = 50_000;

/// Calculate the summarization budget independently of the trigger threshold.
///
/// Context windows up to the buffer use 80% of their capacity. Larger windows
/// subtract the buffer instead.
///
/// # Examples
/// ```
/// use codelet_cli::compaction_threshold::calculate_summarization_budget;
/// assert_eq!(calculate_summarization_budget(200_000), 150_000);
/// assert_eq!(calculate_summarization_budget(40_000), 32_000);
/// ```
pub fn calculate_summarization_budget(context_window: u64) -> u64 {
    if context_window <= AUTOCOMPACT_BUFFER {
        (context_window as f64 * 0.8) as u64
    } else {
        context_window - AUTOCOMPACT_BUFFER
    }
}

/// Upper bound on the output reservation used by compaction thresholds.
pub const SESSION_OUTPUT_TOKEN_MAX: u64 = 32_000;

/// Calculate usable context after reserving up to 32k output tokens.
///
/// Missing context or output limits leave no usable context available.
pub fn calculate_usable_context(context_window: u64, model_max_output: u64) -> u64 {
    if context_window == 0 || model_max_output == 0 {
        return 0;
    }
    let output_reservation = model_max_output.min(SESSION_OUTPUT_TOKEN_MAX);
    context_window.saturating_sub(output_reservation)
}

/// User-configurable compaction trigger, independent of model identity.
#[derive(Debug, Clone)]
pub enum CompactionThresholdConfig {
    /// Absolute token count.
    Tokens(u64),
    /// Percentage of the context window.
    Percentage(u8),
}

impl CompactionThresholdConfig {
    /// Resolve an absolute token count against the supplied context window.
    pub fn resolve(&self, context_window: u64) -> u64 {
        match self {
            Self::Tokens(tokens) => *tokens,
            Self::Percentage(pct) => (context_window as f64 * (*pct as f64 / 100.0)) as u64,
        }
    }

    /// Construct from NAPI/TUI settings; unrecognized types use token mode.
    pub fn from_type_value(type_str: &str, value: u64) -> Self {
        if type_str == "percentage" {
            Self::Percentage(value as u8)
        } else {
            Self::Tokens(value)
        }
    }
}

/// Return the universal 80% default, including unknown or absent model IDs.
///
/// The optional return type is retained for compatibility with existing callers.
pub fn builtin_compaction_threshold(_model_id: Option<&str>) -> Option<CompactionThresholdConfig> {
    Some(CompactionThresholdConfig::Percentage(80))
}

/// Resolve an explicit override or the universal 80% default.
///
/// Every result is clamped to usable context after output reservation.
pub fn resolve_compaction_threshold(
    context_window: u64,
    max_output: u64,
    model_id: Option<&str>,
    user_config: Option<&CompactionThresholdConfig>,
) -> u64 {
    let base_threshold = calculate_usable_context(context_window, max_output);
    let config = user_config
        .cloned()
        .or_else(|| builtin_compaction_threshold(model_id));
    config.map_or(base_threshold, |config| {
        config.resolve(context_window).min(base_threshold)
    })
}

#[cfg(test)]
#[path = "compaction_threshold/capacity_tests.rs"]
mod capacity_tests;

#[cfg(test)]
#[path = "compaction_threshold/configuration_tests.rs"]
mod configuration_tests;

#[cfg(test)]
#[path = "compaction_threshold/catalog_tests.rs"]
mod catalog_tests;
