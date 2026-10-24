use super::*;

#[test]
fn test_constants_defined() {
    assert_eq!(AUTOCOMPACT_BUFFER, 50_000);
    assert_eq!(SESSION_OUTPUT_TOKEN_MAX, 32_000);
}

#[test]
fn test_usable_context_claude_sonnet() {
    assert_eq!(SESSION_OUTPUT_TOKEN_MAX, 32_000);
    assert_eq!(calculate_usable_context(200_000, 8_192), 191_808);
}

#[test]
fn test_usable_context_gpt4() {
    assert_eq!(SESSION_OUTPUT_TOKEN_MAX, 32_000);
    assert_eq!(calculate_usable_context(128_000, 4_096), 123_904);
}

#[test]
fn test_usable_context_high_output_capped() {
    assert_eq!(SESSION_OUTPUT_TOKEN_MAX, 32_000);
    assert_eq!(calculate_usable_context(200_000, 64_000), 168_000);
}

#[test]
fn test_usable_context_zero_max_output_unavailable() {
    assert_eq!(calculate_usable_context(100_000, 0), 0);
}

#[test]
fn test_calculate_budget_large_context() {
    assert_eq!(calculate_summarization_budget(200_000), 150_000);
}

#[test]
fn test_calculate_budget_small_context() {
    assert_eq!(calculate_summarization_budget(40_000), 32_000);
}

#[test]
fn test_calculate_budget_equal_to_buffer() {
    assert_eq!(calculate_summarization_budget(50_000), 40_000);
}

#[test]
fn test_compaction_threshold_large_context_model() {
    let usable = calculate_usable_context(200_000, 100_000);
    assert_eq!(usable, 168_000);
    assert!(168_001 > usable);
}

#[test]
fn test_compaction_threshold_small_context_model() {
    let usable = calculate_usable_context(32_000, 4_096);
    assert_eq!(usable, 27_904);
    assert!(27_905 > usable);
}
