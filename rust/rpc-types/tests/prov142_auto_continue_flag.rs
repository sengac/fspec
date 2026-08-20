// Feature: spec/features/profile-auto-continue-wire-schema.feature
//
// PROV-142 — the per-profile `auto_continue: Option<u32>` field on the
// wire-portable `ProfileDefinition` plus its canonical
// `auto_continue_enabled()` helper (None or Some(0) ⇒ off; Some(n≥1) ⇒ on).
//
// Each Gherkin step maps to a `// @step` comment immediately above the
// code that exercises it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_rpc_types::ProfileDefinition;

fn def_with_auto_continue(auto_continue: Option<u32>) -> ProfileDefinition {
    ProfileDefinition {
        base_url: "http://localhost:8888".to_string(),
        api_key: "sk-test".to_string(),
        auto_continue,
        ..ProfileDefinition::default()
    }
}

/// Scenario: The wire autoContinue predicate reports enabled only for positive budgets
#[test]
fn wire_auto_continue_predicate_reports_enabled_only_for_positive_budgets() {
    // @step Given a wire profile definition whose autoContinue value is 300
    let on = def_with_auto_continue(Some(300));

    // @step When the auto-continue enabled predicate is evaluated
    let on_enabled = on.auto_continue_enabled();

    // @step Then it reports enabled
    assert!(on_enabled, "Some(300) must report auto-continue enabled");

    // @step And a wire profile definition whose autoContinue value is 0 reports disabled
    let zero = def_with_auto_continue(Some(0));
    assert!(
        !zero.auto_continue_enabled(),
        "Some(0) must report auto-continue disabled (the OFF sentinel)"
    );

    // @step And a wire profile definition with no autoContinue value reports disabled
    let absent = def_with_auto_continue(None);
    assert!(
        !absent.auto_continue_enabled(),
        "absent autoContinue must report auto-continue disabled (today's behavior)"
    );
}

/// Scenario: The wire autoContinue value round-trips through JSON
#[test]
fn wire_auto_continue_value_round_trips_through_json() {
    // @step Given a wire profile definition whose autoContinue value is 300
    let on = def_with_auto_continue(Some(300));

    // @step When the definition is serialized to JSON and back
    let json = serde_json::to_string(&on).expect("serialize");
    let back: ProfileDefinition = serde_json::from_str(&json).expect("deserialize");

    // @step Then the autoContinue key carries 300
    // The serde wire name is snake_case (`auto_continue`); the camelCase
    // `autoContinue` key is the on-disk / NAPI form (covered by the
    // persistence feature). The round-trip must preserve the value.
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(
        value.get("auto_continue"),
        Some(&serde_json::json!(300)),
        "the auto_continue wire key must carry 300"
    );
    assert_eq!(back.auto_continue, Some(300));

    // @step And a legacy config file without the autoContinue key deserializes as absent
    // A wire shape predating PROV-142 (no auto_continue key) must still
    // deserialize, with the new field defaulting to `None`.
    let legacy = serde_json::json!({
        "base_url": "http://h",
        "api_key": "k"
    });
    let legacy_def: ProfileDefinition =
        serde_json::from_value(legacy).expect("legacy config must deserialize");
    assert_eq!(legacy_def.auto_continue, None);
}
