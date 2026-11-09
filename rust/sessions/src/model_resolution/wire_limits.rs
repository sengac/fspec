//! PROV-172: a session's resolved limits converted to the wire's `u32`.
//!
//! Feature: spec/features/session-model-limits-wire.feature

use codelet_providers::ProviderManager;

use super::ResolvedModelLimits;

/// A resolved model limit that the `u32` RPC wire cannot carry. It is
/// reported, never clamped or truncated into a different number.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("the {limit} of {model} is {value} tokens, which exceeds the RPC wire maximum of {max} tokens", max = u32::MAX)]
pub struct WireLimitError {
    /// Which limit: `context window`, `max output tokens` or `compaction threshold`.
    pub limit: &'static str,
    /// The selected model (`provider/model`), or `no model` when none is selected.
    pub model: String,
    /// The value that does not fit.
    pub value: u64,
}

/// Read the manager's current limits and convert each to the wire's `u32`.
pub fn resolve_current_model_limits(
    pm: &ProviderManager,
) -> Result<ResolvedModelLimits, WireLimitError> {
    let model = pm
        .selected_model_string()
        .unwrap_or_else(|| "no selected model".to_string());
    let threshold = pm.compaction_threshold_override().map(|(kind, value)| {
        codelet_cli::compaction_threshold::CompactionThresholdConfig::from_type_value(kind, value)
    });
    let context_window = pm.context_window().map(|tokens| tokens as u64);
    let max_output_tokens = pm.max_output_tokens().map(|tokens| tokens as u64);
    let compaction_threshold = codelet_cli::compaction_threshold::resolve_compaction_threshold(
        context_window,
        max_output_tokens,
        pm.selected_model_id().as_deref(),
        threshold.as_ref(),
    );
    Ok(ResolvedModelLimits {
        context_window: to_wire("context window", &model, context_window)?,
        max_output_tokens: to_wire("max output tokens", &model, max_output_tokens)?,
        compaction_threshold: to_wire("compaction threshold", &model, compaction_threshold)?,
    })
}

fn to_wire(
    limit: &'static str,
    model: &str,
    value: Option<u64>,
) -> Result<Option<u32>, WireLimitError> {
    value
        .map(|value| {
            u32::try_from(value).map_err(|_| WireLimitError {
                limit,
                model: model.to_string(),
                value,
            })
        })
        .transpose()
}
