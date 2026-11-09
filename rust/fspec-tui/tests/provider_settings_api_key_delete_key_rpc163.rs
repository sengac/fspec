//! RPC-163 — ProviderSettingsView API-key edit Delete-key parity.
//!
//! Feature: spec/features/rpc163-provider-settings-api-key-delete-key-parity.feature
//!
//! This test file validates the acceptance criteria for RPC-163: the
//! `KeyCode::Delete` key in the EditApiKey draft buffer behaves identically
//! to `KeyCode::Backspace` — both pop the last character of `draft` and
//! re-enter `Detail::EditApiKey` with the trimmed draft, emitting
//! `ProviderSettingsEvent::Consumed`.
//!
//! Mirrors TS `key.backspace || key.delete` from
//! src/tui/inputHandlers/apiKeyEditModeHandler.ts:46.
//!
//! Tests are written test-first per ACDD: they FAIL against the current
//! `KeyCode::Backspace => { draft.pop(); … }` arm in detail.rs (which has
//! no Delete binding) and PASS once the arm becomes
//! `KeyCode::Backspace | KeyCode::Delete => { … }`.

use codelet_fspec_tui::views::provider_settings::nav_item::NavItemKind;
use codelet_fspec_tui::views::provider_settings::projection::project_display_infos;
use codelet_fspec_tui::views::{
    DetailSub, ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView,
};
use codelet_rpc_types::ProviderCredentialInfo;
use codelet_test_helpers::{OrFail, TestResult};
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

/// Seed an api_key provider and open Detail::EditApiKey with an empty draft
/// the way the UI does: Enter on the expanded provider's API-key row.
fn view_in_edit_api_key_mode() -> Result<ProviderSettingsView, codelet_test_helpers::TestError> {
    let infos = vec![pinfo("anthropic", "api_key", true, 1)];
    let mut v = ProviderSettingsView::new();
    v.set_provider_display_infos(project_display_infos(&infos, &[]));
    v.set_providers(infos);
    v.toggle_expansion("anthropic");
    v.selected_index = v
        .nav_items
        .iter()
        .position(|n| matches!(n.kind, NavItemKind::ApiKey))
        .or_fail("the expanded provider has an API-key row")?;
    v.handle_key(key(KeyCode::Enter));
    Ok(v)
}

/// Helper: assert the view is in Detail::EditApiKey for the given provider
/// and that the draft matches `expected`.
fn assert_edit_draft_for(
    view: &ProviderSettingsView,
    expected_pid: &str,
    expected_draft: &str,
) -> Result<(), codelet_test_helpers::TestError> {
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
        other => return Err(format!("expected Detail::EditApiKey, got {other:?}").into()),
    }
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing Delete on a multi-character draft pops the last character
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_delete_on_multi_character_draft_pops_the_last_character() -> TestResult {
    // @step Given I am in the EditApiKey form with the draft "abc123"
    let mut view = view_in_edit_api_key_mode()?;
    for c in "abc123".chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
    assert_edit_draft_for(&view, "anthropic", "abc123")?;

    // @step When I press the Delete key
    let out = view.handle_key(key(KeyCode::Delete));

    // @step Then the draft becomes "abc12"
    // @step And the view remains in Detail::EditApiKey for the same provider
    assert_edit_draft_for(&view, "anthropic", "abc12")?;

    // @step And the keystroke is reported as ProviderSettingsEvent::Consumed
    match out {
        ProviderSettingsEvent::Consumed => {}
        other => return Err(format!("expected Consumed, got {other:?}").into()),
    }

    // @step And no Action is dispatched
    assert!(
        !matches!(out, ProviderSettingsEvent::Emit(_)),
        "Delete in EditApiKey must NOT emit an Action, got {out:?}"
    );
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing Delete on an empty draft is a silent no-op
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_delete_on_empty_draft_is_a_silent_no_op() -> TestResult {
    // @step Given I am in the EditApiKey form with an empty draft
    let mut view = view_in_edit_api_key_mode()?;
    assert_edit_draft_for(&view, "anthropic", "")?;
    // @step And the inline validation status is empty
    assert!(
        view.status.is_empty(),
        "precondition: status must start empty"
    );

    // @step When I press the Delete key
    let out = view.handle_key(key(KeyCode::Delete));

    // @step Then the draft remains empty
    // @step And the view remains in Detail::EditApiKey for the same provider
    assert_edit_draft_for(&view, "anthropic", "")?;
    // @step And the inline validation status remains empty
    assert!(
        view.status.is_empty(),
        "Delete on empty draft must NOT raise a validation error, got {:?}",
        view.status
    );

    // @step And the keystroke is reported as ProviderSettingsEvent::Consumed
    match out {
        ProviderSettingsEvent::Consumed => {}
        other => return Err(format!("expected Consumed, got {other:?}").into()),
    }
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing Delete on a single-character draft empties it
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_delete_on_single_character_draft_empties_it() -> TestResult {
    // @step Given I am in the EditApiKey form with the draft "x"
    let mut view = view_in_edit_api_key_mode()?;
    view.handle_key(key(KeyCode::Char('x')));
    assert_edit_draft_for(&view, "anthropic", "x")?;

    // @step When I press the Delete key
    let _ = view.handle_key(key(KeyCode::Delete));

    // @step Then the draft becomes ""
    // @step And the view remains in Detail::EditApiKey for the same provider
    assert_edit_draft_for(&view, "anthropic", "")?;
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Backspace and Delete produce identical pops when alternated
// ────────────────────────────────────────────────────────────────────────

#[test]
fn backspace_and_delete_produce_identical_pops_when_alternated() -> TestResult {
    // @step Given I am in the EditApiKey form with the draft "hello"
    let mut view = view_in_edit_api_key_mode()?;
    for c in "hello".chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
    assert_edit_draft_for(&view, "anthropic", "hello")?;

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
        assert_edit_draft_for(&view, "anthropic", expected_after)?;
        // @step And every keystroke is reported as ProviderSettingsEvent::Consumed
        match out {
            ProviderSettingsEvent::Consumed => {}
            ref other => {
                return Err(format!(
                    "expected Consumed for {code:?} → {expected_after:?}, got {other:?}"
                )
                .into())
            }
        }
        last_out = Some(out);
    }

    // @step And the final draft is ""
    // @step And the view remains in Detail::EditApiKey for the same provider throughout
    assert_edit_draft_for(&view, "anthropic", "")?;
    assert!(
        last_out.is_some(),
        "expected at least one keystroke processed"
    );
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing Delete must not clear the "API key cannot be empty" status
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_delete_must_not_clear_the_api_key_cannot_be_empty_status() -> TestResult {
    // @step Given I am in the EditApiKey form with the draft "abc"
    let mut view = view_in_edit_api_key_mode()?;
    // Build the draft to "abc" — each accepted printable char would clear the
    // status (per RPC-161). RPC-162 made empty-Enter a silent cancel rather
    // than a setter of the legacy validation string, so we seed the status
    // directly via the public set_status() API to mirror the example:
    // draft "abc" + status "API key cannot be empty".
    for c in "abc".chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
    view.set_status("API key cannot be empty");
    assert_edit_draft_for(&view, "anthropic", "abc")?;
    // @step And the inline validation status is "API key cannot be empty"
    assert_eq!(view.status, "API key cannot be empty");

    // @step When I press the Delete key
    let _ = view.handle_key(key(KeyCode::Delete));

    // @step Then the draft becomes "ab"
    // @step And the view remains in Detail::EditApiKey for the same provider
    assert_edit_draft_for(&view, "anthropic", "ab")?;

    // @step And the inline validation status remains "API key cannot be empty"
    assert_eq!(
        view.status, "API key cannot be empty",
        "Delete must NOT clear the validation status, got {:?}",
        view.status
    );
    Ok(())
}
