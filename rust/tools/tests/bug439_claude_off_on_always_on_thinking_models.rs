//! BUG-439 — Claude Off is honoured or refused on always-on thinking models.
//!
//! Feature: spec/features/claude-off-on-always-on-thinking-models.feature

use codelet_tools::facade::claude_off_config;
use serde_json::json;

type TestResult = Result<(), String>;

#[test]
fn off_is_expressed_in_the_form_each_always_on_model_accepts() -> TestResult {
    // @step Given the models "claude-sonnet-5-5", "claude-opus-5" and "claude-haiku-5-5"
    let models = ["claude-sonnet-5-5", "claude-opus-5", "claude-haiku-5-5"];
    // @step When thinking Off is configured for each
    let configs = models
        .iter()
        .map(|model| claude_off_config(model))
        .collect::<Result<Vec<_>, String>>()?;
    // @step Then Sonnet 5.5 sends between_tools and Opus 5 and Haiku 5.5 send disabled
    assert_eq!(
        configs,
        vec![
            Some(json!({"thinking": {"type": "between_tools"}})),
            Some(json!({"thinking": {"type": "disabled"}})),
            Some(json!({"thinking": {"type": "disabled"}})),
        ]
    );
    Ok(())
}

#[test]
fn off_is_refused_on_a_model_that_cannot_turn_thinking_off() {
    // @step Given the model "claude-opus-5-5"
    // @step When thinking Off is configured
    let config = claude_off_config("claude-opus-5-5");
    // @step Then the configuration fails saying thinking cannot be turned off
    let error = config.err().unwrap_or_default();
    assert!(error.contains("cannot be turned off"), "{error}");
}

#[test]
fn off_still_sends_nothing_on_a_model_whose_thinking_is_off_by_default() -> TestResult {
    // @step Given the model "claude-opus-4-6"
    // @step When thinking Off is configured
    let config = claude_off_config("claude-opus-4-6")?;
    // @step Then no thinking configuration is sent
    assert_eq!(config, None);
    Ok(())
}
