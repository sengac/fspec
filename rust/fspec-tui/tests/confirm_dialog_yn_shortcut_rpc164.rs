//! RPC-164 — ConfirmDialog y/Y and n/N shortcut parity tests.
//!
//! Feature: spec/features/rpc164-provider-settings-confirm-dialog-yn-shortcut.feature
//!
//! Validates that pressing 'y'/'Y' emits ConfirmDialogOutcome::Primary and
//! 'n'/'N' emits ConfirmDialogOutcome::Cancel from the shared
//! `ConfirmDialog::handle_key`, matching the TS reference at
//! `src/tui/inputHandlers/deleteConfirmModeHandler.ts`. Also covers the
//! ProviderSettingsView integration path (delete-credentials dialog).
//!
//! These tests validate acceptance criteria defined in the feature file;
//! scenarios map 1:1 via @step comments.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::agent::{ConfirmDialog, ConfirmDialogOutcome};
use codelet_fspec_tui::views::{ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView};
use codelet_rpc_types::ProviderCredentialInfo;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

// ────────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────────

fn two_button_dialog() -> ConfirmDialog {
    ConfirmDialog::new(
        "Delete credentials?",
        "Delete credentials for anthropic?",
        "Delete",
        None,
        "Cancel",
    )
}

fn three_button_dialog() -> ConfirmDialog {
    ConfirmDialog::new(
        "Save changes?",
        "You have unsaved changes.",
        "Save",
        Some("Discard".to_string()),
        "Cancel",
    )
}

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

fn provider_settings_with_open_delete_dialog(provider_id: &str) -> ProviderSettingsView {
    let mut view = ProviderSettingsView::new();
    view.set_providers(vec![pinfo(provider_id, "api_key", true, 8)]);
    // Press 'd' on the configured row to open delete_confirm dialog
    view.handle_key(key(KeyCode::Char('d')));
    assert!(
        view.delete_confirm.is_some(),
        "delete_confirm dialog should be open after pressing 'd' on configured row"
    );
    view
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing 'y' on a 2-button dialog emits Primary
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_y_on_two_button_dialog_emits_primary() {
    // @step Given a 2-button ConfirmDialog is open with buttons "Delete" and "Cancel"
    let mut dialog = two_button_dialog();
    assert_eq!(dialog.buttons().len(), 2);
    assert_eq!(dialog.primary_label(), "Delete");
    assert_eq!(dialog.cancel_label(), "Cancel");
    // @step And the focused button index is 0
    assert_eq!(dialog.focused(), 0);

    // @step When I press the 'y' key with no modifiers
    let outcome = dialog.handle_key(KeyCode::Char('y'), KeyModifiers::NONE);

    // @step Then handle_key returns ConfirmDialogOutcome::Primary
    assert_eq!(outcome, ConfirmDialogOutcome::Primary);
    // @step And the focused button index remains 0
    assert_eq!(dialog.focused(), 0);
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing 'Y' (uppercase) on a 2-button dialog emits Primary
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_capital_y_on_two_button_dialog_emits_primary() {
    // @step Given a 2-button ConfirmDialog is open with buttons "Delete" and "Cancel"
    let mut dialog = two_button_dialog();
    // @step And the focused button index is 0
    assert_eq!(dialog.focused(), 0);

    // @step When I press the 'Y' key with no modifiers
    let outcome = dialog.handle_key(KeyCode::Char('Y'), KeyModifiers::NONE);

    // @step Then handle_key returns ConfirmDialogOutcome::Primary
    assert_eq!(outcome, ConfirmDialogOutcome::Primary);
    // @step And the focused button index remains 0
    assert_eq!(dialog.focused(), 0);
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing 'n' on a 2-button dialog emits Cancel
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_n_on_two_button_dialog_emits_cancel() {
    // @step Given a 2-button ConfirmDialog is open with buttons "Delete" and "Cancel"
    let mut dialog = two_button_dialog();
    // @step And the focused button index is 0
    assert_eq!(dialog.focused(), 0);

    // @step When I press the 'n' key with no modifiers
    let outcome = dialog.handle_key(KeyCode::Char('n'), KeyModifiers::NONE);

    // @step Then handle_key returns ConfirmDialogOutcome::Cancel
    assert_eq!(outcome, ConfirmDialogOutcome::Cancel);
    // @step And the focused button index remains 0
    assert_eq!(dialog.focused(), 0);
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing 'N' (uppercase) on a 2-button dialog emits Cancel
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_capital_n_on_two_button_dialog_emits_cancel() {
    // @step Given a 2-button ConfirmDialog is open with buttons "Delete" and "Cancel"
    let mut dialog = two_button_dialog();
    // @step And the focused button index is 0
    assert_eq!(dialog.focused(), 0);

    // @step When I press the 'N' key with no modifiers
    let outcome = dialog.handle_key(KeyCode::Char('N'), KeyModifiers::NONE);

    // @step Then handle_key returns ConfirmDialogOutcome::Cancel
    assert_eq!(outcome, ConfirmDialogOutcome::Cancel);
    // @step And the focused button index remains 0
    assert_eq!(dialog.focused(), 0);
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing 'n' on a 3-button dialog emits Cancel (not Secondary)
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_n_on_three_button_dialog_emits_cancel_not_secondary() {
    // @step Given a 3-button ConfirmDialog is open with buttons "Save", "Discard", "Cancel"
    let mut dialog = three_button_dialog();
    assert_eq!(dialog.buttons().len(), 3);
    assert_eq!(dialog.primary_label(), "Save");
    assert_eq!(dialog.secondary_label(), Some("Discard"));
    assert_eq!(dialog.cancel_label(), "Cancel");
    // @step And the focused button index is 0
    assert_eq!(dialog.focused(), 0);

    // @step When I press the 'n' key with no modifiers
    let outcome = dialog.handle_key(KeyCode::Char('n'), KeyModifiers::NONE);

    // @step Then handle_key returns ConfirmDialogOutcome::Cancel
    assert_eq!(outcome, ConfirmDialogOutcome::Cancel);
    // @step And the outcome is NOT ConfirmDialogOutcome::Secondary
    assert_ne!(outcome, ConfirmDialogOutcome::Secondary);
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing 'y' on a 3-button dialog emits Primary
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_y_on_three_button_dialog_emits_primary() {
    // @step Given a 3-button ConfirmDialog is open with buttons "Save", "Discard", "Cancel"
    let mut dialog = three_button_dialog();
    // @step And the focused button index is 0
    assert_eq!(dialog.focused(), 0);

    // @step When I press the 'y' key with no modifiers
    let outcome = dialog.handle_key(KeyCode::Char('y'), KeyModifiers::NONE);

    // @step Then handle_key returns ConfirmDialogOutcome::Primary
    assert_eq!(outcome, ConfirmDialogOutcome::Primary);
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing 'y' ignores the currently focused button
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_y_ignores_currently_focused_button() {
    // @step Given a 2-button ConfirmDialog is open with buttons "Delete" and "Cancel"
    let mut dialog = two_button_dialog();
    // @step And the focused button index has been moved to 1 by pressing Tab
    let _ = dialog.handle_key(KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(
        dialog.focused(),
        1,
        "Tab should move focus to Cancel (idx 1)"
    );

    // @step When I press the 'y' key with no modifiers
    let outcome = dialog.handle_key(KeyCode::Char('y'), KeyModifiers::NONE);

    // @step Then handle_key returns ConfirmDialogOutcome::Primary
    assert_eq!(outcome, ConfirmDialogOutcome::Primary);
    // @step And the focused button index remains 1
    assert_eq!(dialog.focused(), 1);
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing 'n' with Ctrl modifier returns Ignored
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_ctrl_n_returns_ignored() {
    // @step Given a 2-button ConfirmDialog is open with buttons "Delete" and "Cancel"
    let mut dialog = two_button_dialog();

    // @step When I press the 'n' key with the CONTROL modifier
    let outcome = dialog.handle_key(KeyCode::Char('n'), KeyModifiers::CONTROL);

    // @step Then handle_key returns ConfirmDialogOutcome::Ignored
    assert_eq!(outcome, ConfirmDialogOutcome::Ignored);
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing 'y' with Alt modifier returns Ignored
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_alt_y_returns_ignored() {
    // @step Given a 2-button ConfirmDialog is open with buttons "Delete" and "Cancel"
    let mut dialog = two_button_dialog();

    // @step When I press the 'y' key with the ALT modifier
    let outcome = dialog.handle_key(KeyCode::Char('y'), KeyModifiers::ALT);

    // @step Then handle_key returns ConfirmDialogOutcome::Ignored
    assert_eq!(outcome, ConfirmDialogOutcome::Ignored);
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing an unrelated printable character returns Ignored
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_unrelated_printable_char_returns_ignored() {
    // @step Given a 2-button ConfirmDialog is open with buttons "Delete" and "Cancel"
    let mut dialog = two_button_dialog();

    // @step When I press the 'q' key with no modifiers
    let outcome = dialog.handle_key(KeyCode::Char('q'), KeyModifiers::NONE);

    // @step Then handle_key returns ConfirmDialogOutcome::Ignored
    assert_eq!(outcome, ConfirmDialogOutcome::Ignored);
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing 'y' inside ProviderSettingsView delete-credentials
//   dialog emits ConfirmDeleteProviderCredentials
// ────────────────────────────────────────────────────────────────────────

#[test]
fn provider_settings_y_emits_confirm_delete_provider_credentials() {
    // @step Given the ProviderSettingsView has a delete-credentials ConfirmDialog open for provider "anthropic"
    let mut view = provider_settings_with_open_delete_dialog("anthropic");

    // @step When I press the 'y' key with no modifiers
    let out = view.handle_key(key(KeyCode::Char('y')));

    // @step Then ProviderSettingsView::handle_key returns ProviderSettingsEvent::Emit(Action::ConfirmDeleteProviderCredentials("anthropic"))
    match out {
        ProviderSettingsEvent::Emit(Action::ConfirmDeleteProviderCredentials(id)) => {
            assert_eq!(id, "anthropic");
        }
        _ => panic!("expected Emit(ConfirmDeleteProviderCredentials), got {out:?}"),
    }
    // @step And the delete_confirm dialog is cleared from view state
    assert!(view.delete_confirm.is_none());
    assert!(matches!(view.mode, ProviderSettingsMode::List));
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing 'n' inside ProviderSettingsView delete-credentials
//   dialog dismisses silently
// ────────────────────────────────────────────────────────────────────────

#[test]
fn provider_settings_n_dismisses_dialog_silently() {
    // @step Given the ProviderSettingsView has a delete-credentials ConfirmDialog open for provider "anthropic"
    let mut view = provider_settings_with_open_delete_dialog("anthropic");

    // @step When I press the 'n' key with no modifiers
    let out = view.handle_key(key(KeyCode::Char('n')));

    // @step Then ProviderSettingsView::handle_key returns ProviderSettingsEvent::Consumed
    assert!(
        matches!(out, ProviderSettingsEvent::Consumed),
        "expected Consumed, got {out:?}"
    );
    // @step And no Action is dispatched
    // (Consumed guarantees no Emit; verified by the match above)
    // @step And the delete_confirm dialog is cleared from view state
    assert!(view.delete_confirm.is_none());
    assert!(matches!(view.mode, ProviderSettingsMode::List));
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pre-existing keybinds remain unchanged
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pre_existing_keybinds_remain_unchanged() {
    // @step Given a 2-button ConfirmDialog is open with buttons "Delete" and "Cancel"
    let mut dialog = two_button_dialog();

    // @step When I press the following keys in order: Esc
    let esc_outcome = dialog.handle_key(KeyCode::Esc, KeyModifiers::NONE);
    // @step Then handle_key returns ConfirmDialogOutcome::Cancel for Esc
    assert_eq!(esc_outcome, ConfirmDialogOutcome::Cancel);

    // Re-create dialog for subsequent assertions (each starts fresh on the Primary)
    let mut dialog = two_button_dialog();

    // @step And pressing Tab returns ConfirmDialogOutcome::Continued and advances focus
    let tab_outcome = dialog.handle_key(KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(tab_outcome, ConfirmDialogOutcome::Continued);
    assert_eq!(dialog.focused(), 1);

    // @step And pressing Left returns ConfirmDialogOutcome::Continued and rotates focus backward
    let left_outcome = dialog.handle_key(KeyCode::Left, KeyModifiers::NONE);
    assert_eq!(left_outcome, ConfirmDialogOutcome::Continued);
    assert_eq!(dialog.focused(), 0);

    // @step And pressing Right returns ConfirmDialogOutcome::Continued and rotates focus forward
    let right_outcome = dialog.handle_key(KeyCode::Right, KeyModifiers::NONE);
    assert_eq!(right_outcome, ConfirmDialogOutcome::Continued);
    assert_eq!(dialog.focused(), 1);

    // @step And pressing Enter on the focused button returns the matching outcome
    // focused is currently 1 (Cancel), so Enter → Cancel
    let enter_outcome = dialog.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(enter_outcome, ConfirmDialogOutcome::Cancel);

    // And Enter on Primary (focused = 0) → Primary
    let mut dialog = two_button_dialog();
    assert_eq!(dialog.focused(), 0);
    let enter_primary = dialog.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(enter_primary, ConfirmDialogOutcome::Primary);
}
