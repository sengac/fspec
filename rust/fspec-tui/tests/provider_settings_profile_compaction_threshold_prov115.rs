//! PROV-115 — profile compaction-threshold range-validation parity.
//!
//! Feature: spec/features/provider-settings-profile-compaction-threshold.feature
//!
//! Offline pure-state tests: build a `ProfileForm` with a valid base_url /
//! api_key / name, drive `compaction_threshold` to each input, call
//! `build_definition()` and assert the resulting
//! `compaction_threshold_type` / `compaction_threshold_value`. Mirrors TS
//! `parseCompactionThreshold` range rules (MIN_PERCENTAGE=1, MAX_PERCENTAGE=100,
//! MIN_TOKEN_THRESHOLD=1000): out-of-range / empty / non-numeric → unset.
//!
//! No App, no backend, no filesystem.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::provider_settings::profile_form::ProfileForm;

/// A valid create form (base URL, API key and a trimmed name present) with the
/// compaction-threshold field set to `threshold`.
fn form_with_threshold(threshold: &str) -> ProfileForm {
    let mut form = ProfileForm::new_create();
    form.is_editing_name = false;
    form.name = "openai-profile".to_string();
    form.base_url = "https://api.openai.com/v1".to_string();
    form.api_key = "sk-valid".to_string();
    form.compaction_threshold = threshold.to_string();
    form
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A below-minimum percentage is treated as unset
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn below_minimum_percentage_is_unset() {
    // @step Given a profile create form with a valid base URL, API key and name
    // @step And the compaction trigger field contains "0%"
    let form = form_with_threshold("0%");

    // @step When the profile is saved
    let def = form
        .build_definition()
        .expect("a valid profile must still build");

    // @step Then the saved profile definition has no compaction threshold type
    assert_eq!(
        def.compaction_threshold_type, None,
        "0% is below MIN_PERCENTAGE (1) and must be unset"
    );
    // @step And the saved profile definition has no compaction threshold value
    assert_eq!(def.compaction_threshold_value, None);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Percentage boundaries 1 and 100 are accepted
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn percentage_boundaries_one_and_hundred_are_accepted() {
    // @step Given a profile create form with a valid base URL, API key and name
    // @step When the compaction trigger field contains "1%" and the profile is saved
    let def = form_with_threshold("1%")
        .build_definition()
        .expect("valid profile builds");

    // @step Then the saved profile definition has compaction threshold type "percentage" and value 1
    assert_eq!(def.compaction_threshold_type.as_deref(), Some("percentage"));
    assert_eq!(def.compaction_threshold_value, Some(1));

    // @step When the compaction trigger field contains "100%" and the profile is saved
    let def = form_with_threshold("100%")
        .build_definition()
        .expect("valid profile builds");

    // @step Then the saved profile definition has compaction threshold type "percentage" and value 100
    assert_eq!(def.compaction_threshold_type.as_deref(), Some("percentage"));
    assert_eq!(def.compaction_threshold_value, Some(100));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: An above-maximum percentage is omitted but the profile still saves
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn above_maximum_percentage_is_omitted_but_profile_saves() {
    // @step Given a profile create form with a valid base URL, API key and name
    // @step And the compaction trigger field contains "101%"
    let form = form_with_threshold("101%");

    // @step When the profile is saved
    let def = form.build_definition();

    // @step Then the profile is saved successfully
    let def = def.expect("an out-of-range threshold must NOT block saving the profile");

    // @step And the saved profile definition has no compaction threshold type
    assert_eq!(
        def.compaction_threshold_type, None,
        "101% is above MAX_PERCENTAGE (100) and must be unset"
    );
    // @step And the saved profile definition has no compaction threshold value
    assert_eq!(def.compaction_threshold_value, None);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A below-minimum token count is treated as unset
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn below_minimum_token_count_is_unset() {
    // @step Given a profile create form with a valid base URL, API key and name
    // @step When the compaction trigger field contains "999" and the profile is saved
    let def = form_with_threshold("999")
        .build_definition()
        .expect("valid profile builds");

    // @step Then the saved profile definition has no compaction threshold value
    assert_eq!(
        def.compaction_threshold_value, None,
        "999 is below MIN_TOKEN_THRESHOLD (1000) and must be unset"
    );
    assert_eq!(def.compaction_threshold_type, None);

    // @step When the compaction trigger field contains "1000" and the profile is saved
    let def = form_with_threshold("1000")
        .build_definition()
        .expect("valid profile builds");

    // @step Then the saved profile definition has compaction threshold type "tokens" and value 1000
    assert_eq!(def.compaction_threshold_type.as_deref(), Some("tokens"));
    assert_eq!(def.compaction_threshold_value, Some(1000));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Empty and non-numeric input remain unset
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn empty_and_non_numeric_input_remain_unset() {
    // @step Given a profile create form with a valid base URL, API key and name
    // @step When the compaction trigger field contains "" and the profile is saved
    let def = form_with_threshold("")
        .build_definition()
        .expect("valid profile builds");

    // @step Then the saved profile definition has no compaction threshold value
    assert_eq!(def.compaction_threshold_value, None);
    assert_eq!(def.compaction_threshold_type, None);

    // @step When the compaction trigger field contains "abc" and the profile is saved
    let def = form_with_threshold("abc")
        .build_definition()
        .expect("valid profile builds");

    // @step Then the saved profile definition has no compaction threshold value
    assert_eq!(def.compaction_threshold_value, None);
    assert_eq!(def.compaction_threshold_type, None);
}
