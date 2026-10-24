use codelet_cli::compaction_threshold::{
    builtin_compaction_threshold, resolve_compaction_threshold, CompactionThresholdConfig,
};

const MODELS: [Option<&str>; 8] = [
    Some("claude-sonnet-4"),
    Some("claude_opus_4"),
    Some("gemini-2.5-pro"),
    Some("gpt-4o"),
    Some("o3-mini"),
    Some("custom-model"),
    Some(""),
    None,
];

#[test]
fn all_model_identities_receive_the_same_default() {
    for model in MODELS {
        assert!(
            matches!(
                builtin_compaction_threshold(model),
                Some(CompactionThresholdConfig::Percentage(80))
            ),
            "unexpected default for {model:?}"
        );
        for (capacity, output, expected) in [
            (200_000, 8_192, 160_000),
            (400_000, 128_000, 320_000),
            (100_000, 32_000, 68_000),
            (20_000, 30_000, 0),
        ] {
            assert_eq!(
                resolve_compaction_threshold(capacity, output, model, None),
                expected,
                "unexpected threshold for {model:?} with capacity {capacity} and output {output}"
            );
        }
    }
}

#[test]
fn explicit_overrides_retain_priority_over_the_universal_default() {
    for model in MODELS {
        for (override_config, expected) in [
            (CompactionThresholdConfig::Tokens(150_000), 150_000),
            (CompactionThresholdConfig::Percentage(60), 120_000),
            (CompactionThresholdConfig::Percentage(90), 180_000),
            (CompactionThresholdConfig::Tokens(300_000), 191_808),
        ] {
            assert_eq!(
                resolve_compaction_threshold(200_000, 8_192, model, Some(&override_config)),
                expected,
                "override lost priority for {model:?}"
            );
        }
    }
}

#[test]
fn missing_limits_remain_unavailable_under_the_universal_default() {
    for model in MODELS {
        for (capacity, output) in [(0, 8_192), (200_000, 0), (0, 0)] {
            assert_eq!(
                resolve_compaction_threshold(capacity, output, model, None),
                0
            );
            assert_eq!(
                resolve_compaction_threshold(
                    capacity,
                    output,
                    model,
                    Some(&CompactionThresholdConfig::Tokens(150_000))
                ),
                0
            );
        }
    }
}
