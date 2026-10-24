use super::*;

#[test]
fn test_threshold_config_tokens_resolve() {
    assert_eq!(
        CompactionThresholdConfig::Tokens(150_000).resolve(200_000),
        150_000
    );
}

#[test]
fn test_threshold_config_percentage_resolve() {
    assert_eq!(
        CompactionThresholdConfig::Percentage(80).resolve(1_000_000),
        800_000
    );
}

#[test]
fn test_threshold_config_percentage_60() {
    assert_eq!(
        CompactionThresholdConfig::Percentage(60).resolve(200_000),
        120_000
    );
}

#[test]
fn test_builtin_claude_returns_80_percent() {
    for model in ["claude-sonnet-4", "claude-opus-4-6", "claude-3-haiku"] {
        assert!(matches!(
            builtin_compaction_threshold(Some(model)),
            Some(CompactionThresholdConfig::Percentage(80))
        ));
    }
}

#[test]
fn test_builtin_gemini_returns_80_percent() {
    assert!(matches!(
        builtin_compaction_threshold(Some("gemini-2.5-pro")),
        Some(CompactionThresholdConfig::Percentage(80))
    ));
}

#[test]
fn test_builtin_openai_returns_80_percent() {
    for (model, capacity, expected) in [("gpt-4o", 128_000, 102_400), ("o3-mini", 200_000, 160_000)]
    {
        assert_eq!(
            builtin_compaction_threshold(Some(model)).map(|config| config.resolve(capacity)),
            Some(expected)
        );
    }
}

#[test]
fn test_builtin_unknown_returns_80_percent() {
    for model in [Some("my-custom-llm"), None] {
        assert!(matches!(
            builtin_compaction_threshold(model),
            Some(CompactionThresholdConfig::Percentage(80))
        ));
    }
}

#[test]
fn test_resolve_priority_user_override() {
    let user_config = CompactionThresholdConfig::Tokens(150_000);
    assert_eq!(
        resolve_compaction_threshold(200_000, 8_192, Some("claude-sonnet-4"), Some(&user_config)),
        150_000
    );
}

#[test]
fn test_resolve_priority_builtin_gemini() {
    assert_eq!(
        resolve_compaction_threshold(1_000_000, 65_536, Some("gemini-2.5-pro"), None),
        800_000
    );
}

#[test]
fn test_resolve_priority_claude_default() {
    assert_eq!(
        resolve_compaction_threshold(200_000, 8_192, Some("claude-sonnet-4"), None),
        160_000
    );
}

#[test]
fn test_resolve_priority_unknown_default() {
    assert_eq!(
        resolve_compaction_threshold(100_000, 8_192, Some("my-custom-llm"), None),
        80_000
    );
}

#[test]
fn test_resolve_clamps_user_threshold_above_context() {
    let user_config = CompactionThresholdConfig::Tokens(300_000);
    assert_eq!(
        resolve_compaction_threshold(
            200_000,
            100_000,
            Some("claude-sonnet-4"),
            Some(&user_config)
        ),
        168_000
    );
}

#[test]
fn test_resolve_openai_gpt4o() {
    assert_eq!(
        resolve_compaction_threshold(128_000, 16_384, Some("gpt-4o"), None),
        102_400
    );
}

#[test]
fn test_from_type_value_percentage() {
    assert_eq!(
        CompactionThresholdConfig::from_type_value("percentage", 80).resolve(200_000),
        160_000
    );
}

#[test]
fn test_from_type_value_tokens() {
    assert_eq!(
        CompactionThresholdConfig::from_type_value("tokens", 150_000).resolve(200_000),
        150_000
    );
}

#[test]
fn test_from_type_value_unknown_defaults_to_tokens() {
    assert_eq!(
        CompactionThresholdConfig::from_type_value("unknown", 100_000).resolve(200_000),
        100_000
    );
}
