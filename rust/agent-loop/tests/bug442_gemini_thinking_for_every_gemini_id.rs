//! BUG-442 — Gemini thinking for every Gemini 3.x and 2.5 id, nothing forced.
//!
//! Feature: spec/features/gemini-thinking-for-every-gemini-id.feature

use codelet_agent_loop::thinking_config::{ThinkingTarget, ThinkingWire};
use codelet_agent_loop::thinking_level_detection::base_thinking_level;
use codelet_providers::gemini::gemini_generation_config;
use codelet_providers::models::ReasoningCapability;
use codelet_providers::models::ReasoningOption;
use codelet_rpc_types::ThinkingLevel;
use serde_json::json;

type TestResult = Result<(), String>;

/// What `model`'s Off sends; `toggle` says whether it advertises a thinking
/// toggle (models.dev reasoning_options).
fn off(model: &str, toggle: bool) -> Result<Option<serde_json::Value>, String> {
    let mut options = vec![ReasoningOption::Effort(vec!["low".into(), "high".into()])];
    if toggle {
        options.push(ReasoningOption::Toggle);
    }
    let id = format!("google/{model}");
    let capability =
        ReasoningCapability::resolve(&id, Some(options.as_slice())).map_err(|e| e.to_string())?;
    let target = ThinkingTarget::new(Ok(ThinkingWire::Gemini), &id, model, Ok(capability));
    target.config_for(Some(
        &base_thinking_level(&ThinkingLevel::Off)?.ok_or("no Off")?,
    ))
}

#[test]
fn off_is_expressed_or_refused_per_gemini_model() -> TestResult {
    // @step Given gemini-3-pro-preview and gemini-2.5-pro advertise no thinking toggle and gemini-2.5-flash advertises one
    // @step When thinking Off is configured for each
    let gemini_3 = off("gemini-3-pro-preview", false);
    let pro = off("gemini-2.5-pro", false);
    let flash = off("gemini-2.5-flash", true)?;
    // @step Then Gemini 3 and 2.5 Pro are refused and 2.5 Flash sends thinkingBudget 0
    for refused in [gemini_3, pro] {
        let error = refused.err().unwrap_or_default();
        assert!(error.contains("cannot be turned off"), "{error}");
    }
    assert_eq!(
        flash,
        Some(json!({"thinkingConfig": {"thinkingBudget": 0}}))
    );
    Ok(())
}

#[test]
fn no_thinking_config_means_no_thinking_is_forced() {
    // @step Given a Gemini 3 request without a thinking configuration
    // @step When its generation config is built
    let generation = gemini_generation_config(None);
    // @step Then it carries no thinkingConfig
    assert_eq!(
        generation,
        json!({"generationConfig": {"temperature": 1.0, "topP": 0.95, "topK": 64}})
    );
}
