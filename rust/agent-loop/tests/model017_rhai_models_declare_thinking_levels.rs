//! MODEL-017: Rhai custom-provider models declare their thinking levels.
//!
//! Feature: spec/features/rhai-custom-provider-models-declare-thinking-levels.feature
//!
//! Each test writes custom provider configs into a throwaway FSPEC_HOME
//! providers directory, selects the model through the resolver every
//! session path uses (`apply_model_selection`) and reads the thinking
//! target the agent loop builds from that ProviderManager. The dialog and
//! /thinking options come from `codelet_sessions::thinking_options`, and a
//! turn's parameters come from `ThinkingTarget::config_for`. FSPEC_HOME is
//! process-global, so `GUARD` serialises the tests.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use codelet_agent_loop::thinking_config::{ThinkingTarget, TurnLevel};
use codelet_common::Message;
use codelet_providers::custom::{ProviderConfig, RhaiCustomProvider, ScriptLoader};
use codelet_providers::models::ReasoningLevel;
use codelet_providers::oauth::engine::build_provider_engine;
use codelet_providers::{ProviderManager, ProviderType};
use codelet_rpc_types::{ThinkingChoice, ThinkingLevel, ThinkingOptions};
use codelet_sessions::model_resolution::apply_model_selection;
use codelet_sessions::thinking_options::{accepted, options_for};
use codelet_test_helpers::{OrFail, TestError, TestResult};
use serde_json::{json, Value};
use tokio::sync::{Mutex, MutexGuard};

static GUARD: Mutex<()> = Mutex::const_new(());

/// A FSPEC_HOME whose providers directory holds the test's configs.
struct Providers {
    dir: tempfile::TempDir,
    previous: Option<String>,
    _guard: MutexGuard<'static, ()>,
}

impl Drop for Providers {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var("FSPEC_HOME", value),
            None => std::env::remove_var("FSPEC_HOME"),
        }
    }
}

impl Providers {
    async fn new() -> Result<Self, TestError> {
        let guard = GUARD.lock().await;
        let dir = tempfile::tempdir()?;
        std::fs::create_dir_all(dir.path().join("credentials"))?;
        std::fs::create_dir_all(dir.path().join("providers"))?;
        let previous = std::env::var("FSPEC_HOME").ok();
        std::env::set_var("FSPEC_HOME", dir.path().join("credentials"));
        Ok(Self {
            dir,
            previous,
            _guard: guard,
        })
    }

    fn providers(&self) -> PathBuf {
        self.dir.path().join("providers")
    }

    /// The Rhai provider `name` whose models are `(alias, thinkingLevels)`.
    fn rhai(&self, name: &str, models: &[(&str, Option<Value>)]) -> Result<(), TestError> {
        std::fs::write(
            self.providers().join(format!("{name}.rhai")),
            "fn build_request(request) { request }\n",
        )?;
        let mut map = serde_json::Map::new();
        for (alias, declaration) in models {
            let mut model = json!({"id": format!("{alias}-api"), "supports_thinking": true});
            if let Some(declaration) = declaration {
                model["thinkingLevels"] = declaration.clone();
            }
            map.insert((*alias).to_string(), model);
        }
        let config = json!({
            "name": name,
            "display_name": name,
            "base_url": "http://127.0.0.1:9",
            "script": format!("{name}.rhai"),
            "models": map,
        });
        std::fs::write(
            self.providers().join(format!("{name}.json")),
            config.to_string(),
        )?;
        Ok(())
    }

    /// The bundled codex.rhai template, installed under `name`.
    fn bundled_codex(&self, name: &str) -> Result<PathBuf, TestError> {
        let mut config: Value =
            serde_json::from_str(&std::fs::read_to_string(bundle().join("codex.json"))?)?;
        config["name"] = json!(name);
        config["script"] = json!(bundle().join("codex.rhai").to_string_lossy());
        let path = self.providers().join(format!("{name}.json"));
        std::fs::write(&path, config.to_string())?;
        Ok(path)
    }
}

fn bundle() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../providers/templates/codex")
}

/// The turn target of `provider/model`, selected as every session does.
fn target(model: &str) -> Result<ThinkingTarget, TestError> {
    let (provider, _) = model.split_once('/').or_fail("provider/model")?;
    let mut manager =
        ProviderManager::for_testing(ProviderType::Custom(provider.into()), None, None);
    apply_model_selection(&mut manager, model)?;
    Ok(ThinkingTarget::for_provider_manager(&manager))
}

/// The offered levels as the scenarios name them.
fn offered(target: &ThinkingTarget) -> Result<Vec<String>, TestError> {
    match options_for(target) {
        ThinkingOptions::Levels { choices, .. } => Ok(choices
            .into_iter()
            .map(|choice| match choice {
                ThinkingChoice::Off => "Off".to_string(),
                ThinkingChoice::Effort(name) => name,
                ThinkingChoice::NumericEffort { kind, min, max } => {
                    format!("{kind:?} {min} to {max}")
                }
                ThinkingChoice::TokenBudget { min, max } => {
                    format!("budget {min:?} to {max:?}")
                }
                ThinkingChoice::On => "On".to_string(),
            })
            .collect()),
        other => Err(format!("expected levels, got {other:?}").into()),
    }
}

fn no_control(target: &ThinkingTarget) -> Result<String, TestError> {
    match options_for(target) {
        ThinkingOptions::NoReasoningControl { reason, .. } => Ok(reason),
        other => Err(format!("expected no reasoning control, got {other:?}").into()),
    }
}

fn effort(name: &str) -> Result<TurnLevel, TestError> {
    Ok(TurnLevel::Level(ReasoningLevel::effort(name)?))
}

fn sent(target: &ThinkingTarget, level: &TurnLevel) -> Result<Value, TestError> {
    Ok(target
        .config_for(Some(level))?
        .or_fail("the level sends thinking parameters")?)
}

fn refused(target: &ThinkingTarget, level: &TurnLevel) -> Result<String, TestError> {
    match target.config_for(Some(level)) {
        Ok(config) => Err(format!("expected a refusal, got {config:?}").into()),
        Err(reason) => Ok(reason),
    }
}

fn efforts(values: &[&str]) -> Option<Value> {
    Some(json!({"efforts": values}))
}

fn lmh() -> Option<Value> {
    efforts(&["low", "medium", "high"])
}

/// Scenario: A Rhai model offers the thinking levels it declares
#[tokio::test]
async fn a_rhai_model_offers_the_levels_it_declares() -> TestResult {
    // @step Given the Rhai provider acme's model coder declares efforts "low, medium, high"
    let providers = Providers::new().await?;
    providers.rhai("acme", &[("coder", lmh())])?;
    // @step When the thinking options for acme/coder are read
    let coder = target("acme/coder")?;
    // @step Then they are low, medium, high in that order
    assert_eq!(offered(&coder)?, ["low", "medium", "high"]);
    // @step And at medium the script's thinking_config is level "medium" with capability efforts "low, medium, high"
    let config = sent(&coder, &effort("medium")?)?;
    assert_eq!(config["level"], "medium");
    assert_eq!(
        config["capability"]["efforts"],
        json!(["low", "medium", "high"])
    );
    Ok(())
}

/// Scenario: A level the declaration does not list is refused
#[tokio::test]
async fn a_level_the_declaration_does_not_list_is_refused() -> TestResult {
    // @step Given the Rhai provider acme's model coder declares efforts "low, medium, high"
    let providers = Providers::new().await?;
    providers.rhai("acme", &[("coder", lmh())])?;
    let coder = target("acme/coder")?;
    // @step When the level xhigh is chosen for acme/coder
    let chosen = accepted(&coder, &ThinkingLevel::Effort("xhigh".into()));
    // @step Then it is refused naming acme/coder and listing low, medium, high
    let reason = chosen.err().or_fail("xhigh is refused")?;
    assert!(
        reason.contains("acme/coder") && reason.contains("low, medium, high"),
        "{reason}"
    );
    let turn = refused(&coder, &effort("xhigh")?)?;
    assert!(turn.contains("acme/coder"), "{turn}");
    Ok(())
}

/// Scenario: A model with no declaration has no reasoning control and a held level is refused
#[tokio::test]
async fn an_undeclared_model_has_no_control_and_a_held_level_is_refused() -> TestResult {
    // @step Given the Rhai provider acme's model plain declares no thinking levels
    let providers = Providers::new().await?;
    providers.rhai("acme", &[("plain", None)])?;
    // @step When the thinking options for acme/plain are read
    let plain = target("acme/plain")?;
    // @step Then acme/plain has no reasoning control and the reason says to declare thinkingLevels on its model in the acme provider config
    let reason = no_control(&plain)?;
    assert!(
        reason.contains("acme/plain")
            && reason.contains("thinkingLevels")
            && reason.contains("acme provider config"),
        "{reason}"
    );
    // @step And a turn at high on acme/plain is refused instead of being sent unchecked
    let turn = refused(&plain, &effort("high")?)?;
    assert!(
        turn.contains("acme/plain") && turn.contains("no reasoning control"),
        "{turn}"
    );
    Ok(())
}

/// Scenario: A declaration takes numeric efforts, a token budget and a template toggle
#[tokio::test]
async fn a_declaration_takes_numeric_efforts_a_budget_and_a_toggle() -> TestResult {
    // @step Given the Rhai provider acme's model wide declares whole-number efforts 1 to 100, a token budget 1024 to 32768 and templateToggle enable_thinking
    let providers = Providers::new().await?;
    providers.rhai(
        "acme",
        &[(
            "wide",
            Some(json!({
                "numericEfforts": {"kind": "whole", "min": 1, "max": 100},
                "tokenBudget": {"min": 1024, "max": 32768},
                "templateToggle": "enable_thinking"
            })),
        )],
    )?;
    // @step When the thinking options for acme/wide are read
    let wide = target("acme/wide")?;
    // @step Then they are Off, the whole numbers 1 to 100 and the token budget 1024 to 32768, with no On since the model declares more than a toggle
    assert_eq!(
        offered(&wide)?,
        ["Off", "Whole 1 to 100", "budget Some(1024) to Some(32768)"]
    );
    // @step And at 40 the script receives level 40 and at a budget of 4096 level 4096, with the numeric range, the budget and the template toggle in its capability
    let numeric = sent(&wide, &TurnLevel::Level(ReasoningLevel::Numeric(40.into())))?;
    assert_eq!(numeric["level"], 40);
    let budget = sent(&wide, &TurnLevel::Level(ReasoningLevel::Budget(4096)))?;
    assert_eq!(budget["level"], 4096);
    let capability = &budget["capability"];
    assert_eq!(
        capability["numeric"],
        json!({"kind": "whole", "min": 1, "max": 100})
    );
    assert_eq!(capability["budget"], json!({"min": 1024, "max": 32768}));
    assert_eq!(capability["templateToggle"], "enable_thinking");
    Ok(())
}

/// Scenario: A declaration fspec cannot use fails only that model
#[tokio::test]
async fn an_unusable_declaration_fails_only_that_model() -> TestResult {
    // @step Given the Rhai provider acme's model broken declares efforts "Extra High" and its model coder declares efforts "low, medium, high"
    let providers = Providers::new().await?;
    providers.rhai(
        "acme",
        &[("broken", efforts(&["Extra High"])), ("coder", lmh())],
    )?;
    // @step When the thinking options are read for acme/broken and acme/coder
    let broken = target("acme/broken")?;
    let coder = target("acme/coder")?;
    // @step Then acme/broken has no reasoning control with a reason naming acme, broken and Extra High
    let reason = no_control(&broken)?;
    assert!(
        reason.contains("acme") && reason.contains("broken") && reason.contains("Extra High"),
        "{reason}"
    );
    // @step And acme/coder still offers low, medium, high
    assert_eq!(offered(&coder)?, ["low", "medium", "high"]);
    Ok(())
}

/// Scenario: Off needs a declared off form
#[tokio::test]
async fn off_needs_a_declared_off_form() -> TestResult {
    // @step Given the Rhai provider acme's model coder declares efforts "low, medium, high" and its model lean declares efforts "none, low, high"
    let providers = Providers::new().await?;
    providers.rhai(
        "acme",
        &[
            ("coder", lmh()),
            ("lean", efforts(&["none", "low", "high"])),
        ],
    )?;
    let coder = target("acme/coder")?;
    let lean = target("acme/lean")?;
    // @step When Off is chosen
    let coder_off = refused(&coder, &TurnLevel::Off);
    // @step Then it is refused for acme/coder
    let reason = coder_off?;
    assert!(reason.contains("acme/coder"), "{reason}");
    assert!(accepted(&coder, &ThinkingLevel::Off).is_err());
    // @step And acme/lean offers none, low, high and its Off sends level "off" to the script
    assert_eq!(offered(&lean)?, ["none", "low", "high"]);
    assert_eq!(sent(&lean, &TurnLevel::Off)?["level"], "off");
    Ok(())
}

/// Scenario: The bundled codex.rhai template declares Codex's levels
#[tokio::test]
async fn the_bundled_codex_template_declares_codex_levels() -> TestResult {
    // @step Given the bundled codex.rhai template installed as the custom provider codex-script
    let providers = Providers::new().await?;
    let installed = providers.bundled_codex("codex-script")?;
    // @step When the thinking options for its default model are read
    let codex = target("codex-script/default")?;
    // @step Then they are low, medium, high, xhigh in that order
    assert_eq!(offered(&codex)?, ["low", "medium", "high", "xhigh"]);
    // @step And at xhigh the template sends reasoning effort "xhigh" with summary "auto"
    let config = sent(&codex, &effort("xhigh")?)?;
    let provider = template_provider(&installed)?;
    let body = provider
        .invoke_build_stream_request(
            &[Message::system("Instructions"), Message::user("Hello")],
            &[],
            Some(config),
        )
        .await?;
    assert_eq!(
        body["reasoning"],
        json!({"effort": "xhigh", "summary": "auto"})
    );
    // @step And none is refused
    assert!(accepted(&codex, &ThinkingLevel::Effort("none".into())).is_err());
    Ok(())
}

fn template_provider(config: &Path) -> Result<RhaiCustomProvider, TestError> {
    let mut provider_config = ProviderConfig::from_file(config)?;
    provider_config.script = bundle().join("codex.rhai").to_string_lossy().into_owned();
    Ok(RhaiCustomProvider::new(
        Arc::new(provider_config),
        Arc::new(ScriptLoader::new(build_provider_engine("codex"))),
        "default".into(),
    )?)
}

/// Scenario: A cloud model missing from models.dev has no reasoning control and a held level is refused
#[test]
fn a_cloud_model_missing_from_models_dev_has_no_control_and_a_held_level_is_refused() -> TestResult
{
    // @step Given the cloud model openai/gpt-unlisted is missing from models.dev
    // (the target ThinkingTarget::for_provider_manager builds for a catalog miss)
    let wire = codelet_providers::thinking_wire::ThinkingWire::for_dispatch(
        &ProviderType::OpenAI,
        None,
        None,
    );
    // @step When the thinking options for openai/gpt-unlisted are read
    let unlisted = ThinkingTarget::new(wire, "openai/gpt-unlisted", "gpt-unlisted", Ok(None));
    // @step Then openai/gpt-unlisted has no reasoning control
    let reason = no_control(&unlisted)?;
    assert!(
        reason.contains("openai/gpt-unlisted") && reason.contains("no reasoning control"),
        "{reason}"
    );
    // @step And a turn at high on openai/gpt-unlisted is refused naming the model instead of being sent unchecked
    let turn = refused(&unlisted, &effort("high")?)?;
    assert!(
        turn.contains("openai/gpt-unlisted") && turn.contains("no reasoning control"),
        "{turn}"
    );
    let off = refused(&unlisted, &TurnLevel::Off)?;
    assert!(off.contains("openai/gpt-unlisted"), "{off}");
    Ok(())
}
