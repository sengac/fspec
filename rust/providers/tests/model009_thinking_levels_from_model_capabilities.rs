//! Feature: spec/features/thinking-levels-from-model-capabilities.feature

use codelet_providers::models::{
    BudgetRange, ModelRegistry, ModelsDevResponse, ReasoningCapability, ReasoningLevel,
};
use codelet_test_helpers::{OrFail, OrFailErr, TestError, TestResult};
use serde_json::{json, Value};

/// A models.dev catalog of `(provider, model, reasoning_options)` entries;
/// `Value::Null` leaves `reasoning_options` out of the model entirely.
fn catalog(entries: &[(&str, &str, Value)]) -> Result<ModelRegistry, TestError> {
    let mut providers = serde_json::Map::new();
    for (provider, model, options) in entries {
        let mut entry = json!({
            "id": model,
            "name": model,
            "reasoning": !options.is_null(),
            "limit": { "context": 200000, "output": 32000 },
        });
        if !options.is_null() {
            entry["reasoning_options"] = options.clone();
        }
        let slot = providers
            .entry((*provider).to_string())
            .or_insert_with(|| json!({ "id": provider, "name": provider, "models": {} }));
        slot["models"][*model] = entry;
    }
    let response: ModelsDevResponse = serde_json::from_value(Value::Object(providers))?;
    Ok(ModelRegistry::from_response(response))
}

fn efforts(values: &[&str]) -> Value {
    json!([{ "type": "effort", "values": values }])
}

fn capability(
    registry: &ModelRegistry,
    provider: &str,
    model: &str,
) -> Result<ReasoningCapability, TestError> {
    Ok(registry
        .reasoning_capability(provider, model)?
        .or_fail("the model has a reasoning capability")?)
}

#[test]
fn an_effort_list_model_offers_exactly_its_advertised_efforts_in_canonical_order() -> TestResult {
    // @step Given a models.dev catalog where openai/gpt-5.5 advertises efforts "xhigh, none, high, low, medium"
    let registry = catalog(&[(
        "openai",
        "gpt-5.5",
        efforts(&["xhigh", "none", "high", "low", "medium"]),
    )])?;
    // @step When the reasoning capability of openai/gpt-5.5 is resolved
    let gpt = capability(&registry, "openai", "gpt-5.5")?;
    // @step Then its efforts are "none, low, medium, high, xhigh"
    assert_eq!(gpt.efforts(), ["none", "low", "medium", "high", "xhigh"]);
    // @step And it takes no token budget
    assert_eq!(gpt.budget(), None);
    // @step And it has no toggle
    assert!(!gpt.has_toggle());
    Ok(())
}

#[test]
fn a_model_can_advertise_both_efforts_and_a_token_budget() -> TestResult {
    // @step Given a models.dev catalog where anthropic/claude-opus-4-6 advertises efforts "low, medium, high, max" and a token budget of at least 1024
    let registry = catalog(&[(
        "anthropic",
        "claude-opus-4-6",
        json!([
            { "type": "effort", "values": ["low", "medium", "high", "max"] },
            { "type": "budget_tokens", "min": 1024 },
        ]),
    )])?;
    // @step When the reasoning capability of anthropic/claude-opus-4-6 is resolved
    let opus = capability(&registry, "anthropic", "claude-opus-4-6")?;
    // @step Then its efforts are "low, medium, high, max"
    assert_eq!(opus.efforts(), ["low", "medium", "high", "max"]);
    // @step And it takes a token budget from 1024 with no upper limit
    assert_eq!(
        opus.budget(),
        Some(BudgetRange {
            min: Some(1024),
            max: None
        })
    );
    Ok(())
}

#[test]
fn a_toggle_only_model_has_no_effort_list() -> TestResult {
    // @step Given a models.dev catalog where zai/glm-4.5 advertises only a toggle
    let registry = catalog(&[("zai", "glm-4.5", json!([{ "type": "toggle" }]))])?;
    // @step When the reasoning capability of zai/glm-4.5 is resolved
    let glm = capability(&registry, "zai", "glm-4.5")?;
    // @step Then it has a toggle
    assert!(glm.has_toggle());
    // @step And its efforts are empty
    assert!(glm.efforts().is_empty());
    // @step And it takes no token budget
    assert_eq!(glm.budget(), None);
    Ok(())
}

#[test]
fn a_model_with_no_reasoning_options_has_no_reasoning_control() -> TestResult {
    // @step Given a models.dev catalog where example/plain-model has an empty reasoning_options list
    // @step And example/older-model has no reasoning_options at all
    let registry = catalog(&[
        ("example", "plain-model", json!([])),
        ("example", "older-model", Value::Null),
    ])?;
    // @step When the reasoning capabilities of both models are resolved
    let plain = registry.reasoning_capability("example", "plain-model")?;
    let older = registry.reasoning_capability("example", "older-model")?;
    // @step Then neither model has a reasoning capability
    assert!(plain.is_none());
    assert!(older.is_none());
    // @step And choosing effort "low" for example/plain-model is refused with an error naming example/plain-model
    let refused = registry
        .check_reasoning_level("example", "plain-model", &ReasoningLevel::effort("low")?)
        .or_fail_err("a model without reasoning control refuses every level")?;
    assert!(
        refused.to_string().contains("example/plain-model"),
        "{refused}"
    );
    Ok(())
}

#[test]
fn an_effort_the_model_does_not_advertise_is_refused_not_remapped() -> TestResult {
    // @step Given a models.dev catalog where anthropic/claude-opus-4-6 advertises efforts "low, medium, high, max"
    let registry = catalog(&[(
        "anthropic",
        "claude-opus-4-6",
        efforts(&["low", "medium", "high", "max"]),
    )])?;
    // @step When effort "xhigh" is chosen for anthropic/claude-opus-4-6
    let outcome = registry.check_reasoning_level(
        "anthropic",
        "claude-opus-4-6",
        &ReasoningLevel::effort("xhigh")?,
    );
    // @step Then the choice is refused with an error naming anthropic/claude-opus-4-6
    let refused = outcome.or_fail_err("xhigh is not advertised")?.to_string();
    assert!(refused.contains("anthropic/claude-opus-4-6"), "{refused}");
    // @step And the error lists "low, medium, high, max"
    assert!(refused.contains("low, medium, high, max"), "{refused}");
    Ok(())
}

#[test]
fn a_token_budget_outside_the_advertised_range_is_refused() -> TestResult {
    // @step Given a models.dev catalog where anthropic/claude-haiku-4-5 takes a token budget of at least 1024
    // @step And google/gemini-2.5-flash takes a token budget from 0 to 24576
    let registry = catalog(&[
        (
            "anthropic",
            "claude-haiku-4-5",
            json!([{ "type": "budget_tokens", "min": 1024 }]),
        ),
        (
            "google",
            "gemini-2.5-flash",
            json!([{ "type": "toggle" }, { "type": "budget_tokens", "min": 0, "max": 24576 }]),
        ),
    ])?;
    // @step When a 512-token budget is chosen for anthropic/claude-haiku-4-5
    let haiku = registry.check_reasoning_level(
        "anthropic",
        "claude-haiku-4-5",
        &ReasoningLevel::Budget(512),
    );
    // @step And a 30000-token budget is chosen for google/gemini-2.5-flash
    let flash = registry.check_reasoning_level(
        "google",
        "gemini-2.5-flash",
        &ReasoningLevel::Budget(30000),
    );
    // @step Then both choices are refused with errors naming the model and its budget range
    let haiku = haiku.or_fail_err("512 is below 1024")?.to_string();
    assert!(
        haiku.contains("anthropic/claude-haiku-4-5") && haiku.contains("1024"),
        "{haiku}"
    );
    let flash = flash.or_fail_err("30000 is above 24576")?.to_string();
    assert!(
        flash.contains("google/gemini-2.5-flash") && flash.contains("24576"),
        "{flash}"
    );
    // @step And a 2048-token budget is accepted for anthropic/claude-haiku-4-5
    assert_eq!(
        registry.check_reasoning_level(
            "anthropic",
            "claude-haiku-4-5",
            &ReasoningLevel::Budget(2048)
        )?,
        ReasoningLevel::Budget(2048)
    );
    Ok(())
}

#[test]
fn a_supported_effort_is_accepted_unchanged() -> TestResult {
    // @step Given a models.dev catalog where openai/gpt-5.5 advertises efforts "none, low, medium, high, xhigh"
    let registry = catalog(&[(
        "openai",
        "gpt-5.5",
        efforts(&["none", "low", "medium", "high", "xhigh"]),
    )])?;
    // @step When effort "high" is chosen for openai/gpt-5.5
    let accepted =
        registry.check_reasoning_level("openai", "gpt-5.5", &ReasoningLevel::effort("high")?)?;
    // @step Then the choice is accepted as effort "high"
    assert_eq!(accepted, ReasoningLevel::effort("high")?);
    Ok(())
}

#[test]
fn an_unrecognised_reasoning_option_type_fails_only_that_model() -> TestResult {
    // @step Given a models.dev catalog where example/odd-model advertises a reasoning option of type "temperature_curve"
    // @step And openai/gpt-5.5 in the same catalog advertises efforts "none, low, medium, high, xhigh"
    // @step When the catalog is loaded
    let registry = catalog(&[
        (
            "example",
            "odd-model",
            json!([{ "type": "temperature_curve", "points": 3 }]),
        ),
        (
            "openai",
            "gpt-5.5",
            efforts(&["none", "low", "medium", "high", "xhigh"]),
        ),
    ])?;
    // @step Then resolving the capability of example/odd-model is an error naming "temperature_curve"
    let odd = registry
        .reasoning_capability("example", "odd-model")
        .or_fail_err("an unrecognised option type is an error")?
        .to_string();
    assert!(
        odd.contains("temperature_curve") && odd.contains("example/odd-model"),
        "{odd}"
    );
    // @step And the capability of openai/gpt-5.5 still resolves
    assert_eq!(
        capability(&registry, "openai", "gpt-5.5")?.efforts(),
        ["none", "low", "medium", "high", "xhigh"]
    );
    Ok(())
}

#[test]
fn a_reasoning_option_with_unusable_data_fails_only_that_model() -> TestResult {
    // @step Given a models.dev catalog where nvidia/nemotron-3-ultra-550b-a55b lists a token budget with min -1 and max 32768
    // @step And openai/gpt-5.5 in the same catalog advertises efforts "none, low, medium, high, xhigh"
    // @step When the catalog is loaded
    let registry = catalog(&[
        (
            "nvidia",
            "nemotron-3-ultra-550b-a55b",
            json!([{ "type": "budget_tokens", "min": -1, "max": 32768 }]),
        ),
        (
            "openai",
            "gpt-5.5",
            efforts(&["none", "low", "medium", "high", "xhigh"]),
        ),
    ])?;
    // @step Then resolving the capability of nvidia/nemotron-3-ultra-550b-a55b is an error naming the bound -1
    let nemotron = registry
        .reasoning_capability("nvidia", "nemotron-3-ultra-550b-a55b")
        .or_fail_err("a negative budget bound is unusable")?
        .to_string();
    assert!(
        nemotron.contains("-1") && nemotron.contains("nvidia/nemotron-3-ultra-550b-a55b"),
        "{nemotron}"
    );
    // @step And the capability of openai/gpt-5.5 still resolves
    assert_eq!(
        capability(&registry, "openai", "gpt-5.5")?.efforts(),
        ["none", "low", "medium", "high", "xhigh"]
    );
    Ok(())
}
