//! Shared setup for the MODEL-016 tests
//! (`model016_local_inference_server_thinking_levels.rs`): a hermetic
//! `fspec-config.json` holding local-server profiles, the profile model
//! selected through the real resolver, and the turn target the agent loop
//! builds from it. A sub-directory module under `tests/` is not its own test
//! binary, so the one-feature-one-test-file mapping holds.

use codelet_agent_loop::thinking_config::{ThinkingTarget, TurnLevel};
use codelet_providers::local_reasoning::LocalReasoning;
use codelet_providers::models::{ReasoningCapability, ReasoningLevel};
use codelet_providers::ProviderManager;
use codelet_sessions::model_resolution::apply_model_selection;
use codelet_sessions::profile_sections::{load_local_server_profiles, LocalServerProfile};
use codelet_test_helpers::{OrFail, TestError};
use serde_json::{json, Value};
use tempfile::TempDir;

/// An endpoint nothing listens on, for profiles whose server is never
/// reached.
pub const UNREACHABLE: &str = "http://127.0.0.1:9/v1";

const ENV: [&str; 6] = [
    "FSPEC_USER_DIR",
    "OPENAI_API_KEY",
    "OPENAI_BASE_URL",
    "OPENAI_CONTEXT_WINDOW",
    "OPENAI_STREAMING",
    "OPENAI_MODEL",
];

/// A user directory holding `config`, restoring the process environment
/// the profile resolver writes when dropped.
pub struct Profiles {
    _dir: TempDir,
    saved: Vec<(&'static str, Option<String>)>,
}

impl Drop for Profiles {
    fn drop(&mut self) {
        for (key, value) in &self.saved {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

/// Store `profiles` (`name`, profile object) as the user's local-server
/// profiles.
pub fn store(profiles: &[(&str, Value)]) -> Result<Profiles, TestError> {
    let saved = ENV
        .iter()
        .map(|key| (*key, std::env::var(key).ok()))
        .collect();
    let dir = codelet_test_helpers::setup_temp_directory()?;
    let mut map = serde_json::Map::new();
    for (name, profile) in profiles {
        map.insert((*name).to_string(), profile.clone());
    }
    let config = json!({"providers": {"openai": {"profiles": Value::Object(map)}}});
    std::fs::write(
        dir.path().join("fspec-config.json"),
        serde_json::to_vec(&config)?,
    )?;
    std::env::set_var("FSPEC_USER_DIR", dir.path());
    Ok(Profiles { _dir: dir, saved })
}

/// A profile object: its engine (none when `None`), a profile-wide
/// declaration and its customModels with their own declarations.
pub fn profile(
    base_url: &str,
    engine: Option<&str>,
    profile_wide: Option<Value>,
    models: &[(&str, Option<Value>)],
) -> Value {
    let mut profile = json!({
        "baseUrl": base_url,
        "apiKey": "test",
        "contextWindow": 32768,
        "customModels": models
            .iter()
            .map(|(id, declaration)| match declaration {
                Some(declaration) => json!({"id": id, "thinkingLevels": declaration}),
                None => json!({"id": id}),
            })
            .collect::<Vec<_>>(),
    });
    if let Some(engine) = engine {
        profile["engine"] = json!(engine);
    }
    if let Some(declaration) = profile_wide {
        profile["thinkingLevels"] = declaration;
    }
    profile
}

/// A profile on an unreachable server with one model and its declaration.
pub fn one_model(engine: &str, model: &str, declaration: Option<Value>) -> Value {
    profile(UNREACHABLE, Some(engine), None, &[(model, declaration)])
}

pub fn efforts(values: &[&str]) -> Value {
    json!({"efforts": values})
}

pub fn whole(min: u64, max: u64) -> Value {
    json!({"kind": "whole", "min": min, "max": max})
}

pub fn decimal(min: f64, max: f64) -> Value {
    json!({"kind": "decimal", "min": min, "max": max})
}

/// The profile model `profile/model`, selected through the resolver every
/// session path uses.
pub fn select(profile: &str, model: &str) -> Result<ProviderManager, TestError> {
    std::env::set_var("OPENAI_API_KEY", "test");
    let mut manager = ProviderManager::with_provider_and_model("openai", Some(model), None, None)?;
    apply_model_selection(&mut manager, &format!("openai:{profile}/{model}"))?;
    Ok(manager)
}

/// The thinking target a turn on the selected profile model is built for.
pub fn target(profile: &str, model: &str) -> Result<ThinkingTarget, TestError> {
    Ok(ThinkingTarget::for_provider_manager(&select(
        profile, model,
    )?))
}

/// The selected profile model's declared reasoning.
pub fn declared(manager: &ProviderManager) -> Result<LocalReasoning, TestError> {
    Ok(manager
        .local_reasoning()
        .clone()?
        .or_fail("the model has a declared reasoning capability")?)
}

/// The levels offered for a capability, joined as the scenarios list them.
pub fn offered(capability: &ReasoningCapability) -> String {
    capability.offered_levels().join(", ")
}

/// The stored profile `name`.
pub fn loaded(name: &str) -> Result<LocalServerProfile, TestError> {
    Ok(load_local_server_profiles()?
        .into_iter()
        .find(|profile| profile.name == name)
        .or_fail("the profile is loaded")?)
}

pub fn effort(name: &str) -> Result<TurnLevel, TestError> {
    Ok(TurnLevel::Level(ReasoningLevel::effort(name)?))
}

pub fn numeric(value: serde_json::Number) -> TurnLevel {
    TurnLevel::Level(ReasoningLevel::Numeric(value))
}

/// The thinking parameters a turn at `level` sends to `target`.
pub fn sent(target: &ThinkingTarget, level: &TurnLevel) -> Result<Value, TestError> {
    Ok(target
        .config_for(Some(level))?
        .or_fail("the turn sends a thinking parameter")?)
}

/// The error a turn at `level` on `target` is refused with.
pub fn refused(target: &ThinkingTarget, level: &TurnLevel) -> Result<String, TestError> {
    match target.config_for(Some(level)) {
        Ok(config) => Err(format!("expected a refusal, got {config:?}").into()),
        Err(reason) => Ok(reason),
    }
}
