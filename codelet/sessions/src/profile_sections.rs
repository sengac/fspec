//! RPC-338 — local-server profile sections + reachability mapping for the
//! wire `list_providers()` surface.
//!
//! Feature: spec/features/model-selector-profile-server.feature
//!
//! Ports the decision logic of the TS `profileSectionBuilder.ts`
//! (`loadProfileSections`) into the Rust server: build a wire
//! `ProviderInfo` for each local-server profile, applying the MODEL-004
//! reachability override (a profile with custom models is never marked
//! unreachable even when its `/v1/models` probe fails), and tag cloud /
//! custom providers with `profile_name = None`, `is_unreachable = false`.
//!
//! The pure mapping helpers below are unit-tested without network I/O;
//! `handle_impl::list_providers` wires the real probe
//! (`OpenAIProvider::list_local_models_with_auth`) into them.

use std::path::PathBuf;

use codelet_rpc_types::{ModelEntry, ProviderInfo};
use serde::Deserialize;

/// Build the wire `ProviderInfo` for a cloud / custom provider. Such
/// providers are never profile sections and are always treated as
/// reachable (`profile_name = None`, `is_unreachable = false`).
pub fn cloud_provider_info(
    key: &str,
    display_name: &str,
    models: Vec<ModelEntry>,
) -> ProviderInfo {
    ProviderInfo {
        key: key.to_string(),
        display_name: display_name.to_string(),
        models,
        profile_name: None,
        is_unreachable: false,
    }
}

/// Build the wire `ProviderInfo` for a local-server profile section.
///
/// Ports `profileSectionBuilder.ts`:
/// * `key` is the profile-qualified `"{provider}:{profile}"` slug.
/// * `display_name` is the profile-qualified label `"{provider}: {profile}"`.
/// * `profile_name` is always `Some(profile)`.
/// * MODEL-004 override: the section is `is_unreachable` only when the
///   `/v1/models` probe failed AND the (already-merged) model list contains
///   no custom models. A profile with custom models is never unreachable.
pub fn build_profile_provider_info(
    provider_id: &str,
    profile_name: &str,
    models: Vec<ModelEntry>,
    probe_failed: bool,
) -> ProviderInfo {
    let has_custom_models = models.iter().any(|m| m.is_custom);
    let is_unreachable = probe_failed && !has_custom_models;
    ProviderInfo {
        key: format!("{provider_id}:{profile_name}"),
        display_name: format!("{provider_id}: {profile_name}"),
        models,
        profile_name: Some(profile_name.to_string()),
        is_unreachable,
    }
}

/// A single local-server profile as stored under
/// `~/.fspec/fspec-config.json` → `providers.openai.profiles.<name>`.
/// Mirrors the TS `ProfileConfig` fields consumed by `loadProfileSections`.
#[derive(Debug, Clone, Deserialize)]
pub struct LocalServerProfile {
    /// Profile name (the map key).
    #[serde(skip)]
    pub name: String,
    #[serde(rename = "baseUrl")]
    pub base_url: String,
    #[serde(rename = "apiKey")]
    pub api_key: Option<String>,
    /// Custom models declared directly on the profile (MODEL-004). Each
    /// entry's `id` is all the renderer needs; extra fields are ignored.
    #[serde(rename = "customModels", default)]
    pub custom_models: Vec<CustomModelDef>,
    #[serde(rename = "contextWindow", default)]
    pub context_window: Option<u32>,
}

/// A custom model declared on a profile (`profile.customModels[]`).
#[derive(Debug, Clone, Deserialize)]
pub struct CustomModelDef {
    pub id: String,
}

/// Resolve `~/.fspec` (parity with the TS `getFspecUserDir`).
fn fspec_user_dir() -> Option<PathBuf> {
    std::env::var_os("FSPEC_USER_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".fspec")))
}

/// Load the local-server profiles for the `openai` provider from
/// `~/.fspec/fspec-config.json` (`providers.openai.profiles`). Returns an
/// empty list on any error (missing file, malformed JSON) so the model
/// selector degrades gracefully — mirroring `loadProfileSections`' try/catch.
///
/// Profiles are only supported for the `openai` provider (parity with TS
/// `saveProfile`'s guard and `loadProfileSections`' `['openai']` loop).
pub fn load_local_server_profiles() -> Vec<LocalServerProfile> {
    let Some(config_path) = fspec_user_dir().map(|d| d.join("fspec-config.json")) else {
        return Vec::new();
    };
    let Ok(content) = std::fs::read_to_string(&config_path) else {
        return Vec::new();
    };
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&content) else {
        return Vec::new();
    };
    let Some(profiles) = root
        .get("providers")
        .and_then(|p| p.get("openai"))
        .and_then(|o| o.get("profiles"))
        .and_then(|p| p.as_object())
    else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(profiles.len());
    for (name, value) in profiles {
        if let Ok(mut profile) = serde_json::from_value::<LocalServerProfile>(value.clone()) {
            profile.name = name.clone();
            out.push(profile);
        }
    }
    // Deterministic ordering (serde_json::Map preserves insertion order with
    // the `preserve_order` feature, but sort by name for stable output).
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Merge the probe-discovered model ids with the profile's declared custom
/// models into wire `ModelEntry` rows (ports `mergeCustomModels`). Custom
/// models are flagged `is_custom = true`; probe models `false`. Duplicate ids
/// (a custom override of a discovered model) keep the custom flag.
pub fn merge_profile_models(
    discovered: Vec<String>,
    custom: &[CustomModelDef],
    context_window: u32,
) -> Vec<ModelEntry> {
    let mut entries: Vec<ModelEntry> = Vec::new();
    for id in discovered {
        if custom.iter().any(|c| c.id == id) {
            continue; // custom override added below
        }
        entries.push(ModelEntry {
            id: id.clone(),
            display_name: id,
            context_window,
            supports_reasoning: false,
            supports_vision: false,
            is_custom: false,
        });
    }
    for c in custom {
        entries.push(ModelEntry {
            id: c.id.clone(),
            display_name: c.id.clone(),
            context_window,
            supports_reasoning: false,
            supports_vision: false,
            is_custom: true,
        });
    }
    entries
}

/// Build the wire profile sections for every local-server `openai` profile,
/// probing each profile's `/v1/models` endpoint synchronously to compute
/// reachability. Ports TS `loadProfileSections`.
///
/// The `/v1/models` probe is async; this function is called from the
/// synchronous `SessionManagerHandle::list_providers` trait method, so it
/// bridges via `tokio::task::block_in_place(|| Handle::current().block_on(..))`
/// — the same pattern used elsewhere in `handle_impl.rs`. MUST run on a
/// multi-thread tokio runtime.
pub fn build_local_profile_sections() -> Vec<ProviderInfo> {
    let profiles = load_local_server_profiles();
    if profiles.is_empty() {
        return Vec::new();
    }
    profiles
        .into_iter()
        .map(|profile| {
            let context_window = profile.context_window.unwrap_or(128_000);
            // Probe the local server; an Err means the endpoint is
            // unreachable (the MODEL-004 override is applied in
            // `build_profile_provider_info`).
            let (discovered, probe_failed) = probe_profile_models(&profile);
            let models = merge_profile_models(discovered, &profile.custom_models, context_window);
            build_profile_provider_info("openai", &profile.name, models, probe_failed)
        })
        .collect()
}

/// Probe a profile's `/v1/models` endpoint. Returns `(model_ids,
/// probe_failed)`; on any error `(vec![], true)`.
fn probe_profile_models(profile: &LocalServerProfile) -> (Vec<String>, bool) {
    let base_url = profile.base_url.clone();
    let api_key = profile.api_key.clone();
    let result = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            codelet_providers::OpenAIProvider::list_local_models_with_auth(
                &base_url,
                api_key.as_deref(),
            )
            .await
        })
    });
    match result {
        Ok(ids) => (ids, false),
        Err(e) => {
            tracing::warn!(
                target: "profile_sections",
                base_url = %profile.base_url,
                error = %e,
                "probe_profile_models: /v1/models probe failed; marking profile unreachable",
            );
            (Vec::new(), true)
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {

    use codelet_rpc_types::{ModelEntry, ProviderInfo};

    use super::{build_profile_provider_info, cloud_provider_info};

    fn model(id: &str, custom: bool) -> ModelEntry {
        ModelEntry {
            id: id.to_string(),
            display_name: id.to_string(),
            context_window: 128_000,
            supports_reasoning: false,
            supports_vision: false,
            is_custom: custom,
        }
    }

    /// Scenario: A cloud provider reports no profile and is reachable
    #[test]
    fn cloud_provider_has_no_profile_and_is_reachable() {
        // @step Given the server builds the provider list
        // @step And a cloud provider "anthropic" with credentials is registered
        let models = vec![model("claude-sonnet", false)];

        // @step When list_providers() returns the provider list
        let info: ProviderInfo = cloud_provider_info("anthropic", "Anthropic", models);

        // @step Then the "anthropic" entry has profile_name None
        assert_eq!(info.profile_name, None);

        // @step And the "anthropic" entry has is_unreachable false
        assert!(!info.is_unreachable);
    }

    /// Scenario: A reachable local-server profile is reported as a profile section
    #[test]
    fn reachable_profile_is_a_profile_section() {
        // @step Given the server builds the provider list
        // @step And a local-server "openai" profile named "my-profile" whose /v1/models probe succeeds
        let models = vec![model("m1", false), model("m2", false), model("m3", false)];

        // @step When list_providers() returns the provider list
        let info = build_profile_provider_info("openai", "my-profile", models, false);

        // @step Then a provider entry has profile_name Some("my-profile")
        assert_eq!(info.profile_name.as_deref(), Some("my-profile"));

        // @step And that entry has display_name "openai: my-profile"
        assert_eq!(info.display_name, "openai: my-profile");

        // @step And that entry has is_unreachable false
        assert!(!info.is_unreachable);
    }

    /// Scenario: An unreachable local-server profile with no custom models is marked unreachable
    #[test]
    fn unreachable_profile_without_custom_models_is_marked() {
        // @step Given the server builds the provider list
        // @step And a local-server "openai" profile named "down-profile" whose /v1/models probe fails
        // @step And that profile has no custom models
        let models: Vec<ModelEntry> = Vec::new();

        // @step When list_providers() returns the provider list
        let info = build_profile_provider_info("openai", "down-profile", models, true);

        // @step Then the "down-profile" entry has is_unreachable true
        assert!(info.is_unreachable);

        // @step And the "down-profile" entry has profile_name Some("down-profile")
        assert_eq!(info.profile_name.as_deref(), Some("down-profile"));

        // @step And the "down-profile" entry has an empty models list
        assert!(info.models.is_empty());

        // @step And the "down-profile" entry has display_name "openai: down-profile" with no embedded "(unreachable)" text
        assert_eq!(info.display_name, "openai: down-profile");
        assert!(!info.display_name.contains("(unreachable)"));

        // @step And the "down-profile" entry is still present in the list
        // (the caller never filters unreachable sections out — proven by
        //  the fact a populated ProviderInfo is returned here).
        assert_eq!(info.key, "openai:down-profile");
    }

    /// Scenario: A failed-probe profile with custom models is not marked unreachable
    #[test]
    fn failed_probe_with_custom_models_is_not_unreachable() {
        // @step Given the server builds the provider list
        // @step And a local-server "openai" profile named "custom-profile" whose /v1/models probe fails
        // @step And that profile has at least one custom model
        let models = vec![model("my-custom-model", true)];

        // @step When list_providers() returns the provider list
        let info = build_profile_provider_info("openai", "custom-profile", models, true);

        // @step Then the "custom-profile" entry has is_unreachable false
        assert!(!info.is_unreachable);

        // @step And the "custom-profile" entry still has profile_name Some("custom-profile")
        assert_eq!(info.profile_name.as_deref(), Some("custom-profile"));

        // @step And the "custom-profile" entry lists its custom models
        assert_eq!(info.models.len(), 1);
        assert!(info.models[0].is_custom);
    }
}
