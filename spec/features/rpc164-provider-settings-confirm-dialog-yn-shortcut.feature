@done
@agent-view
@ts-parity
@provider-settings
@tui
@rust
@RPC-164
Feature: Provider settings: n/N as cancel shortcut in delete-confirm ConfirmDialog
  """
  TS reference: src/tui/inputHandlers/deleteConfirmModeHandler.ts handleConfirmation() function — 'y' || 'Y' → onConfirm; 'n' || 'N' || Esc → onCancel. Visible UI hint: 'Press y to confirm, n or Esc to cancel' (ProviderSettingsPanel.tsx lines 198, 225, 251).
  Rust implementation site: codelet/fspec-tui/src/views/agent/confirm_dialog.rs ConfirmDialog::handle_key() (line 139-160). Insert two new match arms BEFORE the catch-all `_ => Ignored`: KeyCode::Char('y') | KeyCode::Char('Y') => self.outcome_for_index(0) which yields Primary; KeyCode::Char('n') | KeyCode::Char('N') => self.outcome_for_index(self.cancel_index()) which yields Cancel.
  The existing modifier-guard at the top of handle_key (`if mods.contains(CONTROL) || mods.contains(ALT) → Ignored`) already covers y/Y/n/N with modifiers — no extra guard required for Rule 4.
  ConfirmDialog is shared by two callers: ProviderSettingsView (delete provider credentials) and ResumeSessionView (delete session). Both benefit equally from this parity addition; no caller-side changes required since both already destructure Primary/Cancel outcomes.
  MergeConfirmDialog (codelet/fspec-tui/src/views/agent/merge_confirm_dialog.rs) is a SEPARATE component with its own enum and is NOT covered by this work unit. Out of scope.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Pressing 'y' or 'Y' (no Ctrl/Alt modifiers) in a ConfirmDialog emits ConfirmDialogOutcome::Primary — identical to Enter on the Primary button
  #   2. Pressing 'n' or 'N' (no Ctrl/Alt modifiers) in a ConfirmDialog emits ConfirmDialogOutcome::Cancel — identical to Esc or to Enter on the Cancel button
  #   3. y/Y/n/N shortcuts ignore the currently focused button — focus state is NOT consulted and NOT mutated by the shortcut
  #   4. y/Y/n/N with CONTROL or ALT modifiers return ConfirmDialogOutcome::Ignored (same pre-existing guard already applied to all other keys)
  #   5. On a 3-button dialog (Primary, Secondary, Cancel), 'n'/'N' still emits Cancel — never Secondary — because 'n' is wired to the cancel-index outcome path
  #   6. All pre-existing keybinds (Esc → Cancel, Left/Right/Tab → focus navigation, Enter → focused outcome) remain unchanged
  #   7. Other printable characters (e.g., 'q', 'a', 'd') continue to return ConfirmDialogOutcome::Ignored — only y/Y/n/N gain new meaning
  #
  # EXAMPLES:
  #   1. A 2-button delete-confirm dialog (Delete | Cancel) is open; user presses 'y'; handle_key returns ConfirmDialogOutcome::Primary; focus index remains 0
  #   2. Same 2-button dialog; user presses 'Y' (uppercase); handle_key returns ConfirmDialogOutcome::Primary (case-insensitive)
  #   3. Same 2-button dialog; user presses 'n'; handle_key returns ConfirmDialogOutcome::Cancel; focus index remains 0
  #   4. Same 2-button dialog; user presses 'N' (uppercase); handle_key returns ConfirmDialogOutcome::Cancel
  #   5. A 3-button dialog (Save | Discard | Cancel); user presses 'n'; handle_key returns ConfirmDialogOutcome::Cancel (NOT Secondary)
  #   6. A 3-button dialog (Save | Discard | Cancel); user presses 'y'; handle_key returns ConfirmDialogOutcome::Primary
  #   7. A 2-button dialog with focus moved to Cancel via Tab; user presses 'y'; outcome is Primary (focus is ignored), focus stays on Cancel
  #   8. A 2-button dialog; user holds Ctrl while pressing 'n'; handle_key returns ConfirmDialogOutcome::Ignored (modifier guard wins)
  #   9. A 2-button dialog; user holds Alt while pressing 'y'; handle_key returns ConfirmDialogOutcome::Ignored
  #   10. A 2-button dialog; user presses 'q' (unrelated printable char); handle_key returns ConfirmDialogOutcome::Ignored — only y/Y/n/N gain new meaning
  #   11. ProviderSettingsView delete-credentials dialog is open; user presses 'y'; ProviderSettingsView::handle_key returns ProviderSettingsEvent::Emit(Action::ConfirmDeleteProviderCredentials(provider_id)); dialog is dismissed
  #   12. ProviderSettingsView delete-credentials dialog is open; user presses 'n'; dialog is dismissed and ProviderSettingsEvent::Consumed is returned (no Action emitted)
  #   13. All pre-existing scenarios continue to pass: Esc → Cancel, Tab cycles focus, Left/Right cycle focus, Enter on focused button emits matching outcome
  #
  # ========================================
  Background: User Story
    As a TUI user using a confirm-dialog overlay
    I want to press y/Y to confirm and n/N to cancel as one-key shortcuts
    So that I match the TypeScript UI's y/n delete-confirm parity and never have to Tab to the Cancel button

  Scenario: Pressing 'y' on a 2-button dialog emits Primary
    Given a 2-button ConfirmDialog is open with buttons "Delete" and "Cancel"
    And the focused button index is 0
    When I press the 'y' key with no modifiers
    Then handle_key returns ConfirmDialogOutcome::Primary
    And the focused button index remains 0

  Scenario: Pressing 'Y' (uppercase) on a 2-button dialog emits Primary
    Given a 2-button ConfirmDialog is open with buttons "Delete" and "Cancel"
    And the focused button index is 0
    When I press the 'Y' key with no modifiers
    Then handle_key returns ConfirmDialogOutcome::Primary
    And the focused button index remains 0

  Scenario: Pressing 'n' on a 2-button dialog emits Cancel
    Given a 2-button ConfirmDialog is open with buttons "Delete" and "Cancel"
    And the focused button index is 0
    When I press the 'n' key with no modifiers
    Then handle_key returns ConfirmDialogOutcome::Cancel
    And the focused button index remains 0

  Scenario: Pressing 'N' (uppercase) on a 2-button dialog emits Cancel
    Given a 2-button ConfirmDialog is open with buttons "Delete" and "Cancel"
    And the focused button index is 0
    When I press the 'N' key with no modifiers
    Then handle_key returns ConfirmDialogOutcome::Cancel
    And the focused button index remains 0

  Scenario: Pressing 'n' on a 3-button dialog emits Cancel (not Secondary)
    Given a 3-button ConfirmDialog is open with buttons "Save", "Discard", "Cancel"
    And the focused button index is 0
    When I press the 'n' key with no modifiers
    Then handle_key returns ConfirmDialogOutcome::Cancel
    And the outcome is NOT ConfirmDialogOutcome::Secondary

  Scenario: Pressing 'y' on a 3-button dialog emits Primary
    Given a 3-button ConfirmDialog is open with buttons "Save", "Discard", "Cancel"
    And the focused button index is 0
    When I press the 'y' key with no modifiers
    Then handle_key returns ConfirmDialogOutcome::Primary

  Scenario: Pressing 'y' ignores the currently focused button
    Given a 2-button ConfirmDialog is open with buttons "Delete" and "Cancel"
    And the focused button index has been moved to 1 by pressing Tab
    When I press the 'y' key with no modifiers
    Then handle_key returns ConfirmDialogOutcome::Primary
    And the focused button index remains 1

  Scenario: Pressing 'n' with Ctrl modifier returns Ignored
    Given a 2-button ConfirmDialog is open with buttons "Delete" and "Cancel"
    When I press the 'n' key with the CONTROL modifier
    Then handle_key returns ConfirmDialogOutcome::Ignored

  Scenario: Pressing 'y' with Alt modifier returns Ignored
    Given a 2-button ConfirmDialog is open with buttons "Delete" and "Cancel"
    When I press the 'y' key with the ALT modifier
    Then handle_key returns ConfirmDialogOutcome::Ignored

  Scenario: Pressing an unrelated printable character returns Ignored
    Given a 2-button ConfirmDialog is open with buttons "Delete" and "Cancel"
    When I press the 'q' key with no modifiers
    Then handle_key returns ConfirmDialogOutcome::Ignored

  Scenario: Pressing 'y' inside ProviderSettingsView delete-credentials dialog emits ConfirmDeleteProviderCredentials
    Given the ProviderSettingsView has a delete-credentials ConfirmDialog open for provider "anthropic"
    When I press the 'y' key with no modifiers
    Then ProviderSettingsView::handle_key returns ProviderSettingsEvent::Emit(Action::ConfirmDeleteProviderCredentials("anthropic"))
    And the delete_confirm dialog is cleared from view state

  Scenario: Pressing 'n' inside ProviderSettingsView delete-credentials dialog dismisses silently
    Given the ProviderSettingsView has a delete-credentials ConfirmDialog open for provider "anthropic"
    When I press the 'n' key with no modifiers
    Then ProviderSettingsView::handle_key returns ProviderSettingsEvent::Consumed
    And no Action is dispatched
    And the delete_confirm dialog is cleared from view state

  Scenario: Pre-existing keybinds remain unchanged
    Given a 2-button ConfirmDialog is open with buttons "Delete" and "Cancel"
    When I press the following keys in order: Esc
    Then handle_key returns ConfirmDialogOutcome::Cancel for Esc
    And pressing Tab returns ConfirmDialogOutcome::Continued and advances focus
    And pressing Left returns ConfirmDialogOutcome::Continued and rotates focus backward
    And pressing Right returns ConfirmDialogOutcome::Continued and rotates focus forward
    And pressing Enter on the focused button returns the matching outcome
