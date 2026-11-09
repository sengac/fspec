//! BUG-441 — Codex reasoning for every Codex model without invented
//! capabilities.
//!
//! Feature: spec/features/codex-reasoning-for-every-codex-model.feature

use codelet_agent_loop::thinking_config::{ThinkingTarget, ThinkingWire};
use codelet_agent_loop::thinking_level_detection::base_thinking_level;
use codelet_providers::codex::request_options::{resolve, CodexCapabilities};
use codelet_providers::ProviderType;
use codelet_rpc_types::ThinkingLevel;
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn codex_levels_become_reasoning_effort_for_any_codex_model_id() -> TestResult {
    // @step Given the "codex" provider
    let target = ThinkingTarget::new(
        ThinkingWire::for_dispatch(&ProviderType::Codex, None, None),
        "codex/gpt-5.5",
        "gpt-5.5",
        advertised(),
    );
    // @step When the Medium and Off thinking levels are configured
    let medium = target.config_for(Some(
        &base_thinking_level(&ThinkingLevel::Effort("medium".to_string()))?.ok_or("no Medium")?,
    ))?;
    let off = target.config_for(Some(
        &base_thinking_level(&ThinkingLevel::Off)?.ok_or("no Off")?,
    ))?;
    // @step Then Medium gives reasoning effort medium with summary auto and Off gives reasoning effort none with summary auto
    assert_eq!(
        medium,
        Some(json!({"reasoning": {"effort": "medium", "summary": "auto"}}))
    );
    assert_eq!(
        off,
        Some(json!({"reasoning": {"effort": "none", "summary": "auto"}}))
    );
    Ok(())
}

#[test]
fn without_fresh_capabilities_no_effort_is_invented_and_explicit_options_are_sent_unchanged(
) -> TestResult {
    // @step Given Codex capability data is missing or stale
    let stale: CodexCapabilities = serde_json::from_value(json!({
        "stale": true,
        "efforts": ["low"],
        "defaults": {"reasoning": {"effort": "low"}}
    }))?;
    for capabilities in [None, Some(&stale)] {
        // @step When requests use defaults or explicitly request xhigh effort
        let defaults = resolve(None, capabilities)?;
        let explicit = resolve(
            Some(&json!({"reasoning": {"effort": "xhigh"}})),
            capabilities,
        )?;
        // @step Then the defaults carry only the required Codex fields and the explicit xhigh effort is sent as xhigh
        assert_eq!(
            defaults,
            json!({"store": false, "include": ["reasoning.encrypted_content"]})
        );
        assert_eq!(explicit["reasoning"], json!({"effort": "xhigh"}));
    }
    Ok(())
}

/// MODEL-017: the levels the model advertises (none, low, medium, high); a
/// model that advertises none has no reasoning control.
fn advertised() -> Result<Option<codelet_providers::models::ReasoningCapability>, String> {
    let options: Vec<codelet_providers::models::ReasoningOption> = serde_json::from_value(
        serde_json::json!([{"type": "effort", "values": ["none", "low", "medium", "high"]}]),
    )
    .map_err(|error| error.to_string())?;
    codelet_providers::models::ReasoningCapability::resolve("provider/model", Some(&options))
        .map_err(|error| error.to_string())
}
