@done
@multi-agent-support
@cli
@high
@BUG-035
Feature: Duplicate 'Next steps' message in fspec init output

  """
  AgentSelector component (src/components/AgentSelector.tsx:58-61) displays success message with 'Next steps' when agent is selected
  Action handler in init.ts (line 394) also displays success message with 'Next steps' after executeInit completes
  Solution: Remove 'Next steps' section from AgentSelector component (lines 60-61), keep only in action handler
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Next steps message should only appear once in init output
  #   2. AgentSelector component shows success message with Next steps in interactive mode
  #   3. Action handler also shows success message with Next steps for all modes
  #   4. CLI mode should not be affected by this fix
  #
  # EXAMPLES:
  #   1. Developer runs 'fspec init' interactively, selects Claude, sees 'Next steps' message twice
  #   2. Developer runs 'fspec init --agent=claude' in CLI mode, sees 'Next steps' message once
  #   3. After fix: interactive mode should show success message only once with Next steps at the end
  #
  # ========================================

  Background: User Story
    As a developer using fspec init interactively
    I want to complete the initialization process
    So that I see clear next steps only once

  Scenario: Interactive mode shows duplicate 'Next steps' message (BEFORE FIX)
    Given I run 'fspec init' in interactive mode
    When I select Claude from the agent menu
    Then the AgentSelector component displays a success message with 'Next steps:'
    And the action handler also displays a success message with 'Next steps:'
    And I see the message 'Next steps: Run /fspec in Claude Code to activate' twice

  Scenario: CLI mode shows single 'Next steps' message (CURRENT BEHAVIOR)
    Given I run 'fspec init --agent=claude' in CLI mode
    When the installation completes successfully
    Then the action handler displays a success message with 'Next steps:'
    And I see the message 'Next steps: Run /fspec in Claude Code to activate' only once

  Scenario: Interactive mode shows single 'Next steps' message (AFTER FIX)
    Given I run 'fspec init' in interactive mode
    When I select Claude from the agent menu
    Then the AgentSelector component displays a success message WITHOUT 'Next steps:'
    And the action handler displays a success message with 'Next steps:'
    And I see the message 'Next steps: Run /fspec in Claude Code to activate' only once
