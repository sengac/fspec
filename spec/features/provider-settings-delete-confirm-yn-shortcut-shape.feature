@done
@validation
@provider-settings
@tui
@regression
@ts-parity
@keyboard-navigation
@source-shape
@rust
@RPC-156
Feature: Provider settings delete-confirm dialog: n/N cancel shortcut

  """
  [0] This card complements the full integration coverage already provided by RPC-164 (codelet/fspec-tui/tests/confirm_dialog_yn_shortcut_rpc164.rs, 13 tests). Pattern matches RPC-077 / RPC-149 fast structural source-string regression-shape complement to slow integration tests.
  [1] Test file: codelet/fspec-tui/tests/rpc156_delete_confirm_yn_shortcut_shape.rs — sub-millisecond execution, no key event simulation, just source-string scanning of confirm_dialog.rs.
  [2] Source path: codelet/fspec-tui/src/views/agent/confirm_dialog.rs handle_key method (lines 138-168). Lines 162-165 contain the y/Y → outcome_for_index(0) and n/N → outcome_for_index(self.cancel_index()) arms added by RPC-164.
  """

  Background: User Story
    As a fspec maintainer
    I want to have fast regression-shape tests pinning the n/N cancel-shortcut binding in confirm_dialog.rs
    So that the TS-parity keybind cannot silently regress without paying the full ratatui integration-test compile cost on every CI run

  Scenario: confirm_dialog.rs handle_key binds n/N as cancel shortcut
    Given I read the source of codelet/fspec-tui/src/views/agent/confirm_dialog.rs
    When I scan the handle_key match body
    Then the source must contain "KeyCode::Char('n') | KeyCode::Char('N')"

  Scenario: confirm_dialog.rs handle_key binds y/Y as primary shortcut
    Given I read the source of codelet/fspec-tui/src/views/agent/confirm_dialog.rs
    When I scan the handle_key match body
    Then the source must contain "KeyCode::Char('y') | KeyCode::Char('Y')"

  Scenario: n/N arm is wired to the cancel-index outcome path (not focused-index)
    Given I read the source of codelet/fspec-tui/src/views/agent/confirm_dialog.rs
    When I locate the handle_key match arm for KeyCode::Char('n')
    Then the source must contain "outcome_for_index(self.cancel_index())"

  Scenario: handle_key modifier guard rejects Ctrl/Alt + y|Y|n|N
    Given I read the source of codelet/fspec-tui/src/views/agent/confirm_dialog.rs
    When I scan the top of the handle_key body
    Then the source must contain "mods.contains(KeyModifiers::CONTROL)"
    And the source must contain "mods.contains(KeyModifiers::ALT)"
    And the source must contain "ConfirmDialogOutcome::Ignored"
