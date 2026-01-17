@interaction
@tui
@done
@TUI-048
Feature: Shift+ESC immediate session detach in AgentView

  """
  Shift+ESC should call onExit() immediately without sessionDetach() to provide instant feedback
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Shift+ESC should immediately detach the session without showing the confirmation modal
  #   2. Shift+ESC should override the default ESC key behavior in all states (thinking, inactive, etc.)
  #   3. The input placeholder text should show shift+ESC as an additional detach option
  #   4. The thinking indicator should show shift+ESC as an additional interrupt option
  #
  # EXAMPLES:
  #   1. While agent is thinking, user presses shift+ESC and session immediately detaches without confirmation
  #   2. While session is inactive, user presses shift+ESC and session immediately detaches without confirmation
  #   3. User presses regular ESC while thinking, sees confirmation modal, then presses shift+ESC and immediately detaches
  #   4. User sees input placeholder text includes 'Shift+ESC to detach' alongside existing options
  #   5. User sees thinking indicator text includes 'Shift+ESC to detach' alongside existing ESC interrupt option
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should shift+ESC work in all input states (provider selector, model selector, settings, etc.) or only in the main input/thinking states?
  #   A: Yes, shift+ESC should work in all input states (provider selector, model selector, settings, etc.) as a universal immediate detach override, providing consistent behavior regardless of current state
  #
  #   Q: When detaching via shift+ESC, should there be any visual feedback or should it immediately exit to the previous view?
  #   A: The detach should happen immediately without any visual feedback - just exit to the previous view (like closing the AgentView component and returning to BoardView)
  #
  #   Q: What is the exact text format for the input placeholder? Should it be 'Shift+ESC detach' or 'Shift+Esc' or something else?
  #   A: Use 'Shift+ESC detach' format to be consistent with the existing format like 'Shift+↑/↓' for history navigation
  #
  # ========================================

  Background: User Story
    As a developer using AgentView
    I want to immediately detach from a session using shift+ESC
    So that I can quickly detach without going through the confirmation modal, even while the agent is thinking

  Scenario: Immediate detach while agent is thinking
    Given I am in AgentView with an active session
    And the agent is currently thinking/processing my request
    When I press Shift+ESC
    Then the session should immediately detach without showing confirmation modal
    And I should return to the previous view

  Scenario: Immediate detach while session is inactive
    Given I am in AgentView with an active session
    And the session is inactive (not thinking)
    When I press Shift+ESC
    Then the session should immediately detach without showing confirmation modal
    And I should return to the previous view

  Scenario: Shift+ESC overrides regular ESC confirmation modal
    Given I am in AgentView with an active session
    And the agent is currently thinking
    When I press regular ESC
    Then I should see the detach confirmation modal
    When I then press Shift+ESC
    Then the session should immediately detach without any additional confirmation
    And I should return to the previous view

  Scenario: Input placeholder shows Shift+ESC detach option
    Given I am in AgentView with an active session
    And the session is ready for input
    When I look at the input placeholder text
    Then it should include "Shift+ESC detach" alongside existing options
    And the format should be consistent with other shortcuts like "Shift+↑/↓"

  Scenario: Thinking indicator shows Shift+ESC detach option
    Given I am in AgentView with an active session
    And the agent is currently thinking
    When I look at the thinking indicator text
    Then it should include "Shift+ESC detach" alongside the existing ESC interrupt option

  Scenario: Shift+ESC works in provider selector state
    Given I am in AgentView with an active session
    And the provider selector is currently open
    When I press Shift+ESC
    Then the session should immediately detach without showing confirmation modal
    And I should return to the previous view

  Scenario: Shift+ESC works in model selector state
    Given I am in AgentView with an active session
    And the model selector is currently open
    When I press Shift+ESC
    Then the session should immediately detach without showing confirmation modal
    And I should return to the previous view

  Scenario: Shift+ESC works in settings state
    Given I am in AgentView with an active session
    And the settings panel is currently open
    When I press Shift+ESC
    Then the session should immediately detach without showing confirmation modal
    And I should return to the previous view
