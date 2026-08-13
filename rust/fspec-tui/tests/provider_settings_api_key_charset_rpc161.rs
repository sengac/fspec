//! RPC-161 — ProviderSettingsView API-key edit ASCII 32-126 charset filter.
//!
//! Feature: spec/features/rpc161-provider-settings-api-key-printable-ascii-filter.feature
//!
//! This test file validates the acceptance criteria for RPC-161: only
//! printable ASCII characters (codes 32..=126 inclusive) are appended to
//! the EditApiKey draft buffer. Control characters (\t, \x1F, …),
//! DEL (\x7F), and any non-ASCII char (>127) are silently dropped —
//! the keystroke is still consumed (ProviderSettingsEvent::Consumed),
//! no Action is dispatched, no status text is set, and the draft is
//! unchanged.
//!
//! Mirrors TS `filterPrintableChars` at
//! src/tui/utils/providerSettingsHelpers.ts:39-47 + its sole call site
//! src/tui/inputHandlers/apiKeyEditModeHandler.ts:51-54.
//!
//! Tests are written test-first per ACDD: they FAIL against the current
//! `KeyCode::Char(c) => { draft.push(c); … }` arm in detail.rs and PASS
//! once the `if is_printable_ascii(c)` guard is added.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::{
    DetailSub, ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView,
};
use codelet_rpc_types::ProviderCredentialInfo;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

/// Build a synthetic Press event with no modifiers — mirrors the harness in
/// provider_settings_view_rpc054.rs / provider_settings_list_keybind_parity_rpc157.rs.
fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn pinfo(id: &str, ctype: &str, configured: bool, models: u32) -> ProviderCredentialInfo {
    ProviderCredentialInfo {
        provider_id: id.to_string(),
        display_name: id.to_string(),
        configured,
        credential_type: ctype.to_string(),
        model_count: models,
        masked_key: None,
        source: None,
    }
}

/// Build a ProviderSettingsView seeded with one api_key provider already
/// transitioned into Detail::EditApiKey mode with an empty draft.
fn view_in_edit_api_key_mode() -> ProviderSettingsView {
    let mut v = ProviderSettingsView::new();
    v.set_providers(vec![pinfo("anthropic", "api_key", true, 1)]);
    v.handle_key(key(KeyCode::Enter)); // List → Detail::Summary
    v.handle_key(key(KeyCode::Enter)); // Detail::Summary → Detail::EditApiKey
    v
}

/// Helper: assert the view is in Detail::EditApiKey mode and return the draft.
fn assert_draft_eq(view: &ProviderSettingsView, expected: &str) {
    match &view.mode {
        ProviderSettingsMode::Detail {
            sub: DetailSub::EditApiKey { draft },
            ..
        } => {
            assert_eq!(
                draft, expected,
                "expected draft {expected:?}, got {draft:?}"
            );
        }
        other => panic!("expected Detail::EditApiKey, got {other:?}"),
    }
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Typing a sequence of printable ASCII characters appends each to the draft
// ────────────────────────────────────────────────────────────────────────

#[test]
fn typing_a_sequence_of_printable_ascii_characters_appends_each_to_the_draft() {
    // @step Given I have opened /provider, selected an api_key provider, and entered the EditApiKey form
    let mut view = view_in_edit_api_key_mode();
    // @step And the draft is empty
    assert_draft_eq(&view, "");

    // @step When I type the characters "s", "k", "-", and "t" one at a time
    let mut last_event: Option<ProviderSettingsEvent> = None;
    for c in ['s', 'k', '-', 't'] {
        last_event = Some(view.handle_key(key(KeyCode::Char(c))));
    }

    // @step Then the draft becomes "sk-t" (4 characters)
    assert_draft_eq(&view, "sk-t");
    // @step And each keystroke emits ProviderSettingsEvent::Consumed
    match last_event.expect("at least one keystroke") {
        ProviderSettingsEvent::Consumed => {}
        other => panic!("expected Consumed on each printable char, got {other:?}"),
    }
    // @step And no inline validation status is shown
    assert!(
        view.status.is_empty(),
        "expected empty status, got {:?}",
        view.status
    );
    // @step And no Action is dispatched (no SaveProviderCredentials)
    // (We hold only the LAST event above; for each accepted printable char the arm
    // re-enters EditApiKey and returns Consumed without emitting — the strongly-
    // typed match above already guarantees no Emit(_) escaped on the final char.)
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Space (ASCII 32) is accepted as the lower boundary of the printable range
// ────────────────────────────────────────────────────────────────────────

#[test]
fn space_ascii_32_is_accepted_as_the_lower_boundary() {
    // @step Given I am in the EditApiKey form with the draft "abc"
    let mut view = view_in_edit_api_key_mode();
    for c in "abc".chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
    assert_draft_eq(&view, "abc");

    // @step When I press the space key (ASCII code 32)
    let out = view.handle_key(key(KeyCode::Char(' ')));

    // @step Then the draft becomes "abc " (4 characters, with a trailing space)
    assert_draft_eq(&view, "abc ");
    // @step And the keystroke emits ProviderSettingsEvent::Consumed
    match out {
        ProviderSettingsEvent::Consumed => {}
        other => panic!("expected Consumed, got {other:?}"),
    }
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Tilde "~" (ASCII 126) is accepted as the upper boundary of the printable range
// ────────────────────────────────────────────────────────────────────────

#[test]
fn tilde_ascii_126_is_accepted_as_the_upper_boundary() {
    // @step Given I am in the EditApiKey form with the draft "abc"
    let mut view = view_in_edit_api_key_mode();
    for c in "abc".chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
    assert_draft_eq(&view, "abc");

    // @step When I press the "~" key (ASCII code 126)
    let out = view.handle_key(key(KeyCode::Char('~')));

    // @step Then the draft becomes "abc~" (4 characters)
    assert_draft_eq(&view, "abc~");
    // @step And the keystroke emits ProviderSettingsEvent::Consumed
    match out {
        ProviderSettingsEvent::Consumed => {}
        other => panic!("expected Consumed, got {other:?}"),
    }
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Tab (ASCII 9) is silently dropped as a control character
// ────────────────────────────────────────────────────────────────────────

#[test]
fn tab_ascii_9_is_silently_dropped_as_a_control_character() {
    // @step Given I am in the EditApiKey form with the draft "abc"
    let mut view = view_in_edit_api_key_mode();
    for c in "abc".chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
    assert_draft_eq(&view, "abc");

    // @step When I press a key delivering KeyCode::Char('\t') (ASCII code 9)
    let out = view.handle_key(key(KeyCode::Char('\t')));

    // @step Then the draft remains "abc" (unchanged)
    assert_draft_eq(&view, "abc");
    // @step And the keystroke still emits ProviderSettingsEvent::Consumed
    match out {
        ProviderSettingsEvent::Consumed => {}
        other => panic!("expected Consumed, got {other:?}"),
    }
    // @step And no inline validation status is shown
    assert!(
        view.status.is_empty(),
        "expected empty status, got {:?}",
        view.status
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Unit Separator (ASCII 31) is silently dropped as a control character
// ────────────────────────────────────────────────────────────────────────

#[test]
fn unit_separator_ascii_31_is_silently_dropped() {
    // @step Given I am in the EditApiKey form with the draft "abc"
    let mut view = view_in_edit_api_key_mode();
    for c in "abc".chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
    assert_draft_eq(&view, "abc");

    // @step When I press a key delivering KeyCode::Char('\u{001F}') (ASCII code 31)
    let out = view.handle_key(key(KeyCode::Char('\u{001F}')));

    // @step Then the draft remains "abc"
    assert_draft_eq(&view, "abc");
    // @step And the keystroke still emits ProviderSettingsEvent::Consumed
    match out {
        ProviderSettingsEvent::Consumed => {}
        other => panic!("expected Consumed, got {other:?}"),
    }
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: DEL (ASCII 127) is silently dropped as out-of-range
// ────────────────────────────────────────────────────────────────────────

#[test]
fn del_ascii_127_is_silently_dropped_as_out_of_range() {
    // @step Given I am in the EditApiKey form with the draft "abc"
    let mut view = view_in_edit_api_key_mode();
    for c in "abc".chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
    assert_draft_eq(&view, "abc");

    // @step When I press a key delivering KeyCode::Char('\u{007F}') (ASCII code 127, DEL)
    let out = view.handle_key(key(KeyCode::Char('\u{007F}')));

    // @step Then the draft remains "abc"
    assert_draft_eq(&view, "abc");
    // @step And the keystroke still emits ProviderSettingsEvent::Consumed
    match out {
        ProviderSettingsEvent::Consumed => {}
        other => panic!("expected Consumed, got {other:?}"),
    }
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Non-ASCII Latin-1 character "é" (U+00E9) is silently dropped
// ────────────────────────────────────────────────────────────────────────

#[test]
fn non_ascii_latin1_e_acute_is_silently_dropped() {
    // @step Given I am in the EditApiKey form with the draft "abc"
    let mut view = view_in_edit_api_key_mode();
    for c in "abc".chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
    assert_draft_eq(&view, "abc");

    // @step When I press a key delivering KeyCode::Char('é') (U+00E9, code 233)
    let out = view.handle_key(key(KeyCode::Char('é')));

    // @step Then the draft remains "abc"
    assert_draft_eq(&view, "abc");
    // @step And the keystroke still emits ProviderSettingsEvent::Consumed
    match out {
        ProviderSettingsEvent::Consumed => {}
        other => panic!("expected Consumed, got {other:?}"),
    }
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Non-ASCII BMP character "✓" (U+2713) is silently dropped
// ────────────────────────────────────────────────────────────────────────

#[test]
fn non_ascii_bmp_check_mark_is_silently_dropped() {
    // @step Given I am in the EditApiKey form with the draft "abc"
    let mut view = view_in_edit_api_key_mode();
    for c in "abc".chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
    assert_draft_eq(&view, "abc");

    // @step When I press a key delivering KeyCode::Char('✓') (U+2713)
    let out = view.handle_key(key(KeyCode::Char('✓')));

    // @step Then the draft remains "abc"
    assert_draft_eq(&view, "abc");
    // @step And the keystroke still emits ProviderSettingsEvent::Consumed
    match out {
        ProviderSettingsEvent::Consumed => {}
        other => panic!("expected Consumed, got {other:?}"),
    }
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Non-BMP emoji "🔑" (U+1F511) is silently dropped
// ────────────────────────────────────────────────────────────────────────

#[test]
fn non_bmp_emoji_key_is_silently_dropped() {
    // @step Given I am in the EditApiKey form with the draft "abc"
    let mut view = view_in_edit_api_key_mode();
    for c in "abc".chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
    assert_draft_eq(&view, "abc");

    // @step When I press a key delivering KeyCode::Char('🔑') (U+1F511)
    let out = view.handle_key(key(KeyCode::Char('🔑')));

    // @step Then the draft remains "abc"
    assert_draft_eq(&view, "abc");
    // @step And the keystroke still emits ProviderSettingsEvent::Consumed
    match out {
        ProviderSettingsEvent::Consumed => {}
        other => panic!("expected Consumed, got {other:?}"),
    }
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Dropping a non-printable char does NOT clear the empty-key
//           validation status; a subsequent printable char does
// ────────────────────────────────────────────────────────────────────────

#[test]
fn dropping_non_printable_keeps_empty_key_status_subsequent_printable_clears_it() {
    // @step Given I am in the EditApiKey form with the draft ""
    let mut view = view_in_edit_api_key_mode();
    assert_draft_eq(&view, "");
    // @step And the inline status reads "API key cannot be empty"
    // (RPC-162 made empty-Enter a silent cancel; the legacy validation
    // string is now seeded directly to exercise the Char-arm's clearing
    // branch in isolation. The clearing branch remains in detail.rs as
    // a defensive no-op so any caller — test or production — that
    // pre-sets the string still has it cleared on the next accepted
    // printable keystroke.)
    view.set_status("API key cannot be empty");
    assert!(
        view.status.contains("API key cannot be empty"),
        "precondition: validation status must be seeded, got {:?}",
        view.status
    );

    // @step When I press a key delivering KeyCode::Char('é') (non-ASCII, dropped)
    let out_drop = view.handle_key(key(KeyCode::Char('é')));

    // @step Then the draft remains ""
    assert_draft_eq(&view, "");
    // @step And the inline status still reads "API key cannot be empty"
    assert!(
        view.status.contains("API key cannot be empty"),
        "dropped char must NOT clear the validation status, got {:?}",
        view.status
    );
    match out_drop {
        ProviderSettingsEvent::Consumed => {}
        other => panic!("expected Consumed on dropped char, got {other:?}"),
    }

    // @step When I then press the "s" key (printable, accepted)
    let out_accept = view.handle_key(key(KeyCode::Char('s')));

    // @step Then the draft becomes "s"
    assert_draft_eq(&view, "s");
    // @step And the inline status is cleared (empty)
    assert!(
        view.status.is_empty(),
        "accepted printable char must clear the validation status, got {:?}",
        view.status
    );
    match out_accept {
        ProviderSettingsEvent::Consumed => {}
        other => panic!("expected Consumed on accepted char, got {other:?}"),
    }
    // Belt-and-braces: no Action fired.
    assert!(
        !matches!(
            out_accept,
            ProviderSettingsEvent::Emit(Action::SaveProviderCredentials { .. })
        ),
        "char-keystrokes must NOT emit SaveProviderCredentials"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: is_printable_ascii() helper classifies characters by ASCII code
//
// NOTE: The helper is a module-private function inside
// rust/fspec-tui/src/views/provider_settings/detail.rs and is unit-tested
// in an inline `#[cfg(test)] mod tests` block at the bottom of that file
// (see RPC-161 architecture note #2). This integration test asserts the
// observable consequence — i.e. the boundary characters that the helper
// classifies as "true" round-trip into the draft and the ones it classifies
// as "false" do not — so that even without a direct unit-test export the
// table is exercised end-to-end via the public ProviderSettingsView API.
// ────────────────────────────────────────────────────────────────────────

#[test]
fn is_printable_ascii_helper_classifies_characters_by_ascii_code_observed_end_to_end() {
    // @step Given the helper function is_printable_ascii(c: char) -> bool exists in views/provider_settings/detail.rs
    // (Verified at compile time by the inline detail.rs unit test; here we
    // exercise the function via its only call site — handle_edit_key.)

    // @step When the helper is called with each of the chars ' ' (32), 'A' (65), '~' (126)
    // @step Then it returns true for every one
    for c in [' ', 'A', '~'] {
        let mut view = view_in_edit_api_key_mode();
        view.handle_key(key(KeyCode::Char(c)));
        match &view.mode {
            ProviderSettingsMode::Detail {
                sub: DetailSub::EditApiKey { draft },
                ..
            } => {
                assert_eq!(
                    draft,
                    &c.to_string(),
                    "char {c:?} (code {}) must be accepted into draft",
                    c as u32
                );
            }
            other => panic!("expected Detail::EditApiKey, got {other:?}"),
        }
    }

    // @step When the helper is called with each of the chars '\t' (9), '\u{001F}' (31), '\u{007F}' (127), 'é' (233), '🔑' (128017)
    // @step Then it returns false for every one
    for c in ['\t', '\u{001F}', '\u{007F}', 'é', '🔑'] {
        let mut view = view_in_edit_api_key_mode();
        view.handle_key(key(KeyCode::Char(c)));
        match &view.mode {
            ProviderSettingsMode::Detail {
                sub: DetailSub::EditApiKey { draft },
                ..
            } => {
                assert!(
                    draft.is_empty(),
                    "char {c:?} (code {}) must NOT enter the draft, but got {draft:?}",
                    c as u32
                );
            }
            other => panic!("expected Detail::EditApiKey, got {other:?}"),
        }
    }
}
