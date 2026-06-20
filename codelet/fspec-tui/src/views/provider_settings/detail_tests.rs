//! RPC-161 — branch-coverage unit tests for the printable-ASCII helper.
//!
//! Feature: spec/features/rpc161-provider-settings-api-key-printable-ascii-filter.feature
//!
//! Covers the same Gherkin scenario as the end-to-end integration test
//! `is_printable_ascii_helper_classifies_characters_by_ascii_code_observed_end_to_end`
//! in tests/provider_settings_api_key_charset_rpc161.rs, but exercises
//! the helper directly so a regression that changed the range bounds
//! without changing observable view state would still fail.
//!
//! Split into a `#[path]`-included sibling so the parent stays under the
//! 300-LoC source-shape ceiling while keeping canonical rustfmt formatting.

use super::is_printable_ascii;

/// Scenario: is_printable_ascii() helper classifies characters by ASCII code
#[test]
fn is_printable_ascii_returns_true_for_inclusive_boundaries_and_interior() {
    // @step Given the helper function is_printable_ascii(c: char) -> bool exists in views/provider_settings/detail.rs
    // @step When the helper is called with each of the chars ' ' (32), 'A' (65), '~' (126)
    // @step Then it returns true for every one
    assert!(is_printable_ascii(' '), "space (32) must be accepted");
    assert!(
        is_printable_ascii('A'),
        "interior char 'A' (65) must be accepted"
    );
    assert!(is_printable_ascii('~'), "tilde (126) must be accepted");
}

#[test]
fn is_printable_ascii_returns_false_for_control_del_and_non_ascii() {
    // @step When the helper is called with each of the chars '\t' (9), '\u{001F}' (31), '\u{007F}' (127), 'é' (233), '🔑' (128017)
    // @step Then it returns false for every one
    assert!(!is_printable_ascii('\t'), "tab (9) must be rejected");
    assert!(
        !is_printable_ascii('\u{001F}'),
        "unit-separator (31) must be rejected"
    );
    assert!(
        !is_printable_ascii('\u{007F}'),
        "DEL (127) must be rejected"
    );
    assert!(!is_printable_ascii('é'), "latin-1 é (233) must be rejected");
    assert!(
        !is_printable_ascii('🔑'),
        "non-BMP emoji (128017) must be rejected"
    );
}
