//! BUG-613 — the TUI reports that a delete cleared fspec's default model.
//!
//! Feature: spec/features/deleting-the-default-model-clears-the-default.feature

use super::default_cleared_notice;

#[test]
fn the_tui_reports_that_the_deleted_model_was_the_default() {
    // @step Given the TUI's default model is "openai:local/m1"
    let before = Some("openai:local/m1".to_string());

    // @step When the TUI deletes the custom model m1 of openai profile "local"
    // (the daemon clears the default in the same delete)
    let after: Option<String> = None;
    let notice = default_cleared_notice(before.as_deref(), after.as_deref());

    // @step Then the session shows a notice that "openai:local/m1" was the default model and that /model chooses a new one
    let notice = notice.unwrap_or_default();
    assert!(notice.contains("openai:local/m1"), "{notice}");
    assert!(notice.contains("default model"), "{notice}");
    assert!(notice.contains("/model"), "{notice}");
    assert_eq!(
        default_cleared_notice(
            Some("anthropic/claude-opus-4-5"),
            Some("anthropic/claude-opus-4-5")
        ),
        None
    );
    assert_eq!(default_cleared_notice(None, None), None);
}
