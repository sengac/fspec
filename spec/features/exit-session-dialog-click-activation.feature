@done
@bug
@BUG-205
@menu-bar
@agent-view
@dialog
@tui
@regression
Feature: Left-click activates the 'Exit Session?' dialog's Close Session and Cancel buttons
  """
  BUG-205 left the 'Exit Session?' ExitConfirmationDialog mouse-activated:
  left-clicking 'Close Session' commits the close (identical to
  Right,Enter-with-Close-Session-preselected) and left-clicking 'Cancel'
  dismisses the dialog without destroying the session. These scenarios pin
  the post-removal two-button click behavior (TUI-112 dialog-click
  activation, superseded by BUG-205 for the Exit Session dialog).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   R1: The 'Exit Session?' ExitConfirmationDialog offers exactly two flat
  #       options in order [Close Session, Cancel]. The Detach option is
  #       removed — it only dispatched Action::BackToBoard, which plain
  #       navigation already does. ExitChoice::Detach is deleted from the
  #       ExitChoice enum.
  #   R2: Close Session is the default/pre-selected option (index 0). The
  #       dialog's yellow accent, footer, and ESC==Cancel semantics are
  #       unchanged.
  #
  # EXAMPLES:
  #   5. User left-clicks 'Close Session' in the 'Exit Session?' dialog →
  #      the close is committed, the session is destroyed, and the dialog
  #      is removed.
  #   6. User left-clicks 'Cancel' → the dialog dismisses and the session
  #      keeps running.
  #
  # ========================================
  Background: User Story
    As a TUI user closing an agent session
    I want left-click to activate the two-button exit dialog
    So that mouse users get the same close/cancel choices as the keyboard

  @regression
  Scenario: Left-clicking the Close Session button commits the close
    Given the "Exit Session?" dialog is open with "Close Session" highlighted
    When the user left-clicks the "Close Session" button
    Then the exit choice CloseSession is committed
    And the dialog is removed

  @regression
  Scenario: Left-clicking the Cancel button dismisses the dialog without destroying the session
    Given the "Exit Session?" dialog is open with "Close Session" highlighted
    When the user left-clicks the "Cancel" button
    Then the exit choice Cancel is committed
    And the dialog is removed
