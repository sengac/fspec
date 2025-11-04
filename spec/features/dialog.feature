@done
@modal
@react
@ink
@ui-enhancement
@tui
@critical
@TUI-018
Feature: Base Dialog modal infrastructure component

  """
  Accepts children prop for dialog content - implements composition pattern
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Dialog handles ONLY modal overlay infrastructure - centering, borders, ESC key, layout. No business logic.
  #   2. Dialog uses isActive prop to control useInput hook - when active, captures all keyboard input
  #   3. Dialog accepts children prop for content - implements composition pattern
  #   4. Dialog provides borderColor prop for customization (red/yellow/green/undefined)
  #
  # EXAMPLES:
  #   1. Dialog component renders centered modal with red border when borderColor='red' prop provided, handles ESC to call onClose
  #   2. User is in BoardDisplay, opens dialog - dialog captures ALL input, board navigation suspended until dialog closes
  #   3. Developer creates InfoDialog wrapping Dialog with custom OK button - Dialog provides modal, InfoDialog provides button logic
  #
  # ========================================

  Background: User Story
    As a TUI developer
    I want to use a reusable base Dialog component for modal overlays
    So that I can create consistent dialogs without duplicating modal infrastructure code

  Scenario: Render centered modal with custom border color
    Given a Dialog component with borderColor='red' and children 'Test Content'
    When the Dialog is rendered
    Then a centered modal should be displayed
    And the border should be red
    And the children 'Test Content' should be visible


  Scenario: Dialog captures all input when active
    Given a Dialog component with isActive=true and onClose callback
    When the user presses a key
    Then the Dialog should capture the input first
    And a parent component with its own useInput handler
    And the parent useInput handler should not receive the input


  Scenario: Handle ESC key to call onClose
    Given a Dialog component with onClose callback
    When the user presses the ESC key
    Then the onClose callback should be called

