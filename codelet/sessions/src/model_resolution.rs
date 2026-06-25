//! RPC-343: shared model-selection resolution.
//!
//! Both session creation (`SessionManager::create_session_with_id`) and the
//! mid-session model switch (`SessionManagerHandle::set_model`) must resolve a
//! `provider/model` selection the same way — detect the model kind, apply it to
//! the `ProviderManager`, and read back the resolved limits. Keeping that logic
//! in one place stops the two paths from drifting (the RPC-343 bug was exactly
//! that drift: the mid-session path re-resolved nothing).

use codelet_providers::ProviderManager;

/// Limits resolved for a selected model, read back from the provider manager
/// after the selection is applied. Values are already clamped by the provider's
/// `ModelLimitsResolver`.
pub struct ResolvedModelLimits {
    pub context_window: u32,
    pub max_output_tokens: u32,
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
        pm.set_model_direct_with_profile(
            registry_provider,
            model_part,
            Some(profile_name),
            None,
            None,
            None,
        )
        .map_err(|e| format!("Failed to set profile model: {e}"))?;

        // Single source of truth for the credential bridge — shared with the
        // isolated-session create path (session_manager.rs).
        apply_profile_env_vars(registry_provider, profile_name, model_part)?;
    } else if is_codex_model || is_custom_model {
        tracing::info!(
            target: "model_resolution",
            model,
            "applying model via set_model_direct (codex/custom)"
        );
        pm.set_model_direct(registry_provider, model_part, None, None, None)
            .map_err(|e| format!("Failed to set model: {e}"))?;

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
        // RPC-348: `select_model` never touches `facade_override`, so a switch
        // from a previously-selected custom model would otherwise leave a stale
        // facade pointing at the old custom provider. Clear it for plain
        // registry selections.
        pm.set_facade_override(None);
    }

    Ok(ResolvedModelLimits {
        context_window: pm.context_window() as u32,
        max_output_tokens: pm.max_output_tokens() as u32,
    })
}

/// PROV-121: bridge a selected local-server profile's stored credentials into
/// the process environment the OpenAI client reads at dispatch time. This is
/// the SINGLE source of truth for the credential bridge — both the shared
/// resolver ([`apply_model_selection`]'s profile branch) and the
/// isolated-session create path (`SessionManager::create_isolated_session_with_id`)
/// call it so the two paths can never drift.
///
/// Mirrors the TS `configureProfileEnvironment` and the custom-provider
/// `apply_custom_provider_env_vars` pattern: looks the profile up by name in
/// `~/.fspec/fspec-config.json` (`providers.openai.profiles.<name>`) and sets:
/// * `OPENAI_BASE_URL` ← `profile.baseUrl`
/// * `OPENAI_API_KEY`  ← `profile.apiKey` (only when present and non-empty)
/// * `OPENAI_CONTEXT_WINDOW` ← `profile.contextWindow` (only when present)
///
/// Per the PROV-121 ruling `max_output_tokens` is intentionally NOT bridged
/// (`OPENAI_MAX_OUTPUT_TOKENS` is left unset). A profile that cannot be found
/// degrades gracefully (warn + `Ok`) rather than failing dispatch, matching the
/// TS try/catch behaviour.
pub fn apply_profile_env_vars(
    provider: &str,
    profile_name: &str,
    model: &str,
) -> Result<(), String> {
    let Some(profile) = crate::profile_sections::load_local_server_profiles()
        .into_iter()
        .find(|p| p.name == profile_name)
    else {
        tracing::warn!(
            target: "model_resolution",
            provider,
            profile_name,
            model,
            "apply_profile_env_vars: no stored profile found; leaving OPENAI_* env unchanged"
        );
        return Ok(());
    };

    std::env::set_var("OPENAI_BASE_URL", &profile.base_url);
    if let Some(api_key) = profile.api_key.as_deref() {
        if !api_key.is_empty() {
            std::env::set_var("OPENAI_API_KEY", api_key);
        }
    }
    if let Some(context_window) = profile.context_window {
        std::env::set_var("OPENAI_CONTEXT_WINDOW", context_window.to_string());
    }

    tracing::info!(
        target: "model_resolution",
        provider,
        profile_name,
        model,
        base_url = %profile.base_url,
        "apply_profile_env_vars: bridged profile credentials into OPENAI_* env"
    );
    Ok(())
}
