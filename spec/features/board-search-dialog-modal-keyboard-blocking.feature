@done
@tui
@board
@dialog
@search
@bug-161
Feature: Board search dialog is a true modal that blocks board keyboard shortcuts
  """
  BUG-161: While the BOARD-022 WorkUnitSearchDialog is open, keys the dialog
  does not explicitly handle (j/k/h/l, [, ], f, c, d, a, ., ?, Shift+Right,
  Enter with zero matches) returned Ignored and fell through the Compositor
  to the BoardView behind the modal, moving the selection or opening other
  views. The dialog's catch-all arm and the SHIFT/CTRL modifier guard now
  return Consumed (a no-op) so the dialog fully owns the keyboard while it
  is open. The dialog's own explicit arms (Esc, Tab, Backspace, '/',
  printable chars, Up/Down/PageUp/PageDown/Home/End, Enter) keep their
  current behavior unchanged.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. While the search dialog is open, any key it does not explicitly
  #      handle (j/k/h/l, [, ], f, c, d, a, ., ?, and any other unhandled
  #      key) is CONSUMED as a no-op so the BoardView behind the modal is
  #      frozen
  #   2. SHIFT/CTRL-chorded keys are CONSUMED by the dialog (the modifier
  #      guard returns Consumed instead of Ignored), so chords like
  #      Shift+Right cannot reach the board or the App-level handler
  #   3. The dialog's own explicit arms (Esc, Tab, Backspace, '/',
  #      printable chars, Up/Down/PageUp/PageDown/Home/End, Enter) keep
  #      their current behavior unchanged — the fix only changes the
  #      catch-all and the modifier guard to Consumed
  #
  # EXAMPLES:
  #   - With the search dialog open and zero matches, pressing 'j' does not
  #     move the board selection — the dialog consumes the key and the
  #     board stays frozen
  #   - While typing in the search box, pressing 'f' does not open the
  #     Changed Files view and pressing 'd' does not open the Foundation
  #     doc — the board shortcuts stay frozen behind the modal
  #   - The dialog's own keys still work while the board is frozen: Tab
  #     still cycles the search mode and a printable character still edits
  #     the query
  #
  # QUESTIONS:
  #   (none — the modal contract is locked by the research note)
  # ========================================
  # SCENARIOS
  # ========================================
  # @BUG-161
  Scenario: Board navigation keys are consumed while the dialog is open
    Given the work-unit search dialog is open with zero matches
    When I press the "j" key
    Then the dialog consumes the key
    And the board selection is unchanged

  Scenario: Board view-opening shortcuts are consumed while the dialog is open
    Given the work-unit search dialog is open
    When I press one of the keys "f", "c", "d", "a", "."
    Then the dialog consumes the key
    And no board action is emitted

  Scenario: Modifier-chorded keys are consumed while the dialog is open
    Given the work-unit search dialog is open
    When I press Shift+Right
    Then the dialog consumes the key
    And no agent view is opened

  Scenario: The help key is consumed while the dialog is open
    Given the work-unit search dialog is open
    When I press the "?" key
    Then the dialog consumes the key
    And the help dialog does not open

  Scenario: Enter with zero matches is consumed and does not enter a work unit
    Given the work-unit search dialog is open with zero matches
    When I press Enter
    Then the dialog consumes the key
    And no work unit is entered

  Scenario: The dialog's own keys still work while the board is frozen
    Given the work-unit search dialog is open in Id mode with a match list
    When I press Tab
    Then the dialog shows the Title search mode
    When I type "auth" into the dialog
    Then the dialog's query is "auth"
    When I press Esc
    Then the work-unit search dialog is closed

  Scenario: Unmodified arrow keys are consumed while the dialog is open
    Given the work-unit search dialog is open
    When I press Left and Right
    Then the dialog consumes the key
    And the board column focus is unchanged

