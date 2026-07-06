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

use std::path::{Path, PathBuf};

use codelet_rpc_types::{ModelEntry, ProviderInfo};
use serde::{Deserialize, Serialize};

/// Build the wire `ProviderInfo` for a cloud / custom provider. Such
/// providers are never profile sections and are always treated as
/// reachable (`profile_name = None`, `is_unreachable = false`).
pub fn cloud_provider_info(key: &str, display_name: &str, models: Vec<ModelEntry>) -> ProviderInfo {
    ProviderInfo {
        key: key.to_string(),
        display_name: display_name.to_string(),
        models,
        profile_name: None,
        is_unreachable: false,
    }
}

/// PROV-127: drop cloud provider sections whose model list is empty, so the
/// `/model` selector does not render dead "Provider (0 models)" headers.
/// Mirrors the TS reference (`cloudSectionBuilder.ts`
/// `filter(s => s.hasCredentials)` + `modelInitializationService.ts`
/// `filter(s => s.models.length > 0)`).
///
/// A **cloud** section is identified by `profile_name == None`. A **local
/// profile** section (`profile_name == Some(..)`) is NEVER dropped by this
/// filter — an unreachable profile with zero models must still render (see
/// RPC-338 / MODEL-004).
pub fn retain_populated_cloud_sections(sections: Vec<ProviderInfo>) -> Vec<ProviderInfo> {
    sections
        .into_iter()
        .filter(|section| section.profile_name.is_some() || !section.models.is_empty())
        .collect()
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
    #[serde(
        rename = "contextWindow",
        default,
        deserialize_with = "de_opt_u32_lenient"
    )]
    pub context_window: Option<u32>,
    /// PROV-140: per-profile streaming toggle. `None` (absent) means streaming
    /// stays enabled (the provider default); `Some(false)` selects the
    /// non-streaming request path.
    #[serde(rename = "streaming", default)]
    pub streaming: Option<bool>,
}

/// A custom model declared on a profile (`profile.customModels[]`).
///
/// RPC-346: extended from the read-only `{ id }` shape to the full
/// definition mirroring the TS `CustomModelDefinition`
/// (provider-config.ts:95-112). All fields besides `id` are optional and
/// omitted from the serialized JSON when `None`, keeping existing config
/// files backward-compatible. Wire names are camelCase.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomModelDef {
    /// Model ID string sent to the API (required).
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display_name: Option<String>,
    /// Facade override for tool-schema selection
    /// (`openai` | `codex` | `claude` | `gemini` | `zai`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub facade: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "de_opt_u32_lenient"
    )]
    pub context_window: Option<u32>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "de_opt_u32_lenient"
    )]
    pub max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub compaction_threshold: Option<CompactionThreshold>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reasoning: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub has_vision: Option<bool>,
}

/// CTX-008 compaction-threshold override on a custom model. Mirrors the TS
/// `CompactionThresholdConfig` (`{ type: 'tokens' | 'percentage', value }`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompactionThreshold {
    /// `"tokens"` (absolute count) or `"percentage"` (1-100 of context window).
    #[serde(rename = "type")]
    pub threshold_type: String,
    #[serde(deserialize_with = "de_u32_lenient")]
    pub value: u32,
}

/// Lenient deserializer for an optional unsigned integer that tolerates a JSON
/// number written as a float (e.g. `80.0`) or one exceeding the `u32` range
/// (saturating to `u32::MAX`), matching the TS `number` width so a config
/// written by the TS build never drops the whole profile on read (RPC-346
/// parity fix). A non-numeric / negative / non-finite value deserializes to
/// `None` rather than failing the surrounding profile.
fn de_opt_u32_lenient<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(value.and_then(coerce_u32))
}

/// Lenient deserializer for a required `u32`; tolerates floats / oversized
/// numbers the same way as [`de_opt_u32_lenient`], falling back to `0` rather
/// than failing the surrounding profile.
fn de_u32_lenient<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(coerce_u32(value).unwrap_or(0))
}

/// Coerce a JSON value into a `u32`, truncating floats and saturating values
/// above `u32::MAX`. Returns `None` for non-numeric, negative or non-finite
/// inputs.
fn coerce_u32(value: serde_json::Value) -> Option<u32> {
    match value {
        serde_json::Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                Some(u.min(u64::from(u32::MAX)) as u32)
            } else if let Some(f) = n.as_f64() {
                if f.is_finite() && f >= 0.0 {
                    Some(f.min(f64::from(u32::MAX)) as u32)
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Resolve `~/.fspec` (parity with the TS `getFspecUserDir`).
pub(crate) fn fspec_user_dir() -> Option<PathBuf> {
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
    load_local_server_profiles_from(&config_path)
}

/// Path-injectable core of [`load_local_server_profiles`]. Reads the given
/// `fspec-config.json` and returns the `providers.openai.profiles` as
/// [`LocalServerProfile`]s, degrading to an empty list on any error. Kept
/// private so callers go through the env-resolved public function; the
/// RPC-346 persistence tests exercise this directly against a temp file.
fn load_local_server_profiles_from(config_path: &Path) -> Vec<LocalServerProfile> {
    let Ok(content) = std::fs::read_to_string(config_path) else {
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

/// RPC-346: add or update a custom model on a local-server profile.
///
/// When `original_model_id` is `Some`, the entry with that id is replaced in
/// place (edit); otherwise the definition is appended (add). Mirrors the TS
/// `customModelCrudService.saveCustomModel`. Resolves the config path via the
/// `FSPEC_USER_DIR`/`HOME` convention and delegates to
/// [`save_custom_model_at`]. No-op (returns `Ok`) when the target profile does
/// not exist or `provider_id` is not `"openai"`.
pub fn save_custom_model(
    provider_id: &str,
    profile_name: &str,
    definition: &CustomModelDef,
    original_model_id: Option<&str>,
) -> std::io::Result<()> {
    let Some(config_path) = fspec_user_dir().map(|d| d.join("fspec-config.json")) else {
        return Ok(());
    };
    save_custom_model_at(
        &config_path,
        provider_id,
        profile_name,
        definition,
        original_model_id,
    )
}

/// RPC-346: delete a custom model from a local-server profile by id.
///
/// Mirrors the TS `customModelCrudService.deleteCustomModel`: removes the
/// matching entry and drops the `customModels` key entirely when the array
/// becomes empty. No-op when the profile is absent or `provider_id` is not
/// `"openai"`.
pub fn delete_custom_model(
    provider_id: &str,
    profile_name: &str,
    model_id: &str,
) -> std::io::Result<()> {
    let Some(config_path) = fspec_user_dir().map(|d| d.join("fspec-config.json")) else {
        return Ok(());
    };
    delete_custom_model_at(&config_path, provider_id, profile_name, model_id)
}

/// Borrow the `providers.openai.profiles.<name>` object out of the parsed
/// config, or `None` when any segment is missing. Profiles are only supported
/// for the `openai` provider (parity with the TS `saveProfile` guard).
fn profile_object_mut<'a>(
    root: &'a mut serde_json::Value,
    provider_id: &str,
    profile_name: &str,
) -> Option<&'a mut serde_json::Map<String, serde_json::Value>> {
    if provider_id != "openai" {
        return None;
    }
    root.get_mut("providers")?
        .get_mut(provider_id)?
        .get_mut("profiles")?
        .get_mut(profile_name)?
        .as_object_mut()
}

/// Path-injectable core of [`save_custom_model`]. Performs a whole-file
/// read-modify-write of `config_path`, mutating only the target profile's
/// `customModels` array so unrelated keys are preserved verbatim.
fn save_custom_model_at(
    config_path: &Path,
    provider_id: &str,
    profile_name: &str,
    definition: &CustomModelDef,
    original_model_id: Option<&str>,
) -> std::io::Result<()> {
    let Some(mut root) = read_config_value(config_path) else {
        // Missing or malformed config: there is no profile to modify, so the
        // operation is a no-op and the file is left untouched.
        return Ok(());
    };
    let Some(profile) = profile_object_mut(&mut root, provider_id, profile_name) else {
        return Ok(());
    };

    let def_value = serde_json::to_value(definition)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let mut entries: Vec<serde_json::Value> = profile
        .get("customModels")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    match original_model_id {
        Some(original) => {
            for entry in entries.iter_mut() {
                if entry.get("id").and_then(|v| v.as_str()) == Some(original) {
                    *entry = def_value.clone();
                }
            }
        }
        None => entries.push(def_value),
    }

    profile.insert(
        "customModels".to_string(),
        serde_json::Value::Array(entries),
    );
    write_config_value(config_path, &root)
}

/// Path-injectable core of [`delete_custom_model`].
fn delete_custom_model_at(
    config_path: &Path,
    provider_id: &str,
    profile_name: &str,
    model_id: &str,
) -> std::io::Result<()> {
    let Some(mut root) = read_config_value(config_path) else {
        return Ok(());
    };
    let Some(profile) = profile_object_mut(&mut root, provider_id, profile_name) else {
        return Ok(());
    };

    let mut entries: Vec<serde_json::Value> = profile
        .get("customModels")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    entries.retain(|entry| entry.get("id").and_then(|v| v.as_str()) != Some(model_id));

    if entries.is_empty() {
        profile.remove("customModels");
    } else {
        profile.insert(
            "customModels".to_string(),
            serde_json::Value::Array(entries),
        );
    }
    write_config_value(config_path, &root)
}

/// Read `config_path` into a JSON `Value`, returning `None` on a missing or
/// malformed file (caller treats this as a no-op).
pub(crate) fn read_config_value(config_path: &Path) -> Option<serde_json::Value> {
    let content = std::fs::read_to_string(config_path).ok()?;
    serde_json::from_str::<serde_json::Value>(&content).ok()
}

/// Write the JSON `Value` back to `config_path` (pretty-printed with a
/// trailing newline). With the workspace `preserve_order` serde_json feature,
/// existing key order is retained.
pub(crate) fn write_config_value(
    config_path: &Path,
    root: &serde_json::Value,
) -> std::io::Result<()> {
    let mut serialized = serde_json::to_string_pretty(root)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    serialized.push('\n');
    std::fs::write(config_path, serialized)
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
            // Parity with TS `custom.displayName || custom.id`: fall back to
            // the id only when no display name is stored, instead of always
            // overwriting it with the id (RPC-346/RPC-344 prefill fix).
            display_name: c.display_name.clone().unwrap_or_else(|| c.id.clone()),
            context_window: c.context_window.unwrap_or(context_window),
            supports_reasoning: c.reasoning.unwrap_or(false),
            supports_vision: c.has_vision.unwrap_or(false),
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
///
/// PROV-131: the probe bridges an async call into this synchronous function via
/// `tokio::task::block_in_place(|| Handle::current().block_on(..))`, which
/// requires a live MULTI-THREAD tokio runtime. When `list_providers` is invoked
/// from a plain (non-tokio) or current-thread context there is no such runtime,
/// so `block_in_place` / `Handle::current()` would PANIC ("there is no reactor
/// running" / "can call blocking only when running on the multi-threaded
/// runtime"). Guard the bridge with `Handle::try_current()` +
/// `runtime_flavor() == MultiThread`; when absent, log via `tracing::warn` and
/// degrade gracefully — skip the probe and report it as failed so the profile
/// renders unreachable (empty models). Mirrors the TS `loadProfileSections`
/// try/catch that sets `isUnreachable = true` on any probe failure.
fn probe_profile_models(profile: &LocalServerProfile) -> (Vec<String>, bool) {
    let on_multi_thread_runtime = matches!(
        tokio::runtime::Handle::try_current().map(|h| h.runtime_flavor()),
        Ok(tokio::runtime::RuntimeFlavor::MultiThread)
    );
    if !on_multi_thread_runtime {
        tracing::warn!(
            target: "profile_sections",
            base_url = %profile.base_url,
            "probe_profile_models: no multi-thread tokio runtime available; \
             skipping /v1/models probe and marking profile unreachable",
        );
        return (Vec::new(), true);
    }
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

    // ====================================================================
    // PROV-127 — drop empty/uncredentialed cloud sections (TS parity).
    // Feature: spec/features/model-selector-cloud-sections.feature
    // Mirrors TS cloudSectionBuilder.ts `filter(s => s.hasCredentials)` +
    // modelInitializationService.ts `filter(s => s.models.length > 0)`.
    // A cloud section (profile_name == None) with an empty model list must
    // be dropped; a local profile section (profile_name == Some) is never
    // dropped by this filter.
    // ====================================================================

    /// Scenario: A cloud provider with no models is dropped from the provider list
    #[test]
    fn empty_cloud_section_is_dropped() {
        // @step Given a canonical cloud provider that resolves to an empty model list
        let sections = vec![cloud_provider_info("cohere", "Cohere", Vec::new())];

        // @step When the provider list is assembled
        let kept = super::retain_populated_cloud_sections(sections);

        // @step Then that cloud provider section is not present in the list
        assert!(
            !kept.iter().any(|p| p.key == "cohere"),
            "empty cloud section must be dropped"
        );
    }

    /// Scenario: A cloud provider with one or more models is kept unchanged
    #[test]
    fn populated_cloud_section_is_kept_unchanged() {
        // @step Given a canonical cloud provider that resolves to a non-empty model list
        let models = vec![model("claude-sonnet", false)];
        let sections = vec![cloud_provider_info("anthropic", "Anthropic", models)];

        // @step When the provider list is assembled
        let kept = super::retain_populated_cloud_sections(sections);

        // @step Then that cloud provider section is present with its models intact
        let anthropic = kept
            .iter()
            .find(|p| p.key == "anthropic")
            .expect("anthropic present");
        assert_eq!(anthropic.models.len(), 1);
        assert_eq!(anthropic.models[0].id, "claude-sonnet");
    }

    /// Scenario: Local profile sections are not affected by the empty-cloud filter
    #[test]
    fn empty_local_profile_section_is_not_dropped() {
        // @step Given a local profile section with an empty model list
        let sections = vec![build_profile_provider_info(
            "openai",
            "down-profile",
            Vec::new(),
            true,
        )];

        // @step When the provider list is assembled
        let kept = super::retain_populated_cloud_sections(sections);

        // @step Then the local profile section is still present in the list
        assert!(
            kept.iter()
                .any(|p| p.profile_name.as_deref() == Some("down-profile")),
            "local profile section must be preserved even with zero models"
        );
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod persistence_tests {
    //! RPC-346 — custom-model persistence (save/delete) on local-server
    //! profiles. Exercises the path-injectable helpers against a temp
    //! config file so the suite stays fully offline.

    use super::{
        delete_custom_model_at, load_local_server_profiles_from, merge_profile_models,
        save_custom_model_at, CompactionThreshold, CustomModelDef,
    };
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn def(id: &str) -> CustomModelDef {
        CustomModelDef {
            id: id.to_string(),
            display_name: None,
            facade: None,
            context_window: None,
            max_output_tokens: None,
            compaction_threshold: None,
            reasoning: None,
            has_vision: None,
        }
    }

    /// Write a config file with one openai profile, returning (dir, path).
    fn config_with(profile: &str, custom_models_json: &str) -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("fspec-config.json");
        let body = format!(
            r#"{{"providers":{{"openai":{{"profiles":{{"{profile}":{{"baseUrl":"http://localhost:8888","apiKey":"sk-test"{custom_models_json}}}}}}}}}}}"#
        );
        fs::write(&path, body).unwrap();
        (dir, path)
    }

    fn profile_custom_ids(path: &PathBuf, profile: &str) -> Option<Vec<String>> {
        let content = fs::read_to_string(path).unwrap();
        let root: serde_json::Value = serde_json::from_str(&content).unwrap();
        let p = &root["providers"]["openai"]["profiles"][profile];
        p.get("customModels").map(|v| {
            v.as_array()
                .unwrap()
                .iter()
                .map(|m| m["id"].as_str().unwrap().to_string())
                .collect()
        })
    }

    /// Scenario: Add a custom model to a profile with no custom models
    #[test]
    fn add_to_profile_without_custom_models() {
        // @step Given an "openai" profile "work-vllm" with no custom models
        let (_dir, path) = config_with("work-vllm", "");

        // @step When I save a new custom model "my-model" in add mode
        save_custom_model_at(&path, "openai", "work-vllm", &def("my-model"), None).unwrap();

        // @step Then the profile's custom models are exactly ["my-model"]
        assert_eq!(
            profile_custom_ids(&path, "work-vllm"),
            Some(vec!["my-model".to_string()])
        );

        // @step And the profile's baseUrl and apiKey are unchanged
        let root: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let p = &root["providers"]["openai"]["profiles"]["work-vllm"];
        assert_eq!(p["baseUrl"], "http://localhost:8888");
        assert_eq!(p["apiKey"], "sk-test");
    }

    /// Scenario: Edit replaces the matching custom model in place
    #[test]
    fn edit_replaces_in_place_preserving_order() {
        // @step Given an "openai" profile "work-vllm" with custom models ["alpha", "beta"]
        let (_dir, path) = config_with(
            "work-vllm",
            r#","customModels":[{"id":"alpha"},{"id":"beta"}]"#,
        );

        // @step When I save a definition with id "alpha2" and original id "alpha"
        save_custom_model_at(&path, "openai", "work-vllm", &def("alpha2"), Some("alpha")).unwrap();

        // @step Then the profile's custom models are exactly ["alpha2", "beta"]
        assert_eq!(
            profile_custom_ids(&path, "work-vllm"),
            Some(vec!["alpha2".to_string(), "beta".to_string()])
        );
    }

    /// Scenario: Delete one of several custom models
    #[test]
    fn delete_one_of_several() {
        // @step Given an "openai" profile "work-vllm" with custom models ["alpha", "beta"]
        let (_dir, path) = config_with(
            "work-vllm",
            r#","customModels":[{"id":"alpha"},{"id":"beta"}]"#,
        );

        // @step When I delete the custom model "alpha"
        delete_custom_model_at(&path, "openai", "work-vllm", "alpha").unwrap();

        // @step Then the profile's custom models are exactly ["beta"]
        assert_eq!(
            profile_custom_ids(&path, "work-vllm"),
            Some(vec!["beta".to_string()])
        );
    }

    /// Scenario: Deleting the last custom model removes the customModels key
    #[test]
    fn delete_last_removes_key() {
        // @step Given an "openai" profile "work-vllm" with custom models ["alpha"]
        let (_dir, path) = config_with("work-vllm", r#","customModels":[{"id":"alpha"}]"#);

        // @step When I delete the custom model "alpha"
        delete_custom_model_at(&path, "openai", "work-vllm", "alpha").unwrap();

        // @step Then the profile has no customModels key
        assert_eq!(profile_custom_ids(&path, "work-vllm"), None);
    }

    /// Scenario: Unrelated config is preserved on save
    #[test]
    fn unrelated_config_is_preserved() {
        // @step Given a config with "openai" profiles "work-vllm" and "home" and a top-level "theme" key
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("fspec-config.json");
        fs::write(
            &path,
            r#"{"theme":"dark","providers":{"openai":{"profiles":{"work-vllm":{"baseUrl":"http://a","apiKey":"k1"},"home":{"baseUrl":"http://b","apiKey":"k2"}}}}}"#,
        )
        .unwrap();

        // @step When I save a new custom model "my-model" to "work-vllm" in add mode
        save_custom_model_at(&path, "openai", "work-vllm", &def("my-model"), None).unwrap();

        let root: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();

        // @step Then the "home" profile is unchanged
        let home = &root["providers"]["openai"]["profiles"]["home"];
        assert_eq!(home["baseUrl"], "http://b");
        assert_eq!(home["apiKey"], "k2");
        assert!(home.get("customModels").is_none());

        // @step And the top-level "theme" key is unchanged
        assert_eq!(root["theme"], "dark");
    }

    /// Scenario: Saving to a missing profile is a no-op
    #[test]
    fn saving_to_missing_profile_is_noop() {
        // @step Given an "openai" profile "work-vllm" exists
        let (_dir, path) = config_with("work-vllm", "");
        let before = fs::read_to_string(&path).unwrap();

        // @step When I save a custom model to the profile "does-not-exist"
        save_custom_model_at(&path, "openai", "does-not-exist", &def("x"), None).unwrap();

        // @step Then the config file is unchanged
        let after = fs::read_to_string(&path).unwrap();
        assert_eq!(before, after);
    }

    /// Scenario: Full definition round-trips through save and reload
    #[test]
    fn full_definition_round_trips() {
        // @step Given an "openai" profile "work-vllm" with no custom models
        let (_dir, path) = config_with("work-vllm", "");

        // @step When I save a custom model with every field set
        let full = CustomModelDef {
            id: "full-model".to_string(),
            display_name: Some("Full Model".to_string()),
            facade: Some("gemini".to_string()),
            context_window: Some(1_048_576),
            max_output_tokens: Some(65_536),
            compaction_threshold: Some(CompactionThreshold {
                threshold_type: "percentage".to_string(),
                value: 80,
            }),
            reasoning: Some(true),
            has_vision: Some(true),
        };
        save_custom_model_at(&path, "openai", "work-vllm", &full, None).unwrap();

        // @step And I reload the local-server profiles
        let profiles = load_local_server_profiles_from(&path);

        // @step Then the reloaded profile contains a matching custom model definition
        let profile = profiles.iter().find(|p| p.name == "work-vllm").unwrap();
        assert_eq!(profile.custom_models, vec![full]);
    }

    /// Scenario: A custom model with a float compaction value still loads
    #[test]
    fn float_compaction_value_still_loads() {
        // @step Given an "openai" profile "work-vllm" whose custom model stores compactionThreshold.value as the float 80.0
        let (_dir, path) = config_with(
            "work-vllm",
            r#","customModels":[{"id":"m","compactionThreshold":{"type":"percentage","value":80.0}}]"#,
        );

        // @step When I reload the local-server profiles
        let profiles = load_local_server_profiles_from(&path);

        // @step Then the profile loads and the custom model's compaction value is 80
        let profile = profiles.iter().find(|p| p.name == "work-vllm").unwrap();
        let threshold = profile.custom_models[0]
            .compaction_threshold
            .as_ref()
            .unwrap();
        assert_eq!(threshold.value, 80);
    }

    /// Scenario: A stored display name is surfaced when merging custom models
    #[test]
    fn stored_display_name_is_surfaced_on_merge() {
        // @step Given a custom model stored with displayName "My Model" and another with no stored display name
        let custom = vec![
            CustomModelDef {
                id: "with-name".to_string(),
                display_name: Some("My Model".to_string()),
                facade: None,
                context_window: None,
                max_output_tokens: None,
                compaction_threshold: None,
                reasoning: Some(true),
                has_vision: None,
            },
            CustomModelDef {
                id: "no-name".to_string(),
                display_name: None,
                facade: None,
                context_window: None,
                max_output_tokens: None,
                compaction_threshold: None,
                reasoning: None,
                has_vision: None,
            },
        ];

        // @step When the custom models are merged into wire model rows
        let entries = merge_profile_models(Vec::new(), &custom, 128_000);

        // @step Then the first row's display name is "My Model" and the second falls back to its id
        let with_name = entries.iter().find(|e| e.id == "with-name").unwrap();
        let no_name = entries.iter().find(|e| e.id == "no-name").unwrap();
        assert_eq!(with_name.display_name, "My Model");
        assert!(with_name.supports_reasoning);
        assert_eq!(no_name.display_name, "no-name");
    }
}
