//! BUG-547: a local profile model's context window and max output are
//! resolved when it is selected (session create, resume and model switch),
//! never discovered at its first turn.
//!
//! Feature: spec/features/a-missing-profile-model-limit-is-resolved-at-selection.feature
//!
//! A declared limit (the profile's, or its customModels entry's) always
//! wins. A missing context window is asked of the profile's declared engine
//! (`codelet_providers::engine_limits`); no engine reports a max output.
//! A limit still missing refuses the selection with [`MissingProfileLimit`],
//! whose message names the field, the profile and where to set it, and which
//! the TUI parses back to ask the user for the value. Nothing is defaulted.

use codelet_providers::local_reasoning::LocalEngine;
use codelet_providers::ProviderManager;

/// A limit a profile model must have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitField {
    ContextWindow,
    MaxOutputTokens,
}

impl LimitField {
    const ALL: [Self; 2] = [Self::ContextWindow, Self::MaxOutputTokens];

    /// The profile key in fspec-config.json.
    pub fn config_key(self) -> &'static str {
        match self {
            Self::ContextWindow => "contextWindow",
            Self::MaxOutputTokens => "maxOutputTokens",
        }
    }

    /// The limit as messages name it.
    pub fn description(self) -> &'static str {
        match self {
            Self::ContextWindow => "context window",
            Self::MaxOutputTokens => "max output tokens",
        }
    }

    /// The limit as the TUI provider settings label it.
    pub fn label(self) -> &'static str {
        match self {
            Self::ContextWindow => "Context Window",
            Self::MaxOutputTokens => "Max Output Tokens",
        }
    }
}

/// A profile model refused because a limit is missing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingProfileLimit {
    /// The selection, `provider:profile/model`.
    pub model: String,
    pub provider: String,
    pub profile: String,
    pub field: LimitField,
    /// The fspec-config.json the profile is stored in.
    pub config_path: String,
}

impl std::fmt::Display for MissingProfileLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} has no {}: set {} on the {} profile in {} (providers.{}.profiles.{}) or in the TUI provider settings",
            self.model,
            self.field.description(),
            self.field.config_key(),
            self.profile,
            self.config_path,
            self.provider,
            self.profile
        )
    }
}

impl MissingProfileLimit {
    /// The refusal inside `error` (which may carry a prefix such as
    /// `[error] create session: `), parsed back; `None` when `error` is
    /// not one.
    pub fn find_in(error: &str) -> Option<Self> {
        let (before, after) = error.split_once(" has no ")?;
        let (description, rest) = after.split_once(": set ")?;
        let field = LimitField::ALL
            .into_iter()
            .find(|field| field.description() == description)?;
        let rest = rest
            .strip_prefix(field.config_key())?
            .strip_prefix(" on the ")?;
        let (profile, rest) = rest.split_once(" profile in ")?;
        let (config_path, rest) = rest.rsplit_once(" (providers.")?;
        let (provider, rest) = rest.split_once(".profiles.")?;
        rest.strip_prefix(profile)?
            .strip_prefix(") or in the TUI provider settings")?;
        let start = before.rfind(&format!("{provider}:{profile}/"))?;
        Some(Self {
            model: before[start..].to_string(),
            provider: provider.to_string(),
            profile: profile.to_string(),
            field,
            config_path: config_path.to_string(),
        })
    }
}

/// `openai:profile/model` split into `(profile, model)`; `None` for any
/// other selection.
fn local_profile_selection(model: &str) -> Option<(&str, &str)> {
    let rest = model.strip_prefix("openai:")?;
    let (profile, model_id) = rest.split_once('/')?;
    (!profile.is_empty() && !model_id.is_empty()).then_some((profile, model_id))
}

/// BUG-547: for a local profile model, ask its engine for a missing context
/// window, then refuse the selection while a limit is still missing. Other
/// selections are untouched.
pub(crate) async fn resolve_missing_profile_limits(
    manager: &mut ProviderManager,
    model: &str,
) -> Result<(), String> {
    let Some((profile_name, model_id)) = local_profile_selection(model) else {
        return Ok(());
    };
    if manager.context_window().is_none() {
        ask_engine_for_context_window(manager, profile_name, model_id).await?;
    }
    let missing = [
        (
            LimitField::ContextWindow,
            manager.context_window().is_none(),
        ),
        (
            LimitField::MaxOutputTokens,
            manager.max_output_tokens().is_none(),
        ),
    ]
    .into_iter()
    .find_map(|(field, missing)| missing.then_some(field));
    let Some(field) = missing else {
        return Ok(());
    };
    let config_path = codelet_common::user_dir::fspec_user_dir()?.join("fspec-config.json");
    Err(MissingProfileLimit {
        model: model.to_string(),
        provider: "openai".to_string(),
        profile: profile_name.to_string(),
        field,
        config_path: config_path.display().to_string(),
    }
    .to_string())
}

/// Ask the profile's declared engine (never inferred) for `model_id`'s
/// running context window and record a usable report on the manager.
async fn ask_engine_for_context_window(
    manager: &mut ProviderManager,
    profile_name: &str,
    model_id: &str,
) -> Result<(), String> {
    let Some(profile) = crate::profile_sections::load_local_server_profiles()?
        .into_iter()
        .find(|profile| profile.name == profile_name)
    else {
        return Ok(());
    };
    let Some(engine) = profile.engine.as_deref().and_then(LocalEngine::from_config) else {
        tracing::info!(
            profile = profile_name,
            "BUG-547: the profile declares no engine; its context window is not asked"
        );
        return Ok(());
    };
    match codelet_providers::engine_limits::context_window_report(
        engine,
        &profile.base_url,
        profile.api_key.as_deref(),
        model_id,
    )
    .await
    {
        Ok(limit) => manager.set_engine_context_window(limit),
        Err(reason) => tracing::warn!(
            profile = profile_name,
            model = model_id,
            %reason,
            "BUG-547: the engine reports no usable context window"
        ),
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_refusal_parses_back_from_its_message() {
        let refusal = MissingProfileLimit {
            model: "openai:My profile/org/model".into(),
            provider: "openai".into(),
            profile: "My profile".into(),
            field: LimitField::ContextWindow,
            config_path: "/home/u/.fspec/fspec-config.json".into(),
        };
        let wrapped = format!("[error] Model selection failed: {refusal}");
        assert_eq!(MissingProfileLimit::find_in(&wrapped), Some(refusal));
        assert_eq!(MissingProfileLimit::find_in("session busy"), None);
    }

    #[test]
    fn only_local_profile_selections_are_checked() {
        assert_eq!(
            local_profile_selection("openai:Strata/qwen"),
            Some(("Strata", "qwen"))
        );
        assert_eq!(local_profile_selection("openai/gpt-4o"), None);
        assert_eq!(local_profile_selection("anthropic:work/claude"), None);
    }
}
