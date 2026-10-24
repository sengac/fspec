//! RPC-343: shared model-selection resolution.
//!
//! Both session creation (`SessionManager::create_session_with_id`) and the
//! mid-session model switch (`SessionManagerHandle::set_model`) must resolve a
//! `provider/model` selection the same way — detect the model kind, apply it to
//! the `ProviderManager`, and read back the resolved limits. Keeping that logic
//! in one place stops the two paths from drifting (the RPC-343 bug was exactly
//! that drift: the mid-session path re-resolved nothing).

pub use crate::profile_environment::apply_profile_env_vars;
use codelet_providers::ProviderManager;

/// BUG-168: resolve the vision capability of the manager's currently selected
/// model, for the tool-layer session capability registry.
///
/// Resolution order:
/// 1. **Profile model** (composite `provider:profile/model`) — the profile's
///    `customModels[]` entry with a matching id reports its `hasVision`
///    (absent/unknown -> `false`, conservative).
/// 2. **Custom provider** (registered via `<home>/providers/*.json`) — the
///    config's `ModelDef.supports_vision` (absent -> `false`).
/// 3. **Cloud/codex** — the models.dev registry entry's vision capability
///    (no registry or miss -> `false`, conservative).
pub fn resolve_model_vision(pm: &ProviderManager) -> bool {
    let Some(composite) = pm.selected_model_string() else {
        return false;
    };

    // Profile selection: "provider:profile/model" (colon before the slash).
    let (colon, slash) = (composite.find(':'), composite.find('/'));
    if let (Some(colon), Some(slash)) = (colon, slash) {
        if colon < slash && composite.starts_with("openai:") {
            let profile_name = &composite[colon + 1..slash];
            let model_id = &composite[slash + 1..];
            return crate::profile_sections::load_local_server_profiles()
                .into_iter()
                .find(|p| p.name == profile_name)
                .and_then(|p| p.custom_models.into_iter().find(|c| c.id == model_id))
                .and_then(|c| c.has_vision)
                .unwrap_or(false);
        }
    }

    // Custom provider model: registered config carries supports_vision.
    if let Some(slash) = composite.find('/') {
        let provider = pm.current_provider_name();
        let model_id = &composite[slash + 1..];
        if codelet_providers::custom_provider_registered(provider) {
            return codelet_providers::custom::discover_provider_configs()
                .ok()
                .and_then(|configs| {
                    configs
                        .into_iter()
                        .find(|c| c.name == provider)
                        .and_then(|c| c.models.get(model_id).cloned())
                })
                .map(|def| def.supports_vision)
                .unwrap_or(false);
        }
    }

    // Cloud/codex: models.dev registry capability.
    pm.selected_model_info()
        .map(|info| info.has_capability(codelet_providers::models::Capability::Vision))
        .unwrap_or(false)
}

pub fn resolve_profile_max_images(pm: &ProviderManager) -> Option<u32> {
    pm.runtime_options().max_images
}

/// PROV-145: the per-profile loop-detection values, resolved flat.
///
/// `codelet-sessions` cannot return the `LoopDetectorConfig` that lives in
/// `codelet-agent-loop` (crate cycle: agent-loop already depends on
/// sessions), so this returns the flat stored values — `Option<bool>` for
/// the enabled toggle, `Option<u32>` for the numeric fields. The caller
/// (the agent-loop layer) assembles the detector config and applies the
/// canonical defaults (enabled, 160, 10, 10) for absent values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProfileLoopDetection {
    /// Stored `loopDetectionEnabled`; `None` ⇒ enabled (RIG-014 default).
    pub enabled: Option<bool>,
    /// Stored `loopDetectionWindow` (words); `None` ⇒ 160.
    pub window: Option<u32>,
    /// Stored `loopDetectionMaxRepeats`; `None` ⇒ 10.
    pub max_repeats: Option<u32>,
    /// Stored `loopDetectionMaxRetries`; `None` ⇒ 10.
    pub max_retries: Option<u32>,
}

pub fn resolve_profile_loop_detection(pm: &ProviderManager) -> ProfileLoopDetection {
    let options = pm.runtime_options();
    ProfileLoopDetection {
        enabled: options.loop_detection_enabled,
        window: options.loop_detection_window,
        max_repeats: options.loop_detection_max_repeats,
        max_retries: options.loop_detection_max_retries,
    }
}

/// Limits resolved for a selected model, read back from the provider manager
/// after the selection is applied. Values are already clamped by the provider's
/// `ModelLimitsResolver`.
pub struct ResolvedModelLimits {
    pub context_window: u32,
    pub max_output_tokens: u32,
    pub compaction_threshold: u32,
}

/// Apply a `provider/model` selection to an existing [`ProviderManager`],
/// mirroring creation-time detection: profile / codex / custom models route
/// through `set_model_direct`, everything else through `select_model`. Returns
/// the resolved context window and max output tokens.
///
/// On error the manager state is left unchanged — both `select_model` and
/// `set_model_direct` validate the model against the registry before mutating
/// any internal state, so a failed call cannot leave a half-applied selection.
pub fn apply_model_selection(
    pm: &mut ProviderManager,
    model: &str,
) -> Result<ResolvedModelLimits, String> {
    if model.is_empty() || !model.contains('/') {
        return Err(format!(
            "Invalid model string '{model}': must be in 'provider/model-id' format (e.g., 'anthropic/claude-opus-4-5')"
        ));
    }

    // Detection mirrors SessionManager::create_session_with_id.
    let is_profile_model = model.contains(':') && model.find(':') < model.find('/');
    let is_codex_model = model.starts_with("codex/");

    let (registry_provider, model_part) = if is_profile_model {
        let colon_idx = model
            .find(':')
            .ok_or_else(|| format!("Invalid profile model string '{model}': missing ':'"))?;
        let slash_idx = model
            .find('/')
            .ok_or_else(|| format!("Invalid profile model string '{model}': missing '/'"))?;
        (&model[..colon_idx], &model[slash_idx + 1..])
    } else {
        let parts: Vec<&str> = model.splitn(2, '/').collect();
        (parts[0], parts.get(1).copied().unwrap_or(""))
    };

    if registry_provider.is_empty() || model_part.is_empty() {
        return Err(format!(
            "Invalid model string '{model}': must be in 'provider/model-id' format (e.g., 'anthropic/claude-opus-4-5')"
        ));
    }

    let is_custom_model = !is_profile_model
        && !is_codex_model
        && codelet_providers::custom_provider_registered(registry_provider);

    if is_profile_model {
        // PROV-121: a profile model (provider:profile/model) must (a) preserve
        // the profile name on the manager so `selected_model_string()` rebuilds
        // the composite `openai:qwen/qwen`, and (b) bridge the profile's stored
        // `baseUrl`/`apiKey` into the environment the OpenAI client reads before
        // dispatch. Without (b) dispatch fails with `OPENAI_API_KEY not set`.
        let colon_idx = model
            .find(':')
            .ok_or_else(|| format!("Invalid profile model string '{model}': missing ':'"))?;
        let slash_idx = model
            .find('/')
            .ok_or_else(|| format!("Invalid profile model string '{model}': missing '/'"))?;
        let profile_name = &model[colon_idx + 1..slash_idx];

        tracing::info!(
            target: "model_resolution",
            model,
            profile_name,
            "applying profile model via set_model_direct_with_profile + env bridge"
        );
        crate::profile_model_limits::apply_profile_model(
            pm,
            registry_provider,
            profile_name,
            model_part,
        )?;
    } else if is_codex_model || is_custom_model {
        tracing::info!(
            target: "model_resolution",
            model,
            "applying model via set_model_direct (codex/custom)"
        );
        crate::profile_model_limits::apply_direct_model(pm, registry_provider, model_part)?;
        // RPC-348: re-resolve the per-selection facade override for custom
        // (registered custom-provider) models. `set_model_direct` was called
        // with `facade_override = None`, so without this step both the
        // creation path and the mid-session set_model path leave the inner
        // manager's facade unset relative to the new custom model — the
        // shared port gap the TS `lookupFacadeOverride` boundary covered.
        //
        // Mirrors the NAPI `session_set_model_profile` post-set_model_direct
        // block (session_bindings.rs): derive the facade from the registered
        // config (explicit `facade` wins; otherwise derived from `api_style`;
        // `None` for Rhai-scripted providers), store it on the manager, and
        // apply the facade's env vars so dispatch works end-to-end.
        if is_custom_model {
            let facade = codelet_providers::custom::derive_facade_for_custom(registry_provider);
            pm.set_facade_override(facade.clone());
            if let Err(e) = codelet_providers::custom::apply_custom_provider_env_vars(
                registry_provider,
                model_part,
                facade.as_deref(),
            ) {
                tracing::warn!(
                    target: "model_resolution",
                    provider = registry_provider,
                    error = %e,
                    "apply_custom_provider_env_vars failed for custom model"
                );
            }
        }
    } else {
        pm.select_model(model)
            .map_err(|e| format!("Failed to select model: {e}"))?;
        pm.set_compaction_threshold_override(None);
        // RPC-348: `select_model` never touches `facade_override`, so a switch
        // from a previously-selected custom model would otherwise leave a stale
        // facade pointing at the old custom provider. Clear it for plain
        // registry selections.
        pm.set_facade_override(None);
    }

    Ok(resolve_current_model_limits(pm))
}

pub fn resolve_current_model_limits(pm: &ProviderManager) -> ResolvedModelLimits {
    let threshold = pm.compaction_threshold_override().map(|(kind, value)| {
        codelet_cli::compaction_threshold::CompactionThresholdConfig::from_type_value(kind, value)
    });
    ResolvedModelLimits {
        context_window: pm.context_window() as u32,
        max_output_tokens: pm.max_output_tokens() as u32,
        compaction_threshold: codelet_cli::compaction_threshold::resolve_compaction_threshold(
            pm.context_window() as u64,
            pm.max_output_tokens() as u64,
            pm.selected_model_id().as_deref(),
            threshold.as_ref(),
        ) as u32,
    }
}

/// WT-012: build a `ProviderManager` for a model string, funneling the
/// construction + selection that was previously copy-pasted across
/// `create_session_with_id`, `create_session_from_manifest` and
/// `create_isolated_session_with_id`.
///
/// The funnel is `with_model_support()` (full registry) +
/// [`apply_model_selection`] — the EXACT sequence the RPC-425 shared helper
/// applies after construction, so every creation path (non-isolated create,
/// resume, and now isolated create) resolves model selection through one
/// code path:
///
/// * **registry model** (`provider/model`) — `select_model` (full registry
///   validation, credential re-detection) + facade clear (RPC-348).
/// * **profile model** (`provider:profile/model`) —
///   `set_model_direct_with_profile` + the PROV-121 credential bridge
///   (`apply_profile_env_vars`).
/// * **codex / custom model** — `set_model_direct` + the RPC-348
///   custom-provider facade re-resolution.
///
/// Must be called AFTER `credentials::resolve_and_set_env_var` so the
/// registry re-detects freshly set credentials (PROV-057 staleness rule).
pub async fn resolve_provider_manager(model: &str) -> Result<ProviderManager, String> {
    resolve_provider_manager_and_limits(model)
        .await
        .map(|(manager, _)| manager)
}

pub(crate) async fn resolve_provider_manager_and_limits(
    model: &str,
) -> Result<(ProviderManager, ResolvedModelLimits), String> {
    let mut provider_manager = ProviderManager::with_model_support()
        .await
        .map_err(|e| format!("Failed to create provider manager: {e}"))?;
    let limits = apply_model_selection(&mut provider_manager, model)?;
    Ok((provider_manager, limits))
}
