//! RPC-163 — ProviderSettingsView API-key edit Delete-key parity.
//!
//! Feature: spec/features/rpc163-provider-settings-api-key-delete-key-parity.feature
//!
//! This test file validates the acceptance criteria for RPC-163: the
//! `KeyCode::Delete` key in the EditApiKey draft buffer behaves identically
//! to `KeyCode::Backspace` — both pop the last character of `draft` and
//! re-enter `Detail::EditApiKey` with the trimmed draft, emitting
//! `ProviderSettingsEvent::Consumed`. In Summary / OAuthNotice sub-modes
//! Delete is treated as an unrelated key (state preserved, no side-effects).
//!
//! Mirrors TS `key.backspace || key.delete` from
//! src/tui/inputHandlers/apiKeyEditModeHandler.ts:46.
//!
//! Tests are written test-first per ACDD: they FAIL against the current
//! `KeyCode::Backspace => { draft.pop(); … }` arm in detail.rs (which has
//! no Delete binding) and PASS once the arm becomes
//! `KeyCode::Backspace | KeyCode::Delete => { … }`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::{
    DetailStatus, DetailSub, ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView,
};
use codelet_rpc_types::ProviderCredentialInfo;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

/// Build a synthetic Press event with no modifiers — mirrors the harness in
/// provider_settings_view_rpc054.rs / provider_settings_api_key_charset_rpc161.rs.
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

/// Seed an api_key provider and transition into Detail::EditApiKey with an
/// empty draft via two Enter keystrokes (List → Detail::Summary → EditApiKey).
fn view_in_edit_api_key_mode() -> ProviderSettingsView {
    let mut v = ProviderSettingsView::new();
    v.set_providers(vec![pinfo("anthropic", "api_key", true, 1)]);
    v.handle_key(key(KeyCode::Enter));
    v.handle_key(key(KeyCode::Enter));
    v
}

/// Seed an api_key provider and transition into Detail::Summary (one Enter).
fn view_in_summary_mode() -> ProviderSettingsView {
    let mut v = ProviderSettingsView::new();
    v.set_providers(vec![pinfo("anthropic", "api_key", true, 1)]);
    v.handle_key(key(KeyCode::Enter));
    v
}

/// Seed an OAuth provider so Enter routes to Detail::OAuthNotice.
fn view_in_oauth_notice_mode() -> ProviderSettingsView {
    let mut v = ProviderSettingsView::new();
    v.set_providers(vec![pinfo("claude-oauth", "oauth", true, 0)]);
    v.handle_key(key(KeyCode::Enter));
    v
}

/// Helper: assert the view is in Detail::EditApiKey for the given provider
/// and that the draft matches `expected`.
fn assert_edit_draft_for(view: &ProviderSettingsView, expected_pid: &str, expected_draft: &str) {
    match &view.mode {
        ProviderSettingsMode::Detail {
            provider_id,
            sub: DetailSub::EditApiKey { draft },
        } => {
            assert_eq!(
                provider_id, expected_pid,
                "expected EditApiKey for {expected_pid:?}, got provider {provider_id:?}"
            );
            assert_eq!(
                draft, expected_draft,
                "expected draft {expected_draft:?}, got {draft:?}"
            );
        }
        other => panic!("expected Detail::EditApiKey, got {other:?}"),
    }
}

/// Helper: assert the view is in Detail::Summary for the given provider
/// with the expected last_status.
fn assert_summary_with_status(
    view: &ProviderSettingsView,
    expected_pid: &str,
    expected_status: Option<DetailStatus>,
) {
    match &view.mode {
        ProviderSettingsMode::Detail {
            provider_id,
            sub: DetailSub::Summary { last_status },
        } => {
            assert_eq!(provider_id, expected_pid, "wrong provider in Summary");
            assert_eq!(
                last_status, &expected_status,
                "wrong last_status; expected {expected_status:?}, got {last_status:?}"
            );
        }
        other => panic!("expected Detail::Summary, got {other:?}"),
    }
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing Delete on a multi-character draft pops the last character
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_delete_on_multi_character_draft_pops_the_last_character() {
    // @step Given I am in the EditApiKey form with the draft "abc123"
    let mut view = view_in_edit_api_key_mode();
    for c in "abc123".chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
    assert_edit_draft_for(&view, "anthropic", "abc123");

    // @step When I press the Delete key
    let out = view.handle_key(key(KeyCode::Delete));

    // @step Then the draft becomes "abc12"
    // @step And the view remains in Detail::EditApiKey for the same provider
    assert_edit_draft_for(&view, "anthropic", "abc12");

    // @step And the keystroke is reported as ProviderSettingsEvent::Consumed
    match out {
        ProviderSettingsEvent::Consumed => {}
        other => panic!("expected Consumed, got {other:?}"),
    }

    // @step And no Action is dispatched
    assert!(
        !matches!(out, ProviderSettingsEvent::Emit(_)),
        "Delete in EditApiKey must NOT emit an Action, got {out:?}"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing Delete on an empty draft is a silent no-op
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_delete_on_empty_draft_is_a_silent_no_op() {
    // @step Given I am in the EditApiKey form with an empty draft
    let mut view = view_in_edit_api_key_mode();
    assert_edit_draft_for(&view, "anthropic", "");
    // @step And the inline validation status is empty
    assert!(view.status.is_empty(), "precondition: status must start empty");

    // @step When I press the Delete key
    let out = view.handle_key(key(KeyCode::Delete));

    // @step Then the draft remains empty
    // @step And the view remains in Detail::EditApiKey for the same provider
    assert_edit_draft_for(&view, "anthropic", "");
    // @step And the inline validation status remains empty
    assert!(
        view.status.is_empty(),
        "Delete on empty draft must NOT raise a validation error, got {:?}",
        view.status
    );

    // @step And the keystroke is reported as ProviderSettingsEvent::Consumed
    match out {
        ProviderSettingsEvent::Consumed => {}
        other => panic!("expected Consumed, got {other:?}"),
    }
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing Delete on a single-character draft empties it
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_delete_on_single_character_draft_empties_it() {
    // @step Given I am in the EditApiKey form with the draft "x"
    let mut view = view_in_edit_api_key_mode();
    view.handle_key(key(KeyCode::Char('x')));
    assert_edit_draft_for(&view, "anthropic", "x");

    // @step When I press the Delete key
    let _ = view.handle_key(key(KeyCode::Delete));

    // @step Then the draft becomes ""
    // @step And the view remains in Detail::EditApiKey for the same provider
    assert_edit_draft_for(&view, "anthropic", "");
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Backspace and Delete produce identical pops when alternated
// ────────────────────────────────────────────────────────────────────────

#[test]
fn backspace_and_delete_produce_identical_pops_when_alternated() {
    // @step Given I am in the EditApiKey form with the draft "hello"
    let mut view = view_in_edit_api_key_mode();
    for c in "hello".chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
    assert_edit_draft_for(&view, "anthropic", "hello");

    // @step When I press the following keys in order: Backspace, Delete, Backspace, Delete, Backspace
    // @step Then the draft is empty after each step matches the same sequence "hell", "hel", "he", "h", ""
    let sequence: &[(KeyCode, &str)] = &[
        (KeyCode::Backspace, "hell"),
        (KeyCode::Delete, "hel"),
        (KeyCode::Backspace, "he"),
        (KeyCode::Delete, "h"),
        (KeyCode::Backspace, ""),
    ];
    let mut last_out: Option<ProviderSettingsEvent> = None;
    for (code, expected_after) in sequence.iter().copied() {
        let out = view.handle_key(key(code));
        assert_edit_draft_for(&view, "anthropic", expected_after);
        // @step And every keystroke is reported as ProviderSettingsEvent::Consumed
        match out {
            ProviderSettingsEvent::Consumed => {}
            ref other => panic!(
                "expected Consumed for {code:?} → {expected_after:?}, got {other:?}"
            ),
        }
        last_out = Some(out);
    }

    // @step And the final draft is ""
    // @step And the view remains in Detail::EditApiKey for the same provider throughout
    assert_edit_draft_for(&view, "anthropic", "");
    assert!(last_out.is_some(), "expected at least one keystroke processed");
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing Delete must not clear the "API key cannot be empty" status
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_delete_must_not_clear_the_api_key_cannot_be_empty_status() {
    // @step Given I am in the EditApiKey form with the draft "abc"
    let mut view = view_in_edit_api_key_mode();
    // Build the draft to "abc" — each accepted printable char would clear the
    // status (per RPC-161). RPC-162 made empty-Enter a silent cancel rather
    // than a setter of the legacy validation string, so we seed the status
    // directly via the public set_status() API to mirror the example:
    // draft "abc" + status "API key cannot be empty".
    for c in "abc".chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
    view.set_status("API key cannot be empty");
    assert_edit_draft_for(&view, "anthropic", "abc");
    // @step And the inline validation status is "API key cannot be empty"
    assert_eq!(view.status, "API key cannot be empty");

    // @step When I press the Delete key
    let _ = view.handle_key(key(KeyCode::Delete));

    // @step Then the draft becomes "ab"
    // @step And the view remains in Detail::EditApiKey for the same provider
    assert_edit_draft_for(&view, "anthropic", "ab");

    // @step And the inline validation status remains "API key cannot be empty"
    assert_eq!(
        view.status, "API key cannot be empty",
        "Delete must NOT clear the validation status, got {:?}",
        view.status
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing Delete in Summary sub-mode is treated as unrelated
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_delete_in_summary_sub_mode_is_treated_as_unrelated() {
    // @step Given I am in Detail::Summary for the provider "anthropic" with last_status Some(Testing)
    let mut view = view_in_summary_mode();
    // RPC-154 removed the `t` keybind from handle_summary_key (TS parity:
    // TS binds no `t` to test-connection on any Detail surface). The
    // earlier setup pressed `t` to transition Summary { None } →
    // Summary { Some(Testing) }; after RPC-154 we construct that state
    // directly. The Delete-key behaviour under test (state preserved,
    // Consumed, no Action) is independent of HOW we arrived at
    // last_status: Some(Testing) — what matters is that the precondition
    // matches the original Gherkin step.
    view.mode = ProviderSettingsMode::Detail {
        provider_id: "anthropic".to_string(),
        sub: DetailSub::Summary {
            last_status: Some(DetailStatus::Testing),
        },
    };
    assert_summary_with_status(&view, "anthropic", Some(DetailStatus::Testing));

    // @step When I press the Delete key
    let out = view.handle_key(key(KeyCode::Delete));

    // @step Then the view remains in Detail::Summary for "anthropic"
    // @step And the last_status remains Some(Testing)
    assert_summary_with_status(&view, "anthropic", Some(DetailStatus::Testing));

    // @step And the keystroke is reported as ProviderSettingsEvent::Consumed
    match out {
        ProviderSettingsEvent::Consumed => {}
        ref other => panic!("expected Consumed, got {other:?}"),
    }

    // @step And no Action is dispatched
    assert!(
        !matches!(out, ProviderSettingsEvent::Emit(_)),
        "Delete in Summary must NOT emit an Action, got {out:?}"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing Delete in OAuthNotice sub-mode does not exit the notice
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_delete_in_oauth_notice_sub_mode_does_not_exit_the_notice() {
    // @step Given I am in Detail::OAuthNotice for an OAuth-only provider
    let mut view = view_in_oauth_notice_mode();
    assert!(
        matches!(
            view.mode,
            ProviderSettingsMode::Detail {
                sub: DetailSub::OAuthNotice,
                ..
            }
        ),
        "precondition: must be in OAuthNotice, got {:?}",
        view.mode
    );

    // @step When I press the Delete key
    let out = view.handle_key(key(KeyCode::Delete));

    // @step Then the view remains in Detail::OAuthNotice
    assert!(
        matches!(
            view.mode,
            ProviderSettingsMode::Detail {
                sub: DetailSub::OAuthNotice,
                ..
            }
        ),
        "Delete must NOT exit OAuthNotice, got {:?}",
        view.mode
    );

    // @step And the keystroke is reported as ProviderSettingsEvent::Consumed
    match out {
        ProviderSettingsEvent::Consumed => {}
        ref other => panic!("expected Consumed, got {other:?}"),
    }
}
