//! PROV-144 — the per-profile `max_images: Option<u32>` field on the
//! wire-portable `ProfileDefinition` plus its canonical
//! `max_images_limit()` predicate (absent ⇒ default 4; `Some(n)` ⇒ n,
//! including the `Some(0)` no-vision sentinel).
//!
//! Feature: spec/features/per-profile-max-images-wire-schema.feature
//!
//! Each Gherkin step maps to a `// @step` comment immediately above the
//! code that exercises it. These tests cover the WIRE half of the two
//! persistence scenarios: the `maxImages` value round-trips through the
//! wire JSON shape (the on-disk `fspec-config.json` half is covered by
//! `rust/sessions/tests/prov144_max_images_persistence.rs`).
//!
//! GREEN PHASE: `ProfileDefinition::max_images` and
//! `ProfileDefinition::max_images_limit` are defined in
//! `codelet-rpc-types`; this target compiles and passes once the
//! predicate's absent-⇒-4 and 0-is-0 semantics are in place.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_rpc_types::ProfileDefinition;

fn def_with_max_images(max_images: Option<u32>) -> ProfileDefinition {
    ProfileDefinition {
        base_url: "http://localhost:8888".to_string(),
        api_key: "sk-test".to_string(),
        max_images,
        ..ProfileDefinition::default()
    }
}

/// Scenario: The maxImages value round-trips through wire and disk
#[test]
fn the_max_images_value_round_trips_through_wire_and_disk() {
    // @step Given a profile definition with maxImages 7
    let def = def_with_max_images(Some(7));

    // @step When the profile is saved to fspec-config.json
    // (the wire JSON form of the saved profile — the snake_case wire key)
    let json = serde_json::to_string(&def).expect("serialize");

    // @step Then the stored profile object contains "maxImages": 7
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(
        value.get("max_images"),
        Some(&serde_json::json!(7)),
        "the max_images wire key must carry 7"
    );

    // @step And re-reading the profile resolves the effective limit to 7
    let back: ProfileDefinition = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.max_images, Some(7));
    assert_eq!(
        back.max_images_limit(),
        7,
        "re-read must resolve the effective limit to 7"
    );

    // A legacy config file without the maxImages key must still deserialize,
    // with the new field defaulting to `None` (⇒ default 4) and an explicit
    // 0 must stay 0 (the no-vision sentinel is never coerced to the default).
    let legacy: ProfileDefinition = serde_json::from_value(serde_json::json!({
        "base_url": "http://h",
        "api_key": "k"
    }))
    .expect("legacy config must deserialize");
    assert_eq!(legacy.max_images, None);
    assert_eq!(
        legacy.max_images_limit(),
        4,
        "absent key ⇒ effective default 4"
    );
    assert_eq!(
        def_with_max_images(Some(0)).max_images_limit(),
        0,
        "an explicit 0 must resolve to 0 (no vision), not the default"
    );
}

/// Scenario: A missing maxImages key resolves to the default 4
///
/// The wire test covers the wire-JSON round-trip half of this scenario:
/// a `None` `max_images` round-trips as `None`, and the canonical
/// `max_images_limit()` predicate resolves the effective default of 4.
/// The on-disk "no maxImages key" half is covered by
/// `rust/sessions/tests/prov144_max_images_persistence.rs` (the
/// `set_or_remove` persistence path is what actually drops the key from
/// `fspec-config.json`; the wire `ProfileDefinition` has no
/// `skip_serializing_if`, matching the sibling `auto_continue` /
/// `streaming` fields).
#[test]
fn a_missing_max_images_key_resolves_to_the_default_4() {
    // @step Given a profile definition without a maxImages field
    let def = def_with_max_images(None);

    // @step When the profile is saved to fspec-config.json
    // (the wire JSON form of the saved profile)
    let json = serde_json::to_string(&def).expect("serialize");

    // @step Then the stored profile object has no maxImages key
    // (wire round-trip: the deserialized field is `None`; the on-disk
    // key-absence is the persistence layer's responsibility, asserted in
    // prov144_max_images_persistence.rs)
    let back: ProfileDefinition = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(
        back.max_images, None,
        "a None maxImages must round-trip as None through the wire JSON"
    );

    // @step And re-reading the profile resolves the effective limit to 4
    assert_eq!(back.max_images_limit(), 4, "absent key ⇒ effective limit 4");
}
