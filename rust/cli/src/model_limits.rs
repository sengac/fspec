//! PROV-165: model limits that work cannot do without.
//!
//! Feature: spec/features/required-model-limits.feature
//!
//! `ProviderManager` reports an unavailable limit as `None`. Prompt preflight,
//! the compactor's publication and turn compaction need numbers, so they ask
//! [`required_limits`], which names the missing limit and the selected model
//! instead of budgeting against 0.

use codelet_providers::ProviderManager;

/// Which model limit is unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelLimitKind {
    ContextWindow,
    MaxOutputTokens,
}

impl std::fmt::Display for ModelLimitKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::ContextWindow => "context window",
            Self::MaxOutputTokens => "max output tokens",
        })
    }
}

/// A limit the work needs is unavailable for the selected model.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "{limit} unavailable for {model}: set it in the model settings or select a catalogued model"
)]
pub struct ModelLimitUnavailable {
    pub limit: ModelLimitKind,
    pub model: String,
}

/// Both limits of the selected model, known to be available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequiredLimits {
    pub context_window: u64,
    pub max_output_tokens: u64,
}

/// The selected model's limits, or the first unavailable one.
pub fn required_limits(manager: &ProviderManager) -> Result<RequiredLimits, ModelLimitUnavailable> {
    let unavailable = |limit| ModelLimitUnavailable {
        limit,
        model: manager
            .selected_model_string()
            .unwrap_or_else(|| manager.current_provider_name().to_string()),
    };
    let context_window = manager
        .context_window()
        .ok_or_else(|| unavailable(ModelLimitKind::ContextWindow))?;
    let max_output_tokens = manager
        .max_output_tokens()
        .ok_or_else(|| unavailable(ModelLimitKind::MaxOutputTokens))?;
    Ok(RequiredLimits {
        context_window: context_window as u64,
        max_output_tokens: max_output_tokens as u64,
    })
}
