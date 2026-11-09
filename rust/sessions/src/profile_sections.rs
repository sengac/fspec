//! RPC-338 — local-server profile sections + reachability mapping for the
//! wire `list_providers()` surface.
//!
//! Feature: spec/features/model-selector-profile-server.feature
//!
//! Ports the decision logic of the TS `profileSectionBuilder.ts`
//! (`loadProfileSections`) into the Rust server: build a wire
//! `ProviderInfo` for each local-server profile, applying the BUG-218 narrowed
//! MODEL-004 override (unsupported discovery never proves inference unreachable;
//! configured models remain usable; other probe failures block selection), and tag cloud /
//! custom providers with `profile_name = None`, `is_unreachable = false`.
//!
//! The pure mapping helpers below are unit-tested without network I/O;
//! `handle_impl::list_providers` wires the real probe
//! (`OpenAIProvider::list_local_models_with_auth`) into them.

pub use crate::inherited_profile_sections::build_inherited_profile_sections;
use codelet_rpc_types::{ModelEntry, ProviderInfo};

#[path = "profile_discovery.rs"]
mod discovery;
use discovery::probe_profile_models;

mod config;
mod config_io;
mod custom_models;

pub use config::{CompactionThreshold, CustomModelDef, LocalServerProfile};
pub use config_io::load_local_server_profiles;
pub(crate) use config_io::read_config_value;
pub use custom_models::{delete_custom_model, save_custom_model};

/// Build the wire `ProviderInfo` for a cloud / custom provider. Such
/// providers are never profile sections and are always treated as
/// reachable (`profile_name = None`, `is_unreachable = false`); the section
/// key is the provider id itself.
pub fn cloud_provider_info(key: &str, display_name: &str, models: Vec<ModelEntry>) -> ProviderInfo {
    ProviderInfo {
        key: key.to_string(),
        display_name: display_name.to_string(),
        models,
        profile_name: None,
        is_unreachable: false,
        provider_id: key.to_string(),
        unavailable_reason: None,
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
        // PROV-176: a section with a reason for listing nothing is shown.
        .filter(|section| {
            section.profile_name.is_some()
                || !section.models.is_empty()
                || section.unavailable_reason.is_some()
        })
        .collect()
}

/// Build the wire `ProviderInfo` for a local-server profile section.
///
/// Ports `profileSectionBuilder.ts`:
/// * `key` is the profile-qualified `"{provider}:{profile}"` slug.
/// * `display_name` is the profile-qualified label `"{provider}: {profile}"`.
/// * `profile_name` is always `Some(profile)`.
/// * `provider_id` is the base provider (BUG-325) — what clients send as the
///   `provider_id` of custom-model writes, never the section `key`.
/// * The section is `is_unreachable` when the probe reports failure after
///   applying the BUG-218 optional-discovery exception for custom models.
pub fn build_profile_provider_info(
    provider_id: &str,
    profile_name: &str,
    models: Vec<ModelEntry>,
    probe_failed: bool,
) -> ProviderInfo {
    ProviderInfo {
        key: format!("{provider_id}:{profile_name}"),
        display_name: format!("{provider_id}: {profile_name}"),
        models,
        profile_name: Some(profile_name.to_string()),
        is_unreachable: probe_failed,
        provider_id: provider_id.to_string(),
        unavailable_reason: None,
    }
}

/// Merge the probe-discovered model ids with the profile's declared custom
/// models into wire `ModelEntry` rows (ports `mergeCustomModels`). Custom
/// models are flagged `is_custom = true`; probe models `false`. Duplicate ids
/// (a custom override of a discovered model) keep the custom flag.
pub fn merge_profile_models(
    discovered: Vec<String>,
    custom: &[CustomModelDef],
    context_window: Option<u32>,
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
    for c in custom.iter().filter(|model| !model.id.trim().is_empty()) {
        entries.push(ModelEntry {
            id: c.id.clone(),
            // Parity with TS `custom.displayName || custom.id`: fall back to
            // the id only when no display name is stored, instead of always
            // overwriting it with the id (RPC-346/RPC-344 prefill fix).
            display_name: c.display_name.clone().unwrap_or_else(|| c.id.clone()),
            // PROV-169: the model's own value, else the profile's, else none.
            context_window: c.context_window.or(context_window),
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
/// PROV-175: a profile config that cannot be loaded is an error.
pub fn build_local_profile_sections() -> Result<Vec<ProviderInfo>, String> {
    let profiles = load_local_server_profiles()?;
    if profiles.is_empty() {
        return Ok(Vec::new());
    }
    Ok(profiles
        .into_iter()
        .map(|profile| {
            // PROV-169: no guessed 128k — an undeclared window stays None.
            let (discovered, probe_failed) = probe_profile_models(&profile);
            let models =
                merge_profile_models(discovered, &profile.custom_models, profile.context_window);
            build_profile_provider_info("openai", &profile.name, models, probe_failed)
        })
        .collect())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod cloud_section_tests;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod persistence_tests;
