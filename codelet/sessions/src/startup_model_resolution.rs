//! PROV-120 — startup first-available model resolution (TS parity).
//!
//! Feature: spec/features/startup-model-initialization-first-available.feature
//!
//! Ports the resolution chain of the TypeScript
//! `src/tui/services/modelInitializationService.ts`
//! (`initializeModels` / `selectDefaultModel` / `restorePersistedModel`) into a
//! pure, network-free function: given the (display-order) provider sections and
//! an optional persisted model string, resolve the startup default model as
//! **restore persisted → else first-available reachable model**.
//!
//! Selection ordering (rule 2): for SELECTION the sections are reordered
//! profiles → custom → cloud, independently of the `list_providers` DISPLAY
//! order (which is left untouched so the PROV-101 selector behaviour does not
//! change). A section is eligible only when it has at least one model — that
//! is the approved proxy for "reachable + credentialed" given the wire
//! `ProviderInfo` carries no `has_credentials` field.
//!
//! Implements the resolution chain in pure Rust (no network / filesystem):
//! reorder the sections for SELECTION (profiles → custom → cloud), drop
//! unreachable / zero-model sections, restore the persisted model when it
//! still matches a model-bearing section, otherwise pick the first-available
//! reachable model — and NEVER substitute a hardcoded anthropic/claude default.

use codelet_rpc_types::ProviderInfo;

use crate::startup_model_utils::{
    build_model_string, classify_for_selection, parse_persisted, reorder_for_selection,
    SectionClass,
};

/// The resolved startup model, expressed as the composite model string that is
/// handed to `set_default_model` (e.g. `"anthropic/claude-opus-4"` for a cloud
/// section or `"openai:work-vllm/Qwen3-80B"` for a profile section). Mirrors
/// the TS `buildModelString` output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedStartupModel {
    /// Composite `provider/model` (or `provider:profile/model`) string.
    pub model_string: String,
}

/// Resolve the startup default model from the provider sections and the
/// optional persisted model string.
///
/// Resolution order (TS parity): restore the persisted model if it matches a
/// model-bearing section, otherwise the first-available reachable model in
/// selection order (profiles → custom → cloud). Returns `None` when there are
/// no model-bearing sections (the genuine zero-models edge case; PROV-101
/// decline preserved) — and NEVER substitutes a hardcoded anthropic/claude
/// default.
#[must_use]
pub fn resolve_startup_model(
    sections: &[ProviderInfo],
    persisted: Option<&str>,
) -> Option<ResolvedStartupModel> {
    // SELECTION ordering (rule 2): reorder profiles → custom → cloud WITHOUT
    // mutating the caller's list, then keep only eligible (reachable +
    // model-bearing) sections. A section with >= 1 model is the approved proxy
    // for "reachable + credentialed"; an `is_unreachable` section is excluded.
    let ordered = reorder_for_selection(sections);
    let eligible: Vec<&ProviderInfo> = ordered
        .into_iter()
        .filter(|s| !s.is_unreachable && !s.models.is_empty())
        .collect();

    // Restore: a persisted Some that still matches an eligible section
    // containing that exact model id wins over first-available.
    if let Some(persisted) = persisted {
        if let Some((provider_id, profile_name, model_id)) = parse_persisted(persisted) {
            for section in &eligible {
                let matches_section = match (&profile_name, classify_for_selection(section)) {
                    // Profile selection: match BOTH provider id AND profile name.
                    (Some(profile), SectionClass::Profile) => {
                        section.profile_name.as_deref() == Some(profile.as_str())
                            && section_provider_id(section) == provider_id
                    }
                    // Cloud / custom selection: match the section key (provider
                    // id) AND the section must NOT be a profile section.
                    (None, SectionClass::Cloud) => section.key == provider_id,
                    _ => false,
                };
                if matches_section && section.models.iter().any(|m| m.id == model_id) {
                    return Some(ResolvedStartupModel {
                        model_string: persisted.to_string(),
                    });
                }
            }
        }
    }

    // First-available: the first eligible section's first model.
    let section = eligible.first()?;
    let model = section.models.first()?;
    Some(ResolvedStartupModel {
        model_string: build_model_string(section, &model.id),
    })
}

/// Extract the bare provider id from a profile section's `"{provider}:{profile}"`
/// key (everything before the first `:`), falling back to the whole key when no
/// colon is present.
fn section_provider_id(section: &ProviderInfo) -> &str {
    section
        .key
        .split_once(':')
        .map_or(section.key.as_str(), |(provider, _)| provider)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "startup_model_resolution_tests.rs"]
mod tests;
