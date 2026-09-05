//! PROV-145 — the per-profile loop-detection fields on the wire-portable
//! `ProfileDefinition` plus their canonical predicates:
//! `loop_detection_enabled()` (absent ⇒ true, today's always-on behavior),
//! `loop_detection_window()` (absent ⇒ 160), `loop_detection_max_repeats()`
//! (absent ⇒ 10), `loop_detection_max_retries()` (absent ⇒ 10).
//!
//! Feature: spec/features/per-profile-loop-detection-wire-schema.feature
//!
//! Each Gherkin step maps to a `// @step` comment immediately above the
//! code that exercises it. These tests cover the WIRE half: the four values
//! round-trip through the wire JSON shape and the canonical predicates
//! resolve the effective defaults. The on-disk `fspec-config.json` half is
//! covered by `rust/sessions/tests/prov145_loop_detection_persistence.rs`.
//!
//! RED PHASE: `ProfileDefinition` has no `loop_detection_*` fields and no
//! `loop_detection_*()` predicates yet, so this target fails to compile
//! until the implementation lands.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_rpc_types::ProfileDefinition;

/// Build a definition with only the loop-detection fields set.
fn def_loop(
    enabled: Option<bool>,
    window: Option<u32>,
    max_repeats: Option<u32>,
    max_retries: Option<u32>,
) -> ProfileDefinition {
    ProfileDefinition {
        base_url: "http://localhost:8888".to_string(),
        api_key: "sk-test".to_string(),
        loop_detection_enabled: enabled,
        loop_detection_window: window,
        loop_detection_max_repeats: max_repeats,
        loop_detection_max_retries: max_retries,
        ..ProfileDefinition::default()
    }
}

/// Scenario: The loop-detection values round-trip through the wire JSON shape
#[test]
fn the_loop_detection_values_round_trip_through_the_wire_json_shape() {
    // @step Given a profile definition with loopDetectionEnabled true, loopDetectionWindow 320, loopDetectionMaxRepeats 5, loopDetectionMaxRetries 2
    let def = def_loop(Some(true), Some(320), Some(5), Some(2));

    // @step When the profile definition is serialized to its wire JSON form
    let json = serde_json::to_string(&def).expect("serialize");

    // @step Then the JSON carries the loop_detection_enabled, loop_detection_window, loop_detection_max_repeats, and loop_detection_max_retries values
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(
        value.get("loop_detection_enabled"),
        Some(&serde_json::json!(true)),
        "the wire key loop_detection_enabled must carry true"
    );
    assert_eq!(
        value.get("loop_detection_window"),
        Some(&serde_json::json!(320)),
        "the wire key loop_detection_window must carry 320"
    );
    assert_eq!(
        value.get("loop_detection_max_repeats"),
        Some(&serde_json::json!(5)),
        "the wire key loop_detection_max_repeats must carry 5"
    );
    assert_eq!(
        value.get("loop_detection_max_retries"),
        Some(&serde_json::json!(2)),
        "the wire key loop_detection_max_retries must carry 2"
    );

    // @step And re-deserializing the JSON yields the same four values
    let back: ProfileDefinition = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.loop_detection_enabled, Some(true));
    assert_eq!(back.loop_detection_window, Some(320));
    assert_eq!(back.loop_detection_max_repeats, Some(5));
    assert_eq!(back.loop_detection_max_retries, Some(2));

    // @step And the canonical predicates resolve the effective values to 320, 5, 2 and enabled
    assert!(back.loop_detection_enabled());
    assert_eq!(back.loop_detection_window(), 320);
    assert_eq!(back.loop_detection_max_repeats(), 5);
    assert_eq!(back.loop_detection_max_retries(), 2);

    // A legacy config file without the keys must still deserialize, with
    // every field defaulting to None.
    let legacy: ProfileDefinition = serde_json::from_value(serde_json::json!({
        "base_url": "http://h",
        "api_key": "k"
    }))
    .expect("legacy config must deserialize");
    assert_eq!(legacy.loop_detection_enabled, None);
    assert_eq!(legacy.loop_detection_window, None);
    assert_eq!(legacy.loop_detection_max_repeats, None);
    assert_eq!(legacy.loop_detection_max_retries, None);
}

/// Scenario: An absent loopDetectionEnabled key resolves to enabled
#[test]
fn an_absent_loop_detection_enabled_key_resolves_to_enabled() {
    // @step Given a legacy config file without the loopDetectionEnabled key
    let legacy: ProfileDefinition = serde_json::from_value(serde_json::json!({
        "base_url": "http://h",
        "api_key": "k"
    }))
    .expect("legacy config must deserialize");

    // @step When it is deserialized into a profile definition
    // (legacy is the deserialized definition — the wire key
    // loop_detection_enabled is absent)
    assert_eq!(
        legacy.loop_detection_enabled, None,
        "the wire field must be None for a legacy config"
    );

    // @step Then the loop_detection_enabled field is absent
    // @step And the canonical enabled predicate resolves to true (today's always-on behavior)
    assert!(
        legacy.loop_detection_enabled(),
        "absent key ⇒ the detector stays ON (today's behavior)"
    );
}

/// Scenario: An explicit loopDetectionEnabled false resolves to disabled
#[test]
fn an_explicit_loop_detection_enabled_false_resolves_to_disabled() {
    // @step Given a profile definition with loopDetectionEnabled false
    let def = def_loop(Some(false), None, None, None);

    // @step When the canonical enabled predicate is applied
    // @step Then it resolves to false
    assert!(!def.loop_detection_enabled());
}

/// Scenario: Absent numeric keys resolve to the RIG-014 defaults
#[test]
fn absent_numeric_keys_resolve_to_the_rig_014_defaults() {
    // @step Given a profile definition without the loop-detection numeric fields
    let def = def_loop(None, None, None, None);

    // @step When the canonical predicates are applied
    let window = def.loop_detection_window();
    let repeats = def.loop_detection_max_repeats();
    let retries = def.loop_detection_max_retries();

    // @step Then the effective window resolves to 160
    assert_eq!(window, 160, "absent key ⇒ default window 160");

    // @step And the effective maxRepeats resolves to 10
    assert_eq!(repeats, 10, "absent key ⇒ default repeat threshold 10");

    // @step And the effective maxRetries resolves to 10
    assert_eq!(retries, 10, "absent key ⇒ default retry cap 10");
}

/// A stored (explicit) numeric value is never coerced to the default.
#[test]
fn a_stored_numeric_value_wins_over_the_default() {
    // @step Given a profile definition with the numeric fields stored
    let def = def_loop(None, Some(40), Some(2), Some(0));

    // @step When the canonical predicates are applied
    assert_eq!(def.loop_detection_window(), 40);
    assert_eq!(def.loop_detection_max_repeats(), 2);

    // @step Then the effective values are the stored values (including the explicit 0 retry sentinel)
    assert_eq!(
        def.loop_detection_max_retries(),
        0,
        "an explicit 0 must stay 0 (never auto-retry), not the default"
    );
}
