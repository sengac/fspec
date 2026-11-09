//! MODEL-005: the compaction threshold is computed from the ProviderManager's
//! per-model context window and output limit.
//!
//! Feature: spec/features/per-model-context-window-and-max-output-configuration.feature

use codelet_cli::compaction_threshold::{calculate_usable_context, compaction_threshold_for};
use codelet_providers::{ProviderManager, ProviderType};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The ProviderManager's resolved limits, as the compaction hook reads them.
fn limits(manager: &ProviderManager) -> Result<(u64, u64), Box<dyn std::error::Error>> {
    let context_window = manager
        .context_window()
        .ok_or("context window unavailable")?;
    let max_output = manager
        .max_output_tokens()
        .ok_or("output limit unavailable")?;
    Ok((u64::try_from(context_window)?, u64::try_from(max_output)?))
}

#[test]
fn compaction_threshold_uses_per_model_context_window_for_large_context_model() -> TestResult {
    // @step Given a ProviderManager with model_context_window=200000 and model_max_output_tokens=100000
    let manager = ProviderManager::for_testing(ProviderType::OpenAI, Some(200_000), Some(100_000));

    // @step When the compaction threshold is calculated
    let (context_window, max_output) = limits(&manager)?;
    let threshold = compaction_threshold_for(context_window, max_output, None, None);

    // @step Then calculate_usable_context(200000, 100000) should return 168000
    assert_eq!(
        calculate_usable_context(context_window, max_output),
        168_000
    );

    // @step And the default compaction threshold is 160000
    assert_eq!(threshold, 160_000);
    Ok(())
}

#[test]
fn compaction_threshold_uses_per_model_context_window_for_small_context_model() -> TestResult {
    // @step Given a ProviderManager with model_context_window=32000 and model_max_output_tokens=4096
    let manager = ProviderManager::for_testing(ProviderType::OpenAI, Some(32_000), Some(4_096));

    // @step When the compaction threshold is calculated
    let (context_window, max_output) = limits(&manager)?;
    let threshold = compaction_threshold_for(context_window, max_output, None, None);

    // @step Then calculate_usable_context(32000, 4096) should return 27904
    assert_eq!(calculate_usable_context(context_window, max_output), 27_904);

    // @step And the default compaction threshold is 25600
    assert_eq!(threshold, 25_600);
    Ok(())
}
