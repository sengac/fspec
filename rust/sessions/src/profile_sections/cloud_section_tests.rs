//! Feature: spec/features/model-selector-cloud-section-assembly.feature
//!
//! Moved out of tests.rs (COV-075: one test file per feature).

use super::{build_profile_provider_info, cloud_provider_info};
use codelet_rpc_types::ModelEntry;

fn model(id: &str, custom: bool) -> ModelEntry {
    ModelEntry {
        id: id.to_string(),
        display_name: id.to_string(),
        context_window: Some(128_000),
        supports_reasoning: false,
        supports_vision: false,
        is_custom: custom,
    }
}

// ====================================================================
// PROV-127 — drop empty/uncredentialed cloud sections (TS parity).
// Feature: spec/features/model-selector-cloud-section-assembly.feature
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
