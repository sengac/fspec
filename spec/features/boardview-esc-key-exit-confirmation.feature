@done
@dialog
@navigation
@board-view
@rpc
@tui
@RPC-102
Feature: BoardView Esc key incorrectly bound to 'q' for exit

  """
  Mirrors the TS BoardView ConfirmationDialog with message 'Exit fspec?' and description 'Are you sure you want to exit?'
  Fix is in codelet/fspec-tui/src/app/events.rs: replace KeyCode::Char('q') at line 130 with KeyCode::Esc that pushes a board exit confirmation dialog onto the compositor (guarded by compositor.contains() to prevent double-push)
  New BoardExitConfirmationDialog component lives in codelet/fspec-tui/src/components/board_exit_confirmation_dialog.rs and emits Action::QuitApp on confirm; the App dispatch sets should_quit=true
  DisconnectDialog 'q'/'r' bindings (events.rs lines 101-122) and Ctrl+D (lines 134-139) are preserved unchanged
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Pressing Esc on the BoardView (no overlay) opens an exit confirmation dialog instead of quitting immediately
  #   2. Pressing 'q' anywhere except the DisconnectDialog is ignored and does not quit the application
  #   3. Confirming the BoardView exit dialog (Enter on Exit) sets should_quit=true and tears down the application
  #   4. Pressing Esc on the BoardView exit confirmation dialog dismisses the dialog without quitting
  #   5. DisconnectDialog 'q' (quit) and 'r' (manual reconnect) bindings are preserved per RPC-011 CR-1
  #   6. AgentView Esc-cascade behaviour is unchanged (regression guard)
  #
  # EXAMPLES:
  #   1. User is on the board view, presses Esc, and sees an 'Exit fspec?' confirmation dialog appear over the board
  #   2. User is on the board view, presses 'q', and nothing happens — the application keeps running
  #   3. User sees the exit confirmation dialog, presses Enter on the 'Exit' option, and the application closes
  #   4. User sees the exit confirmation dialog, presses Esc, the dialog disappears and the user returns to the board view
  #   5. Backend connection drops, DisconnectDialog appears, user presses 'q', the application exits (RPC-011 CR-1 preserved)
  #   6. Backend connection drops, DisconnectDialog appears, user presses 'r', the application attempts reconnection (RPC-011 CR-1 preserved)
  #   7. User is in the AgentView with text in the input box, presses Esc — the input clears (Esc-cascade behaviour unchanged)
  #
  # ========================================

  Background: User Story
    As a fspec TUI user
    I want to press Esc on the BoardView to exit
    So that I get an exit confirmation dialog matching the TypeScript implementation, and 'q' is no longer a hidden destructive quit binding

  Scenario: Pressing Esc on the BoardView opens an exit confirmation dialog
    Given I am viewing the BoardView with no overlay
    When I press the Esc key
    Then an "Exit fspec?" confirmation dialog appears over the board
    And the application is still running

  Scenario: Pressing 'q' on the BoardView is ignored
    Given I am viewing the BoardView with no overlay
    When I press the 'q' key
    Then no dialog appears
    And the application is still running

  Scenario: Confirming the exit dialog closes the application
    Given the BoardView exit confirmation dialog is showing
    And the "Exit" option is selected
    When I press Enter
    Then the dialog closes
    And the application exits

  Scenario: Cancelling the exit dialog with Esc returns to the board
    Given the BoardView exit confirmation dialog is showing
    When I press the Esc key
    Then the dialog disappears
    And I am returned to the BoardView
    And the application is still running

  Scenario: DisconnectDialog still honors 'q' to quit
    Given the backend connection has dropped
    And the DisconnectDialog is showing
    When I press the 'q' key
    Then the application exits

  Scenario: DisconnectDialog still honors 'r' to reconnect
    Given the backend connection has dropped
    And the DisconnectDialog is showing
    When I press the 'r' key
    Then a manual reconnection attempt is initiated
    And the application is still running

  Scenario: AgentView Esc-cascade still clears non-empty input
    Given I am viewing the AgentView with a session attached
    And the input box contains the text "draft message"
    When I press the Esc key
    Then the input box is cleared
    And no exit confirmation dialog appears
    And the application is still running
