//! The thinking wire table (MODEL-010): a turn's thinking level in the form
//! the dispatched provider takes.
//!
//! The form is chosen by the provider that serves the request
//! ([`ThinkingWire::for_dispatch`]), never by the model id. A level is
//! checked against the model's advertised [`ReasoningCapability`] (MODEL-009)
//! and then sent unchanged, or the turn is refused: nothing is remapped and no
//! form silently sends nothing. MODEL-017: a catalog model with no advertised
//! capability (one missing from models.dev) has no reasoning control and any
//! level is refused, where MODEL-010 sent it unchecked; a custom provider
//! model's capability is what its config entry declares, and one that
//! declares nothing has no reasoning control either. MODEL-016: a local profile model's
//! capability is what its profile declares, sent in its engine's form; a
//! profile model that declares nothing has no reasoning control.
//!
//! Feature: spec/features/thinking-levels-reach-each-provider-through-a-wire-table.feature
//! Feature: spec/features/local-inference-server-thinking-levels.feature
//! Feature: spec/features/rhai-custom-provider-models-declare-thinking-levels.feature

mod forms;
mod local;

use crate::local_reasoning::{LocalEngine, LocalReasoning};
use crate::models::{ReasoningCapability, ReasoningLevel};
use crate::{ProviderManager, ProviderType};
use serde_json::Value;

/// The wire form a provider takes for a thinking level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkingWire {
    /// OpenAI Chat Completions (OpenAI, GitHub Copilot): `reasoning_effort`.
    OpenAiChat,
    /// An OpenAI-compatible local server profile: the form its engine takes
    /// for what the model declares (MODEL-016).
    LocalOpenAi,
    /// OpenAI Responses (Codex): `reasoning {effort, summary}`.
    OpenAiResponses,
    /// Anthropic Messages: adaptive or enabled `thinking`, Claude Off forms.
    Anthropic,
    /// Gemini `thinkingConfig`: `thinkingLevel` or `thinkingBudget`.
    Gemini,
    /// Z.AI: only the toggle's off form, `thinking {type: disabled}`.
    Zai,
    /// A Rhai script: `{level, capability}`; the script owns the wire.
    Rhai,
}

impl ThinkingWire {
    /// The wire of the dispatched provider: its facade when it has one, its
    /// profile (local server) or its provider type otherwise.
    pub fn for_dispatch(
        provider: &ProviderType,
        facade: Option<&str>,
        profile: Option<&str>,
    ) -> Result<Self, String> {
        if let Some(facade) = facade {
            return match facade {
                "claude" => Ok(Self::Anthropic),
                "openai" if profile.is_some() => Ok(Self::LocalOpenAi),
                "openai" | "github-copilot" | "copilot" => Ok(Self::OpenAiChat),
                "codex" => Ok(Self::OpenAiResponses),
                "gemini" => Ok(Self::Gemini),
                "zai" => Ok(Self::Zai),
                other => Err(format!(
                    "no thinking form is known for the provider facade {other}; \
                     choose the model default thinking level"
                )),
            };
        }
        Ok(match provider {
            ProviderType::Claude => Self::Anthropic,
            ProviderType::OpenAI if profile.is_some() => Self::LocalOpenAi,
            ProviderType::OpenAI | ProviderType::GitHubCopilot => Self::OpenAiChat,
            ProviderType::Codex => Self::OpenAiResponses,
            ProviderType::Gemini => Self::Gemini,
            ProviderType::ZAI => Self::Zai,
            ProviderType::Custom(_) => Self::Rhai,
        })
    }
}

/// A turn's thinking level: Off, or a level the model may advertise (a
/// named effort or a token budget).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnLevel {
    Off,
    Level(ReasoningLevel),
}

/// Where a target's capability comes from.
#[derive(Debug, Clone)]
enum CapabilitySource {
    /// Advertised by the catalog; none advertised means no reasoning control.
    Advertised,
    /// MODEL-016: declared in a local-server profile, with the profile's
    /// engine; none declared means no reasoning control.
    Declared(Option<LocalEngine>),
    /// MODEL-017: declared on the model's entry in the named custom
    /// provider's config; none declared means no reasoning control.
    ProviderConfig(String),
}

/// MODEL-017: why a custom provider model that declares nothing has no
/// reasoning control.
fn undeclared_custom_model(model: &str, provider: &str) -> String {
    format!(
        "{model} has no reasoning control: it declares no thinking levels; declare \
         thinkingLevels on its model in the {provider} provider config"
    )
}

/// MODEL-017: why a catalog model that advertises no thinking levels (one
/// missing from models.dev) has no reasoning control.
fn unadvertised_model(model: &str) -> String {
    format!("{model} has no reasoning control: it advertises no thinking levels")
}

/// What a turn's thinking parameters are built for: the wire, the model and
/// what the model advertises or declares.
#[derive(Debug, Clone)]
pub struct ThinkingTarget {
    wire: Result<ThinkingWire, String>,
    /// `provider/model`, as errors name it.
    model: String,
    /// The bare API model id (the Claude Off forms are per model).
    api_model: String,
    /// `Ok(None)`: no advertised or declared capability, so no reasoning
    /// control (MODEL-017).
    capability: Result<Option<ReasoningCapability>, String>,
    source: CapabilitySource,
}

impl ThinkingTarget {
    pub fn new(
        wire: Result<ThinkingWire, String>,
        model: &str,
        api_model: &str,
        capability: Result<Option<ReasoningCapability>, String>,
    ) -> Self {
        Self {
            wire,
            model: model.to_string(),
            api_model: api_model.to_string(),
            capability,
            source: CapabilitySource::Advertised,
        }
    }

    /// MODEL-016: the target of a local-server profile model, whose
    /// capability is what its profile declares (`Ok(None)`: nothing, so no
    /// level can be chosen for it).
    pub fn local_profile(
        wire: Result<ThinkingWire, String>,
        model: &str,
        api_model: &str,
        declared: Result<Option<LocalReasoning>, String>,
    ) -> Self {
        let engine = match &declared {
            Ok(Some(reasoning)) => Some(reasoning.engine),
            _ => None,
        };
        Self {
            wire,
            model: model.to_string(),
            api_model: api_model.to_string(),
            capability: declared.map(|declared| declared.map(|reasoning| reasoning.capability)),
            source: CapabilitySource::Declared(engine),
        }
    }

    /// The target of the provider manager's selected model. A local profile
    /// model's capability is its declaration (MODEL-016); a Codex model's is
    /// what the Codex /models response fetched for the selection advertises
    /// (MODEL-014); a custom provider model's is what its config entry
    /// declares (MODEL-017); any other catalog model's capability comes
    /// from its models.dev `reasoning_options`.
    pub fn for_provider_manager(manager: &ProviderManager) -> Self {
        let provider = manager.current_provider_type();
        let profile = manager.selected_profile_name();
        let model = manager
            .selected_model_string()
            .unwrap_or_else(|| manager.current_provider_name().to_string());
        let api_model = manager.selected_model_id().unwrap_or_default();
        let wire = ThinkingWire::for_dispatch(provider, manager.facade_override(), profile);
        // Local-server profiles are OpenAI-compatible; a custom provider's
        // profile is a custom provider model (MODEL-017).
        if profile.is_some() && matches!(provider, ProviderType::OpenAI) {
            return Self::local_profile(
                wire,
                &model,
                &api_model,
                manager.local_reasoning().clone(),
            );
        }
        // MODEL-014: a Codex model's levels come only from the Codex /models
        // response fetched for this selection, never from models.dev.
        if matches!(provider, ProviderType::Codex) {
            let capability = match manager.codex_catalog() {
                None => Err("Codex /models was not fetched for this model selection".to_string()),
                Some(Err(reason)) => Err(reason.clone()),
                Some(Ok(catalog)) => catalog.capability(&api_model).map(Some),
            };
            return Self::new(wire, &model, &api_model, capability);
        }
        // MODEL-017: a custom provider model's levels are what its config
        // entry declares; declaring none means no reasoning control.
        if let ProviderType::Custom(name) = provider {
            return Self {
                wire,
                model,
                api_model,
                capability: manager.custom_model_reasoning(),
                source: CapabilitySource::ProviderConfig(name.clone()),
            };
        }
        let capability = match manager.selected_model_info() {
            Some(info) => ReasoningCapability::resolve(&model, info.reasoning_options.as_deref())
                .map_err(|error| error.to_string()),
            _ => Ok(None),
        };
        Self::new(wire, &model, &api_model, capability)
    }

    pub fn wire(&self) -> Result<ThinkingWire, String> {
        self.wire.clone()
    }

    /// MODEL-015: whether a level change on this target can be an effort
    /// message: an Anthropic-wire model in the explicit per-message table.
    pub fn takes_per_message_effort(&self) -> bool {
        self.wire == Ok(ThinkingWire::Anthropic)
            && crate::claude_effort::takes_per_message_effort(&self.api_model)
    }

    /// The thinking parameters a turn at `level` sends; `None` when nothing
    /// is sent (no level chosen, or Off on a model whose thinking is off by
    /// default). An error refuses the turn.
    pub fn config_for(&self, level: Option<&TurnLevel>) -> Result<Option<Value>, String> {
        let Some(level) = level else {
            return Ok(None);
        };
        let wire = self.wire.clone()?;
        let capability = self.capability.clone()?;
        if let CapabilitySource::ProviderConfig(provider) = &self.source {
            let Some(capability) = capability else {
                return Err(undeclared_custom_model(&self.model, provider));
            };
            return self.checked(wire, level, Some(&capability));
        }
        if let CapabilitySource::Declared(engine) = &self.source {
            let engine = *engine;
            let Some(capability) = capability else {
                return Err(format!(
                    "{} declares no thinking levels, so it has no reasoning control: declare its \
                     levels in the profile (thinkingLevels on the profile or on its customModels \
                     entry, with the profile's engine)",
                    self.model
                ));
            };
            if wire == ThinkingWire::LocalOpenAi {
                return self.local(engine, &capability, level).map(Some);
            }
            return self.checked(wire, level, Some(&capability));
        }
        if wire == ThinkingWire::LocalOpenAi {
            return Err(format!(
                "{} is not a local profile model with declared thinking levels; declare its \
                 levels in the profile",
                self.model
            ));
        }
        // MODEL-017: a catalog model that advertises no levels (one missing
        // from models.dev) has no reasoning control, so a level a session
        // already holds is refused, never sent unchecked (product owner,
        // 2026-11-05, MODEL-011; supervisor correction, 2026-11-06).
        let Some(capability) = capability else {
            return Err(unadvertised_model(&self.model));
        };
        self.checked(wire, level, Some(&capability))
    }

    /// MODEL-016: a level in the local engine's form.
    fn local(
        &self,
        engine: Option<LocalEngine>,
        capability: &ReasoningCapability,
        level: &TurnLevel,
    ) -> Result<Value, String> {
        let Some(engine) = engine else {
            return Err(format!("no engine is known for {}", self.model));
        };
        match level {
            TurnLevel::Off => local::off(engine, capability, &self.model),
            TurnLevel::Level(level) => {
                capability
                    .check(&self.model, level)
                    .map_err(|error| error.to_string())?;
                local::level(engine, capability, level, &self.model)
            }
        }
    }

    /// A level on a non-local wire, checked against `capability` when there
    /// is one.
    fn checked(
        &self,
        wire: ThinkingWire,
        level: &TurnLevel,
        capability: Option<&ReasoningCapability>,
    ) -> Result<Option<Value>, String> {
        match level {
            TurnLevel::Off => self.off(wire, capability),
            TurnLevel::Level(level) => {
                if let Some(capability) = capability {
                    capability
                        .check(&self.model, level)
                        .map_err(|error| error.to_string())?;
                }
                self.level(wire, level, capability).map(Some)
            }
        }
    }
}

mod offer;
pub use offer::{OfferedLevel, ThinkingOffer};
