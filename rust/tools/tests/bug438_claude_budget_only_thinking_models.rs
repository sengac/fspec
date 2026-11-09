//! BUG-438 — budget-only Claude models receive budgeted thinking.
//!
//! Feature: spec/features/claude-budget-only-thinking-models.feature

use codelet_tools::facade::is_adaptive_thinking_model;

#[test]
fn claude_4_0_and_4_1_ids_are_budgeted_while_4_6_stays_adaptive() {
    // @step Given the models "claude-opus-4-1-20250805", "claude-sonnet-4-20250514" and "claude-opus-4-6"
    let models = [
        "claude-opus-4-1-20250805",
        "claude-sonnet-4-20250514",
        "claude-opus-4-6",
    ];
    // @step When each is checked for adaptive thinking
    let adaptive: Vec<&str> = models
        .into_iter()
        .filter(|model| is_adaptive_thinking_model(model))
        .collect();
    // @step Then only "claude-opus-4-6" uses adaptive thinking
    assert_eq!(adaptive, vec!["claude-opus-4-6"]);
}
