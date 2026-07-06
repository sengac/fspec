// Feature: spec/features/provider-settings-profile-streaming.feature
//
// PROV-139 — the per-profile `streaming: Option<bool>` flag on the
// wire-portable `ProfileDefinition` plus its canonical
// `streaming_enabled()` helper (absent ⇒ enabled).
//
// Each Gherkin step maps to a `// @step` comment immediately above the
// code that exercises it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_rpc_types::ProfileDefinition;

/// Scenario: Absent streaming flag is treated as enabled
#[test]
fn absent_streaming_flag_is_treated_as_enabled() {
    // @step Given a profile definition whose streaming flag is not set
    let def = ProfileDefinition {
        base_url: "http://localhost:8888".to_string(),
        api_key: "sk-test".to_string(),
        streaming: None,
        ..ProfileDefinition::default()
    };

    // @step When the streaming_enabled helper is evaluated
    let enabled = def.streaming_enabled();

    // @step Then it reports streaming as enabled
    assert!(enabled, "absent streaming flag must report enabled");
}

/// Scenario: An explicit streaming flag is echoed by the helper
#[test]
fn explicit_streaming_flag_is_echoed_by_the_helper() {
    // @step Given a profile definition whose streaming flag is set to disabled
    let def = ProfileDefinition {
        base_url: "http://localhost:8888".to_string(),
        api_key: "sk-test".to_string(),
        streaming: Some(false),
        ..ProfileDefinition::default()
    };

    // @step When the streaming_enabled helper is evaluated
    let enabled = def.streaming_enabled();

    // @step Then it reports streaming as disabled
    assert!(!enabled, "explicit Some(false) must report disabled");
}
