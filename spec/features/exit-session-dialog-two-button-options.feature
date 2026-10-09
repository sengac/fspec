@done
@bug
@BUG-205
@menu-bar
@agent-view
@dialog
@tui
@regression
Feature: The 'Exit Session?' dialog offers exactly two options (Close Session / Cancel) with keyboard semantics
  """
  BUG-205 removed the pointless 'Detach' option from the 'Exit Session?'
  ExitConfirmationDialog (opened by the agent bar's 'Close Agent' button and by
  the ESC cascade level 7). Detach only dispatched Action::BackToBoard — i.e.
  navigate away while the session keeps running — which plain navigation
  (menu bar Board item, Shift+Left, mux board-pane focus) already does.

  The dialog is reduced to the two flat options [Close Session, Cancel] with
  Close Session pre-selected (index 0), matching the board 'Exit fspec?'
  BoardExitConfirmationDialog convention (destructive option pre-selected,
  Esc cancels). This feature pins the keyboard-side behavior: dialog open,
  cyclic Left/Right navigation, Enter on the pre-selected Close Session,
  Right+Enter committing Cancel, and ESC == Cancel.
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
  #   R2: Close Session is the default/pre-selected option (index 0), matching
  #       the board 'Exit fspec?' convention of pre-selecting the destructive
  #       option. Left from Close Session wraps to Cancel; Right from Cancel
  #       wraps to Close Session. The dialog's busy/idle description variants,
  #       yellow accent, footer, and ESC==Cancel semantics are unchanged.
  #
  # EXAMPLES:
  #   1. User presses ESC in an idle AgentView (L7) → the 'Exit Session?'
  #      dialog shows exactly two buttons [Close Session, Cancel] with
  #      'Close Session' pre-selected (blue bg/white/bold), 'Cancel' in gray;
  #      the 'Detach' label is nowhere painted.
  #   2. Dialog open with Close Session pre-selected → Enter destroys the
  #      backend session, clears the BoardStore attachment, and returns to
  #      the Board view.
  #
  # ========================================
  Background: User Story
    As a TUI user closing an agent session
    I want to see a two-button exit dialog (Close Session / Cancel)
    So that the exit dialog no longer offers a pointless option that duplicates plain navigation

  @regression
  Scenario: Exit dialog shows exactly two options with Close Session pre-selected
    Given I am in the Rust AgentView with an active session whose status is Idle
    And the input buffer is empty
    And no popup or mode view is currently active
    When I press ESC once
    Then an ExitConfirmationDialog is pushed onto the compositor
    And the dialog renders a yellow rounded border centred on screen
    And the title row reads "Exit Session?" in bold
    And the description row reads "Choose how to exit the session." in dim text
    And the button "Close Session" is pre-selected with blue background and white foreground
    And the button "Cancel" is rendered in gray
    And no "Detach" button is painted anywhere in the dialog
    And the footer reads "← → Navigate | Enter Select | Esc Cancel" in dim text

  @regression
  Scenario: Cyclic Left/Right navigation across the two options
    Given the ExitConfirmationDialog is open with Close Session focused
    When I press Left
    Then Cancel is focused
    When I press Right
    Then Close Session is focused
    When I press Right
    Then Cancel is focused

  @regression
  Scenario: Enter on the pre-selected Close Session destroys the session and returns to Board
    Given the ExitConfirmationDialog is open with Close Session pre-selected
    And the current AgentView session is attached to work unit "AUTH-001" in BoardStore
    And the backend records every destroy_session call
    When I press Enter
    Then Action::AgentExitChoice { choice: CloseSession } is emitted
    And the ExitConfirmationDialog is removed from the compositor
    And the App spawns a backend.destroy_session task for session "s-1"
    And the BoardStore work-unit-to-session attachment for "AUTH-001" is cleared
    And the navigator switches to the Board view
    And the destroyed session is removed from AgentViewStore open_sessions

  @regression
  Scenario: Right then Enter on the exit dialog commits Cancel and stays on AgentView
    Given the ExitConfirmationDialog is open with Close Session pre-selected
    When I press Right once
    Then Cancel is focused
    When I press Enter
    Then Action::AgentExitChoice { choice: Cancel } is emitted
    And the ExitConfirmationDialog is removed from the compositor
    And the navigator remains on the Agent view
    And no Action::BackToBoard is dispatched
    And no backend.destroy_session task is spawned

  @regression
  Scenario: ESC inside the dialog is still equivalent to Cancel
    Given the ExitConfirmationDialog is open with Close Session focused
    When I press ESC
    Then Action::AgentExitChoice { choice: Cancel } is emitted
    And the ExitConfirmationDialog is removed from the compositor
    And the navigator remains on the Agent view
    And no backend.destroy_session task is spawned
