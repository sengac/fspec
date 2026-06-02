//! RPC-162 — ProviderSettingsView EditApiKey silent-cancel parity.
//!
//! Feature: spec/features/rpc162-provider-settings-edit-api-key-silent-cancel.feature
//!
//! This test file validates the acceptance criteria for RPC-162:
//! pressing Enter in Detail::EditApiKey with an empty draft now
//! transitions silently to List mode (no Action emitted, no
//! "API key cannot be empty" status set). Pressing Esc in
//! EditApiKey returns directly to List (not to Detail::Summary).
//! Pressing Enter with a non-empty draft still emits
//! `Action::SaveProviderCredentials` but now lands on List mode
//! (no intermediate Detail::Summary { SavingCredentials }).
//!
//! Mirrors TS reference src/tui/components/ProviderSettingsPanel.tsx,
//! whose editApiKey form returns silently on empty submission and
//! unmounts back to the provider list on save.
//!
//! Tests are written test-first per ACDD: they FAIL against the
//! current `handle_edit_key` implementation in detail.rs (which
//! routes to Detail::Summary on every exit path) and PASS once
//! the three exit arms are rewritten to target ProviderSettingsMode::List.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::{
    DetailSub, ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView,
};
use codelet_rpc_types::ProviderCredentialInfo;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

/// Build a synthetic Press event with no modifiers.
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
/// transitioned into Detail::EditApiKey mode with an empty draft via the
/// legacy List → Summary → EditApiKey path. RPC-162 only changes
/// EditApiKey EXIT transitions so this entry path is still valid.
fn view_in_edit_api_key_mode_for(provider: &str) -> ProviderSettingsView {
    let mut v = ProviderSettingsView::new();
    v.set_providers(vec![pinfo(provider, "api_key", true, 1)]);
    v.handle_key(key(KeyCode::Enter)); // List → Detail::Summary (legacy fallback)
    v.handle_key(key(KeyCode::Enter)); // Detail::Summary → Detail::EditApiKey
    v
}

/// Type each char of `text` into the view (Char arm of handle_edit_key).
fn type_chars(view: &mut ProviderSettingsView, text: &str) {
    for c in text.chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
}

/// Press Backspace `n` times.
fn press_backspace(view: &mut ProviderSettingsView, n: usize) {
    for _ in 0..n {
        view.handle_key(key(KeyCode::Backspace));
    }
}

/// Assert the view is in Detail::EditApiKey mode and return the draft.
fn read_edit_draft(view: &ProviderSettingsView) -> String {
    match &view.mode {
        ProviderSettingsMode::Detail { sub: DetailSub::EditApiKey { draft }, .. } => {
            draft.clone()
        }
        other => panic!("expected Detail::EditApiKey, got {other:?}"),
    }
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing Enter on an empty EditApiKey draft transitions to List
// mode and emits no Action
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_enter_on_an_empty_edit_api_key_draft_transitions_to_list_mode_and_emits_no_action() {
    // @step Given the ProviderSettingsView is in Detail::EditApiKey for "anthropic" with empty draft
    let mut view = view_in_edit_api_key_mode_for("anthropic");
    assert_eq!(read_edit_draft(&view), "");

    // @step When the user presses Enter
    let evt = view.handle_key(key(KeyCode::Enter));

    // @step Then the view's mode is ProviderSettingsMode::List
    assert!(
        matches!(view.mode, ProviderSettingsMode::List),
        "expected List, got {:?}", view.mode
    );

    // @step And view.status is the empty string
    assert_eq!(view.status, "", "view.status must be empty after silent cancel");

    // @step And no ProviderSettingsEvent::Emit is dispatched
    assert!(
        !matches!(evt, ProviderSettingsEvent::Emit(_)),
        "expected no Emit, got {evt:?}"
    );

    // @step And handle_key returns ProviderSettingsEvent::Consumed
    assert!(
        matches!(evt, ProviderSettingsEvent::Consumed),
        "expected Consumed, got {evt:?}"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing Enter on an empty EditApiKey draft never writes the
// legacy "API key cannot be empty" status
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_enter_on_an_empty_edit_api_key_draft_never_writes_the_legacy_validation_status() {
    // @step Given the ProviderSettingsView is in Detail::EditApiKey for "anthropic" with empty draft
    let mut view = view_in_edit_api_key_mode_for("anthropic");
    assert_eq!(read_edit_draft(&view), "");

    // @step And view.status is the empty string
    assert_eq!(view.status, "", "precondition: view.status starts empty");

    // @step When the user presses Enter
    view.handle_key(key(KeyCode::Enter));

    // @step Then view.status remains the empty string
    // @step And view.status is never equal to "API key cannot be empty"
    assert_eq!(
        view.status, "",
        "after silent cancel view.status must be empty, never 'API key cannot be empty'"
    );
    assert_ne!(view.status, "API key cannot be empty");
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing Enter on a non-empty EditApiKey draft emits
// SaveProviderCredentials and returns to List mode
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_enter_on_a_non_empty_edit_api_key_draft_emits_save_and_returns_to_list() {
    // @step Given the ProviderSettingsView is in Detail::EditApiKey for "anthropic" with draft "sk-abc"
    let mut view = view_in_edit_api_key_mode_for("anthropic");
    type_chars(&mut view, "sk-abc");
    assert_eq!(read_edit_draft(&view), "sk-abc");

    // @step When the user presses Enter
    let evt = view.handle_key(key(KeyCode::Enter));

    // @step Then the emitted ProviderSettingsEvent is Emit(Action::SaveProviderCredentials { provider_id: "anthropic", api_key: "sk-abc" })
    match evt {
        ProviderSettingsEvent::Emit(Action::SaveProviderCredentials { provider_id, api_key }) => {
            assert_eq!(provider_id, "anthropic");
            assert_eq!(api_key, "sk-abc");
        }
        other => panic!("expected Emit(SaveProviderCredentials), got {other:?}"),
    }

    // @step And the view's mode is ProviderSettingsMode::List
    assert!(
        matches!(view.mode, ProviderSettingsMode::List),
        "expected List after non-empty save, got {:?}", view.mode
    );

    // @step And view.status is the empty string
    assert_eq!(view.status, "", "view.status must be empty after save (no Saving… text)");
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing Esc in EditApiKey transitions directly to List mode
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_esc_in_edit_api_key_transitions_directly_to_list_mode() {
    // @step Given the ProviderSettingsView is in Detail::EditApiKey for "anthropic" with draft "sk-cancel"
    let mut view = view_in_edit_api_key_mode_for("anthropic");
    type_chars(&mut view, "sk-cancel");
    assert_eq!(read_edit_draft(&view), "sk-cancel");

    // @step When the user presses Esc
    let evt = view.handle_key(key(KeyCode::Esc));

    // @step Then the view's mode is ProviderSettingsMode::List
    assert!(
        matches!(view.mode, ProviderSettingsMode::List),
        "expected List after Esc, got {:?}", view.mode
    );

    // @step And view.status is the empty string
    assert_eq!(view.status, "");

    // @step And no ProviderSettingsEvent::Emit is dispatched
    assert!(
        !matches!(evt, ProviderSettingsEvent::Emit(_)),
        "expected no Emit, got {evt:?}"
    );

    // @step And handle_key returns ProviderSettingsEvent::Consumed
    assert!(
        matches!(evt, ProviderSettingsEvent::Consumed),
        "expected Consumed, got {evt:?}"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing Esc in EditApiKey with an empty draft also returns
// directly to List mode
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_esc_in_edit_api_key_with_an_empty_draft_also_returns_directly_to_list_mode() {
    // @step Given the ProviderSettingsView is in Detail::EditApiKey for "openai" with empty draft
    let mut view = view_in_edit_api_key_mode_for("openai");
    assert_eq!(read_edit_draft(&view), "");

    // @step When the user presses Esc
    view.handle_key(key(KeyCode::Esc));

    // @step Then the view's mode is ProviderSettingsMode::List
    assert!(
        matches!(view.mode, ProviderSettingsMode::List),
        "expected List after Esc on empty draft, got {:?}", view.mode
    );

    // @step And view.status is the empty string
    assert_eq!(view.status, "");
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Empty-Enter cancel after typing then deleting all characters
// still produces no validation chrome
// ────────────────────────────────────────────────────────────────────────

#[test]
fn empty_enter_cancel_after_typing_then_deleting_all_characters_still_produces_no_validation_chrome() {
    // @step Given the ProviderSettingsView is in Detail::EditApiKey for "anthropic" with empty draft
    let mut view = view_in_edit_api_key_mode_for("anthropic");
    assert_eq!(read_edit_draft(&view), "");

    // @step When the user types "sk-"
    type_chars(&mut view, "sk-");
    assert_eq!(read_edit_draft(&view), "sk-");

    // @step And the user presses Backspace 3 times so the draft is empty again
    press_backspace(&mut view, 3);
    assert_eq!(read_edit_draft(&view), "");

    // @step And the user presses Enter
    view.handle_key(key(KeyCode::Enter));

    // @step Then the view's mode is ProviderSettingsMode::List
    assert!(
        matches!(view.mode, ProviderSettingsMode::List),
        "expected List after typing/deleting/Enter cycle, got {:?}", view.mode
    );

    // @step And view.status is the empty string
    assert_eq!(view.status, "", "no validation chrome must appear");
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Empty-Enter clears any pre-existing legacy "API key cannot be
// empty" status
// ────────────────────────────────────────────────────────────────────────

#[test]
fn empty_enter_clears_any_pre_existing_legacy_validation_status() {
    // @step Given the ProviderSettingsView is in Detail::EditApiKey for "anthropic" with empty draft
    let mut view = view_in_edit_api_key_mode_for("anthropic");
    assert_eq!(read_edit_draft(&view), "");

    // @step And view.status has been manually set to "API key cannot be empty" (legacy state)
    view.set_status("API key cannot be empty");
    assert_eq!(view.status, "API key cannot be empty");

    // @step When the user presses Enter
    view.handle_key(key(KeyCode::Enter));

    // @step Then view.status is the empty string
    assert_eq!(view.status, "", "silent-cancel must clear any legacy validation status");

    // @step And the view's mode is ProviderSettingsMode::List
    assert!(
        matches!(view.mode, ProviderSettingsMode::List),
        "expected List after empty-Enter with legacy status, got {:?}", view.mode
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Non-empty Enter still consumes the draft into the
// SaveProviderCredentials payload verbatim
// ────────────────────────────────────────────────────────────────────────

#[test]
fn non_empty_enter_still_consumes_the_draft_into_save_provider_credentials_verbatim() {
    // @step Given the ProviderSettingsView is in Detail::EditApiKey for "openai" with draft "sk-test-1"
    let mut view = view_in_edit_api_key_mode_for("openai");
    type_chars(&mut view, "sk-test-1");
    assert_eq!(read_edit_draft(&view), "sk-test-1");

    // @step When the user presses Enter
    let evt = view.handle_key(key(KeyCode::Enter));

    // @step Then the emitted ProviderSettingsEvent is Emit(Action::SaveProviderCredentials { provider_id: "openai", api_key: "sk-test-1" })
    match evt {
        ProviderSettingsEvent::Emit(Action::SaveProviderCredentials { provider_id, api_key }) => {
            assert_eq!(provider_id, "openai");
            assert_eq!(api_key, "sk-test-1");
        }
        other => panic!("expected Emit(SaveProviderCredentials), got {other:?}"),
    }

    // @step And the view's mode is ProviderSettingsMode::List
    assert!(
        matches!(view.mode, ProviderSettingsMode::List),
        "expected List after save, got {:?}", view.mode
    );
}
