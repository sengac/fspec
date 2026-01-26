@done
@TUI-050
Feature: Slash Command Autocomplete Palette
  """
  Integrate with MultiLineInput.tsx for trigger detection and AgentView.tsx/SplitSessionView.tsx for rendering
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Palette appears when / is typed at position 0 in the input area
  #   2. Palette floats above the input area, positioned within terminal bounds
  #   3. Commands are filtered as user types after /, using three-tier matching: prefix first, then substring, then description
  #   4. Keyboard navigation: Up/Down to navigate, Tab/Enter to select, Escape to close, Backspace past / to close
  #   5. Selected command is highlighted with distinct background color
  #   6. Each command shows name and description in the palette
  #   7. Palette integrates with both AgentView and SplitSessionView components
  #   8. When no commands match the filter, display 'No matching commands' message
  #
  # EXAMPLES:
  #   1. User types / at start of input → palette appears with all commands listed
  #   2. User types /m → palette shows filtered commands: model, mode, merge, mcp
  #   3. User presses Down arrow twice → selection moves to third command in list
  #   4. User presses Tab on selected command → input updates to '/model ' and palette closes
  #   5. User presses Escape while palette is open → palette closes and input remains unchanged
  #   6. User types /xyz (no matching commands) → palette shows 'No matching commands'
  #   7. User deletes / character with Backspace → palette closes
  #   8. User presses Enter on command without arguments → command executes immediately
  #
  # ========================================
  Background: User Story
    As a user interacting with fspec TUI
    I want to see autocomplete suggestions when typing slash commands
    So that I can quickly discover and select commands without memorizing them

  Scenario: Show palette when typing slash at start of input
    Given I am in the TUI input area with empty input
    When I type "/" at position 0
    Then the slash command palette should appear
    And the palette should show all available commands
    And the first command should be selected

  Scenario: Filter commands by prefix matching
    Given the slash command palette is visible
    And the input contains "/"
    When I type "m" after the slash
    Then the palette should show only commands starting with "m"
    And the commands "model", "mode", "merge", "mcp" should be visible
    And the first matching command should be selected

  Scenario: Navigate through command list with arrow keys
    Given the slash command palette is visible with multiple commands
    And the first command is selected
    When I press the Down arrow key twice
    Then the third command in the list should be selected
    And the selection highlight should move accordingly

  Scenario: Accept selected command with Tab key
    Given the slash command palette is visible
    And the "model" command is selected
    When I press the Tab key
    Then the input should update to "/model "
    And the palette should close

  Scenario: Close palette with Escape key
    Given the slash command palette is visible
    And the input contains "/m"
    When I press the Escape key
    Then the palette should close
    And the input should remain unchanged as "/m"

  Scenario: Show no matching commands message
    Given the slash command palette is visible
    When I type "/xyz"
    Then the palette should display "No matching commands"
    And no command should be selectable

  Scenario: Close palette when deleting slash character
    Given the slash command palette is visible
    And the input contains only "/"
    When I press Backspace to delete the slash
    Then the palette should close
    And the input should be empty

  Scenario: Execute command immediately with Enter key
    Given the slash command palette is visible
    And the "clear" command is selected
    When I press the Enter key
    Then the command should execute immediately
    And the palette should close
    And the input should be cleared
