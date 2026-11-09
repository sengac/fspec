//! MODEL-010 — thinking levels reach each provider through an explicit
//! wire table chosen by the dispatched provider.
//!
//! Feature: spec/features/thinking-levels-reach-each-provider-through-a-wire-table.feature

#[allow(dead_code)]
#[path = "prov145_profile_config_applied_support/mod.rs"]
mod support;

use codelet_agent_loop::agent_loop::thinking::resolve_turn_thinking;
use codelet_agent_loop::thinking_config::{ThinkingTarget, ThinkingWire, TurnLevel};
use codelet_agent_loop::thinking_level_detection::base_thinking_level;
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_providers::local_reasoning::{LocalEngine, ThinkingLevelsDeclaration};
use codelet_providers::models::{
    ModelRegistry, ModelsDevResponse, ReasoningCapability, ReasoningLevel,
};
use codelet_providers::ProviderType;
use codelet_rpc_types::{SessionId, ThinkingLevel};
use codelet_test_helpers::{OrFail, TestError, TestResult};
use serde_json::{json, Value};

/// A models.dev catalog of `(provider, model, reasoning_options)` entries.
fn catalog(entries: &[(&str, &str, Value)]) -> Result<ModelRegistry, TestError> {
    let mut providers = serde_json::Map::new();
    for (provider, model, options) in entries {
        let entry = json!({
            "id": model,
            "name": model,
            "reasoning": true,
            "limit": { "context": 200000, "output": 32000 },
            "reasoning_options": options,
        });
        let slot = providers
            .entry((*provider).to_string())
            .or_insert_with(|| json!({ "id": provider, "name": provider, "models": {} }));
        slot["models"][*model] = entry;
    }
    let response: ModelsDevResponse = serde_json::from_value(Value::Object(providers))?;
    Ok(ModelRegistry::from_response(response))
}

fn efforts(values: &[&str]) -> Value {
    json!({ "type": "effort", "values": values })
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

/// A turn target on `wire` for `provider/model` with `capability`.
fn target(
    wire: ThinkingWire,
    provider: &str,
    model: &str,
    capability: Option<ReasoningCapability>,
) -> ThinkingTarget {
    ThinkingTarget::new(
        Ok(wire),
        &format!("{provider}/{model}"),
        model,
        Ok(capability),
    )
}

fn dispatch(provider: ProviderType, profile: Option<&str>) -> Result<ThinkingWire, TestError> {
    Ok(ThinkingWire::for_dispatch(&provider, None, profile)?)
}

/// The level a session stores as Off (0), Low (1), Medium (2) or High (3).
fn session_level(stored: ThinkingLevel) -> Result<TurnLevel, TestError> {
    Ok(base_thinking_level(&stored)?.or_fail("the session has a chosen level")?)
}

const OFF: ThinkingLevel = ThinkingLevel::Off;
fn high() -> ThinkingLevel {
    ThinkingLevel::Effort("high".to_string())
}

fn effort(name: &str) -> Result<TurnLevel, TestError> {
    Ok(TurnLevel::Level(ReasoningLevel::effort(name)?))
}

/// The thinking parameters a turn at `level` sends to `target`.
fn sent(target: &ThinkingTarget, level: &TurnLevel) -> Result<Value, TestError> {
    Ok(target
        .config_for(Some(level))?
        .or_fail("the turn sends a thinking parameter")?)
}

/// The error a turn at `level` on `target` is refused with.
fn refused(target: &ThinkingTarget, level: &TurnLevel) -> Result<String, TestError> {
    match target.config_for(Some(level)) {
        Ok(config) => Err(format!("expected a refusal, got {config:?}").into()),
        Err(reason) => Ok(reason),
    }
}

#[test]
fn the_serving_provider_decides_the_wire_form_not_the_model_id() -> TestResult {
    // @step Given claude-sonnet-4-6 is served by the github-copilot provider and advertises efforts "low, medium, high"
    let registry = catalog(&[(
        "github-copilot",
        "claude-sonnet-4-6",
        json!([efforts(&["low", "medium", "high"])]),
    )])?;
    let wire = dispatch(ProviderType::GitHubCopilot, None)?;
    assert_eq!(wire, ThinkingWire::OpenAiChat);
    let copilot = target(
        wire,
        "github-copilot",
        "claude-sonnet-4-6",
        Some(capability(
            &registry,
            "github-copilot",
            "claude-sonnet-4-6",
        )?),
    );
    // @step When the session's level is High
    let config = sent(&copilot, &session_level(high())?)?;
    // @step Then the request carries reasoning_effort "high"
    assert_eq!(config, json!({"reasoning_effort": "high"}));
    // @step And it carries no Claude thinking object
    assert!(config.get("thinking").is_none() && config.get("output_config").is_none());
    Ok(())
}

#[test]
fn openai_chat_completions_receives_the_effort_name_unchanged() -> TestResult {
    // @step Given openai/gpt-5.5 on the openai provider advertises efforts "none, low, medium, high, xhigh"
    let registry = catalog(&[(
        "openai",
        "gpt-5.5",
        json!([efforts(&["none", "low", "medium", "high", "xhigh"])]),
    )])?;
    let gpt = target(
        dispatch(ProviderType::OpenAI, None)?,
        "openai",
        "gpt-5.5",
        Some(capability(&registry, "openai", "gpt-5.5")?),
    );
    // @step When the chosen level is the effort "xhigh"
    let config = sent(&gpt, &effort("xhigh")?)?;
    // @step Then the request carries reasoning_effort "xhigh"
    assert_eq!(config, json!({"reasoning_effort": "xhigh"}));
    Ok(())
}

#[test]
fn codex_receives_the_effort_with_an_automatic_summary() -> TestResult {
    // @step Given gpt-5.5 is served by the codex provider and advertises efforts "none, low, medium, high, xhigh"
    let registry = catalog(&[(
        "openai",
        "gpt-5.5",
        json!([efforts(&["none", "low", "medium", "high", "xhigh"])]),
    )])?;
    let wire = dispatch(ProviderType::Codex, None)?;
    assert_eq!(wire, ThinkingWire::OpenAiResponses);
    let codex = target(
        wire,
        "codex",
        "gpt-5.5",
        Some(capability(&registry, "openai", "gpt-5.5")?),
    );
    // @step When the session's level is High
    let config = sent(&codex, &session_level(high())?)?;
    // @step Then the request carries reasoning effort "high" with summary "auto"
    assert_eq!(
        config,
        json!({"reasoning": {"effort": "high", "summary": "auto"}})
    );
    Ok(())
}

#[test]
fn anthropic_efforts_become_adaptive_thinking_and_budgets_become_extended_thinking() -> TestResult {
    // @step Given anthropic/claude-opus-4-6 advertises efforts "low, medium, high, max" and a token budget of at least 1024
    let registry = catalog(&[(
        "anthropic",
        "claude-opus-4-6",
        json!([
            efforts(&["low", "medium", "high", "max"]),
            {"type": "budget_tokens", "min": 1024}
        ]),
    )])?;
    let wire = dispatch(ProviderType::Claude, None)?;
    assert_eq!(wire, ThinkingWire::Anthropic);
    let opus = target(
        wire,
        "anthropic",
        "claude-opus-4-6",
        Some(capability(&registry, "anthropic", "claude-opus-4-6")?),
    );
    // @step When the chosen level is the effort "max"
    let adaptive = sent(&opus, &effort("max")?)?;
    // @step Then the request carries adaptive thinking with summarized display and output effort "max"
    assert_eq!(
        adaptive,
        json!({
            "thinking": {"type": "adaptive", "display": "summarized"},
            "output_config": {"effort": "max"}
        })
    );
    // @step When the chosen level is a budget of 8000 tokens
    let extended = sent(&opus, &TurnLevel::Level(ReasoningLevel::Budget(8000)))?;
    // @step Then the request carries enabled thinking with a budget of 8000 tokens
    assert_eq!(
        extended,
        json!({"thinking": {"type": "enabled", "budget_tokens": 8000}})
    );
    Ok(())
}

#[test]
fn gemini_efforts_become_a_thinking_level_and_budgets_a_thinking_budget() -> TestResult {
    // @step Given google/gemini-3.1-pro-preview advertises efforts "low, medium, high" and google/gemini-2.5-pro a token budget from 128 to 32768
    let registry = catalog(&[
        (
            "google",
            "gemini-3.1-pro-preview",
            json!([efforts(&["low", "medium", "high"])]),
        ),
        (
            "google",
            "gemini-2.5-pro",
            json!([{"type": "budget_tokens", "min": 128, "max": 32768}]),
        ),
    ])?;
    let wire = dispatch(ProviderType::Gemini, None)?;
    assert_eq!(wire, ThinkingWire::Gemini);
    let three = target(
        wire,
        "google",
        "gemini-3.1-pro-preview",
        Some(capability(&registry, "google", "gemini-3.1-pro-preview")?),
    );
    let two_five = target(
        wire,
        "google",
        "gemini-2.5-pro",
        Some(capability(&registry, "google", "gemini-2.5-pro")?),
    );
    // @step When gemini-3.1-pro-preview is at the effort "low" and gemini-2.5-pro at a budget of 4096 tokens
    let level = sent(&three, &effort("low")?)?;
    let budget = sent(&two_five, &TurnLevel::Level(ReasoningLevel::Budget(4096)))?;
    // @step Then gemini-3.1-pro-preview's request carries thinkingLevel "low" with thoughts included
    assert_eq!(
        level,
        json!({"thinkingConfig": {"includeThoughts": true, "thinkingLevel": "low"}})
    );
    // @step And gemini-2.5-pro's request carries thinkingBudget 4096 with thoughts included
    assert_eq!(
        budget,
        json!({"thinkingConfig": {"includeThoughts": true, "thinkingBudget": 4096}})
    );
    Ok(())
}

#[test]
fn a_local_server_gets_its_engines_form_for_the_declared_levels() -> TestResult {
    // @step Given a model on a local Strata profile that declares efforts "none, low, medium, high" and no template toggle (MODEL-016)
    let wire = dispatch(ProviderType::OpenAI, Some("strata"))?;
    assert_eq!(wire, ThinkingWire::LocalOpenAi);
    let declaration: ThinkingLevelsDeclaration =
        serde_json::from_value(json!({"efforts": ["none", "low", "medium", "high"]}))?;
    let reasoning = declaration.resolve("strata", "qwen3.8-flash-next", LocalEngine::Strata)?;
    let strata = ThinkingTarget::local_profile(
        Ok(wire),
        "openai:strata/qwen3.8-flash-next",
        "qwen3.8-flash-next",
        Ok(Some(reasoning)),
    );
    // @step When the session's level is High
    let high = sent(&strata, &session_level(high())?)?;
    // @step Then the request carries reasoning_effort "high" and no chat_template_kwargs
    assert_eq!(high, json!({"reasoning_effort": "high"}));
    // @step When the session's level is Off
    let off = sent(&strata, &session_level(OFF)?)?;
    // @step Then the request carries reasoning_effort "none" and no chat_template_kwargs
    assert_eq!(off, json!({"reasoning_effort": "none"}));
    Ok(())
}

#[test]
fn an_effort_the_model_does_not_advertise_is_refused_not_remapped() -> TestResult {
    // @step Given anthropic/claude-opus-4-6 advertises efforts "low, medium, high, max"
    let registry = catalog(&[(
        "anthropic",
        "claude-opus-4-6",
        json!([efforts(&["low", "medium", "high", "max"])]),
    )])?;
    let opus = target(
        ThinkingWire::Anthropic,
        "anthropic",
        "claude-opus-4-6",
        Some(capability(&registry, "anthropic", "claude-opus-4-6")?),
    );
    // @step When the chosen level is the effort "xhigh"
    let reason = refused(&opus, &effort("xhigh")?)?;
    // @step Then the turn is refused with an error naming anthropic/claude-opus-4-6 and listing "low, medium, high, max"
    assert!(
        reason.contains("anthropic/claude-opus-4-6") && reason.contains("low, medium, high, max"),
        "{reason}"
    );
    Ok(())
}

#[test]
fn budget_only_and_toggle_only_models_get_no_invented_level() -> TestResult {
    // @step Given anthropic/claude-haiku-4-5 advertises only a token budget of at least 1024 and zai/glm-4.6 only a toggle
    let registry = catalog(&[
        (
            "anthropic",
            "claude-haiku-4-5",
            json!([{"type": "budget_tokens", "min": 1024}]),
        ),
        ("zai", "glm-4.6", json!([{"type": "toggle"}])),
    ])?;
    let haiku = target(
        ThinkingWire::Anthropic,
        "anthropic",
        "claude-haiku-4-5",
        Some(capability(&registry, "anthropic", "claude-haiku-4-5")?),
    );
    let wire = dispatch(ProviderType::ZAI, None)?;
    assert_eq!(wire, ThinkingWire::Zai);
    let glm = target(
        wire,
        "zai",
        "glm-4.6",
        Some(capability(&registry, "zai", "glm-4.6")?),
    );
    // @step When the session's level is High
    let high = session_level(high())?;
    // @step Then the turn on claude-haiku-4-5 is refused with an error naming its token budget
    let reason = refused(&haiku, &high)?;
    assert!(
        reason.contains("anthropic/claude-haiku-4-5")
            && reason.contains("token budget of at least 1024 tokens"),
        "{reason}"
    );
    // @step And the turn on glm-4.6 is refused
    let reason = refused(&glm, &high)?;
    assert!(reason.contains("zai/glm-4.6"), "{reason}");
    // @step When the session's level is Off
    let off = sent(&glm, &session_level(OFF)?)?;
    // @step Then glm-4.6's request carries thinking type "disabled"
    assert_eq!(off, json!({"thinking": {"type": "disabled"}}));
    Ok(())
}

#[test]
fn off_is_the_models_own_none_effort_or_is_refused() -> TestResult {
    // @step Given openai/gpt-5.5 advertises efforts "none, low, medium, high, xhigh" and openai/gpt-5 efforts "minimal, low, medium, high"
    let registry = catalog(&[
        (
            "openai",
            "gpt-5.5",
            json!([efforts(&["none", "low", "medium", "high", "xhigh"])]),
        ),
        (
            "openai",
            "gpt-5",
            json!([efforts(&["minimal", "low", "medium", "high"])]),
        ),
    ])?;
    let five_five = target(
        ThinkingWire::OpenAiChat,
        "openai",
        "gpt-5.5",
        Some(capability(&registry, "openai", "gpt-5.5")?),
    );
    let five = target(
        ThinkingWire::OpenAiChat,
        "openai",
        "gpt-5",
        Some(capability(&registry, "openai", "gpt-5")?),
    );
    // @step When the session's level is Off
    let off = session_level(OFF)?;
    // @step Then gpt-5.5's request carries reasoning_effort "none"
    assert_eq!(sent(&five_five, &off)?, json!({"reasoning_effort": "none"}));
    // @step And the turn on gpt-5 is refused saying thinking cannot be turned off on openai/gpt-5
    let reason = refused(&five, &off)?;
    assert!(
        reason.contains("thinking cannot be turned off on openai/gpt-5"),
        "{reason}"
    );
    Ok(())
}

#[test]
fn a_level_no_wire_form_can_carry_is_refused_naming_the_provider() -> TestResult {
    // @step Given zai/glm-5.3 advertises efforts "low, high, max" and a custom provider uses the unknown facade "mystery"
    let registry = catalog(&[("zai", "glm-5.3", json!([efforts(&["low", "high", "max"])]))])?;
    let glm = target(
        ThinkingWire::Zai,
        "zai",
        "glm-5.3",
        Some(capability(&registry, "zai", "glm-5.3")?),
    );
    let mystery = ThinkingTarget::new(
        ThinkingWire::for_dispatch(
            &ProviderType::Custom("acme".to_string()),
            Some("mystery"),
            None,
        ),
        "acme/acme-large",
        "acme-large",
        Ok(None),
    );
    // @step When the session's level is High
    let high = session_level(high())?;
    // @step Then the turn on glm-5.3 is refused saying no effort form is known for zai
    let reason = refused(&glm, &high)?;
    assert!(
        reason.contains("no effort form is known for zai"),
        "{reason}"
    );
    // @step And the turn on the custom provider is refused naming the facade "mystery"
    let reason = refused(&mystery, &high)?;
    assert!(reason.contains("mystery"), "{reason}");
    assert_eq!(mystery.config_for(None)?, None);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_per_turn_raw_thinking_config_is_refused_not_sent() -> TestResult {
    let _guard = support::DATA_DIR_GUARD.lock().await;
    // @step Given a session at High on openai/gpt-5.5
    let (_data_dir, manager, session) = support::fresh_background_session().await;
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    handle.set_thinking_level(
        &SessionId::new(session.id.to_string()),
        ThinkingLevel::Effort("high".to_string()),
    )?;
    let gpt = advertised_gpt()?;
    assert_eq!(
        resolve_turn_thinking(&session, None, &gpt)?,
        Some(json!({"reasoning_effort": "high"}))
    );
    // @step When a turn arrives carrying a per-turn thinking config
    let raw = r#"{"provider_id":"openai","level":"high","config_json":"{}"}"#;
    let outcome = resolve_turn_thinking(&session, Some(raw), &gpt);
    // @step Then the turn is refused saying levels are chosen with the thinking dialog or /thinking
    let reason = match outcome {
        Ok(config) => return Err(format!("expected a refusal, got {config:?}").into()),
        Err(reason) => reason,
    };
    assert!(
        reason.contains("thinking dialog") && reason.contains("/thinking"),
        "{reason}"
    );
    Ok(())
}

/// MODEL-017: a turn target for openai/gpt-5.5 advertising low, medium and
/// high (a model with no advertised levels has no reasoning control).
fn advertised_gpt() -> Result<ThinkingTarget, TestError> {
    let registry = catalog(&[(
        "openai",
        "gpt-5.5",
        json!([efforts(&["low", "medium", "high"])]),
    )])?;
    Ok(target(
        ThinkingWire::OpenAiChat,
        "openai",
        "gpt-5.5",
        Some(capability(&registry, "openai", "gpt-5.5")?),
    ))
}
