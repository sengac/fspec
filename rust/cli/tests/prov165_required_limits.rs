//! PROV-165: work that needs model limits fails naming the unavailable limit,
//! and no compaction threshold exists without both limits.
//!
//! Feature: spec/features/required-model-limits.feature

use codelet_cli::compaction_threshold::resolve_compaction_threshold;
use codelet_cli::model_limits::{required_limits, ModelLimitKind};
use codelet_providers::{ProviderManager, ProviderType};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn no_compaction_threshold_exists_without_both_limits() {
    // @step Given a context window of 200000 and an output limit of 8192
    let (context_window, output_limit) = (Some(200_000_u64), Some(8_192_u64));

    // @step When the compaction threshold is resolved with either limit unavailable
    let without_window = resolve_compaction_threshold(None, output_limit, None, None);
    let without_output = resolve_compaction_threshold(context_window, None, None, None);

    // @step Then no compaction threshold exists
    assert_eq!(without_window, None);
    assert_eq!(without_output, None);

    // @step And with both limits available the threshold is positive
    let threshold = resolve_compaction_threshold(context_window, output_limit, None, None);
    assert!(threshold.is_some_and(|threshold| threshold > 0));
}

#[test]
fn work_that_needs_model_limits_fails_naming_the_unavailable_limit() -> TestResult {
    // @step Given a Codex provider manager for "codex/unknown-model" with no catalog or user limits
    let mut manager = ProviderManager::for_testing(ProviderType::Codex, None, None);
    manager.set_model_direct("codex", "unknown-model", None, None, None)?;

    // @step When the limits a prompt needs are requested
    let Err(error) = required_limits(&manager) else {
        return Err("limits of an uncatalogued Codex model must be unavailable".into());
    };

    // @step Then the request fails with an error naming the unavailable context window
    assert_eq!(error.limit, ModelLimitKind::ContextWindow);
    assert!(
        error.to_string().contains("context window unavailable"),
        "{error}"
    );

    // @step And the error names the model "codex/unknown-model"
    assert!(error.to_string().contains("codex/unknown-model"), "{error}");
    Ok(())
}
