//! PROV-120 — pure helpers for startup model resolution (TS parity).
//!
//! Feature: spec/features/startup-model-initialization-first-available.feature
//!
//! Splits the section-classification / reorder / model-string / persisted-parse
//! logic out of [`crate::startup_model_resolution`] so the resolver stays a thin
//! wrapper and both files stay well under the 300-line limit. Ports:
//!   * `buildModelString` / `parseModelString` from
//!     `src/tui/utils/model-selection.ts`.
//!   * the `[...profileSections, ...customSections, ...cloudSections]` ordering
//!     from `initializeModels` (`modelInitializationService.ts`).

use codelet_rpc_types::ProviderInfo;

/// How a wire `ProviderInfo` classifies for SELECTION ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionClass {
    /// Local-server profile section (`key` contains `':'` OR `profile_name` is
    /// `Some`). Sorted first.
    Profile,
    /// Cloud / custom provider (built-in or custom). Sorted last; the wire
    /// `ProviderInfo` carries no `is_custom` flag, so custom and built-in
    /// cloud share one trailing bucket that preserves the caller's relative
    /// order (custom-before-cloud as produced by `list_providers`).
    Cloud,
}

/// Classify a section for SELECTION ordering. A section is a profile when its
/// `key` contains a `':'` (the `"{provider}:{profile}"` slug) OR it carries a
/// `profile_name`; everything else is a cloud/custom section. Mirrors the TS
/// split between `profileSections` and `cloudSections`.
#[must_use]
pub fn classify_for_selection(section: &ProviderInfo) -> SectionClass {
    if section.profile_name.is_some() || section.key.contains(':') {
        SectionClass::Profile
    } else {
        SectionClass::Cloud
    }
}

/// Reorder sections for SELECTION as profiles → custom → cloud WITHOUT mutating
/// the caller's slice (the `list_providers` DISPLAY order must be preserved).
/// Relative order within each bucket is preserved (stable partition). Ports the
/// TS `[...profileSections, ...customSections, ...cloudSections]` concatenation.
#[must_use]
pub fn reorder_for_selection(sections: &[ProviderInfo]) -> Vec<&ProviderInfo> {
    let mut profiles: Vec<&ProviderInfo> = Vec::new();
    let mut cloud: Vec<&ProviderInfo> = Vec::new();
    for section in sections {
        match classify_for_selection(section) {
            SectionClass::Profile => profiles.push(section),
            // Cloud/custom share the trailing bucket; relative order preserved.
            SectionClass::Cloud => cloud.push(section),
        }
    }
    profiles.extend(cloud);
    profiles
}

/// Build the composite model string handed to `set_default_model`. Ports the TS
/// `buildModelString`: a profile section yields `"{provider}:{profile}/{model}"`
/// and a cloud/custom section yields `"{provider}/{model}"`.
#[must_use]
pub fn build_model_string(section: &ProviderInfo, model_id: &str) -> String {
    match &section.profile_name {
        Some(profile) => {
            let provider = section
                .key
                .split_once(':')
                .map_or(section.key.as_str(), |(p, _)| p);
            format!("{provider}:{profile}/{model_id}")
        }
        None => format!("{}/{}", section.key, model_id),
    }
}

/// Parse a persisted model string into `(provider_id, profile_name, model_id)`.
/// Ports the TS `parseModelString`: when a `':'` precedes the first `'/'` the
/// string is the profile form `"{provider}:{profile}/{model}"`; otherwise it is
/// the cloud form `"{provider}/{model}"`. Model ids may contain `'/'`, so only
/// the FIRST `'/'` after the colon (or the first overall) splits the model id.
/// Returns `None` for a malformed string (no `'/'`, or empty profile/model).
#[must_use]
pub fn parse_persisted(model_string: &str) -> Option<(String, Option<String>, String)> {
    let colon = model_string.find(':');
    let first_slash = model_string.find('/')?;

    if let Some(colon) = colon {
        if colon < first_slash {
            // Profile form: provider:profile/model
            let provider = &model_string[..colon];
            let rest = &model_string[colon + 1..];
            let slash = rest.find('/')?;
            let profile = &rest[..slash];
            let model = &rest[slash + 1..];
            if provider.is_empty() || profile.is_empty() || model.is_empty() {
                return None;
            }
            return Some((
                provider.to_string(),
                Some(profile.to_string()),
                model.to_string(),
            ));
        }
    }

    // Cloud form: provider/model
    let provider = &model_string[..first_slash];
    let model = &model_string[first_slash + 1..];
    if provider.is_empty() || model.is_empty() {
        return None;
    }
    Some((provider.to_string(), None, model.to_string()))
}
