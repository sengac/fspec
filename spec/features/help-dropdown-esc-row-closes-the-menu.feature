@done
@navigation
@tui
@menu-bar
@bug-fix
@rust
@TUI-135
Feature: The Help dropdown's Esc row says 'Esc Exit' although Esc closes the dropdown
  """
  Registry: components/menu_bar/items.rs. The Help category's second row becomes CLOSE_MENU (key 'Esc', label 'Close', description 'Close this menu', MenuAction::CloseMenu). MenuAction::CloseMenu resolves to Action::MenuDismissBar, which the board store (dismiss_menu) and the mux layout (menu_dismiss) both run idempotently after the execute path has already closed the panel, so the executed row only closes the menu. The u Menu bar dialog (components/menu_bar_help.rs) reads items::help_dialog_entries(): the registry rows in order with the Close row swapped for EXIT ('Esc Exit', MenuAction::Exit -> OpenBoardExitConfirmation), so its row indices and the TUI-129 one-key shortcuts stay aligned with CATEGORIES. The board's bare Esc (app/events.rs) still opens the exit confirmation.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Help dropdown's Esc row reads 'Esc Close' with the description 'Close this menu' on the board bar and the mux bar (product owner, 2026-11-08 and 2026-11-09)
  #   2. Enter or a left-click on the Close row closes the dropdown and de-selects the bar item, like every executed row (BUG-196 R2), and opens nothing
  #   3. Exit stays reachable: Esc on the board with nothing open opens the Exit fspec? confirmation, and the u Menu bar dialog keeps its 'Esc Exit' row whose Enter opens it (product owner, 2026-11-09)
  #   4. The registry stays the one source: the Help category's Close row is the dropdown row; the u dialog lists the same rows in the same order with Exit in that slot, so the row indices and one-key shortcuts are unchanged
  #
  # EXAMPLES:
  #   1. Board bar, Help dropdown open: the rows read '? Help' and 'Esc Close'
  #   2. Board bar, Help dropdown on the Close row, Enter: the dropdown closes, the bar item is de-selected and no dialog opens
  #   3. Board bar, Help dropdown open, left-click on the Close row: the dropdown closes, the bar item is de-selected and no exit confirmation opens
  #   4. Mux bar, Help dropdown on the Close row, Enter: the mux dropdown closes and the bar is de-selected; no dialog opens
  #   5. u dialog on its last row 'Esc Exit', Enter: the Exit fspec? confirmation opens
  #
  # ========================================
  Background: User Story
    As a fspec TUI user with a menu dropdown open
    I want to have the Help dropdown's Esc row say what Esc does there and close the menu when chosen
    So that the row never promises an exit that Esc does not perform

  Scenario: The Help dropdown's Esc row reads Esc Close
    Given the board with the Help dropdown open
    When the dropdown is painted
    Then its rows read "? Help" and "Esc Close"
    And the Esc row's description reads "Close this menu"
    And no dropdown row reads "Esc Exit"

  Scenario: Enter on the Close row closes the menu and opens nothing
    Given the board with the Help dropdown open on the Close row
    When the user presses Enter
    Then the dropdown is closed
    And the bar item is de-selected
    And no dialog opens

  Scenario: Clicking the Close row closes the menu and opens nothing
    Given the board with the Help dropdown open on the Help row
    When the user left-clicks the Close row
    Then the dropdown is closed
    And the bar item is de-selected
    And the exit confirmation does not open

  Scenario: Enter on the mux bar's Close row closes the mux menu
    Given the mux view with its bar's Help dropdown open on the Close row
    When the user presses Enter
    Then the mux dropdown is closed
    And the mux bar item is de-selected
    And no dialog opens

  Scenario: Esc on the board with nothing open still opens the exit confirmation
    Given the board with no menu or dialog open
    When the user presses Esc
    Then the exit confirmation opens

  Scenario: The u dialog keeps its Esc Exit row
    Given the u Menu bar dialog open on its last row
    When the user presses Enter
    Then the exit confirmation opens
    And the row read "Esc Exit" with the description "Confirm exiting fspec"
