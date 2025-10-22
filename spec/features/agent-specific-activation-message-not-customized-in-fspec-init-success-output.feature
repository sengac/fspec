@done
@multi-agent-support
@cli
@high
@BUG-030
Feature: Agent-specific activation message not customized in fspec init success output

  """
  Helper function getActivationMessage(agentConfig) returns customized message string per agent capabilities.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Activation message must be customized based on detected agent (from INIT-008 implementation)
  #   2. Message appears in 2 places in fspec init output and both must be updated
  #   3. Claude Code users should see: 'Run /fspec in Claude Code to activate'
  #   4. CLI agents (Aider, Gemini) should see agent-specific command format
  #   5. IDE agents (Cursor, Cline) should see instructions relevant to their IDE integration
  #   6. Default/unknown agents should see generic fallback message
  #   7. Two places: (1) src/commands/init.ts:234 in CLI success message (2) src/components/AgentSelector.tsx:56 in UI component
  #
  # EXAMPLES:
  #   1. User runs 'fspec init' with Claude Code detected, sees 'Run /fspec in Claude Code to activate' in 2 places in output
  #   2. User runs 'fspec init' with Cursor detected, sees 'Open .cursor/commands/ in Cursor to activate' in output
  #   3. User runs 'fspec init' with Aider detected, sees 'Add .aider/ to your Aider configuration to activate' in output
  #   4. User runs 'fspec init' with unknown agent, sees generic fallback: 'Refer to your AI agent documentation to activate fspec'
  #
  # QUESTIONS (ANSWERED):
  #   Q: Where exactly are the 2 places in the init command output where this message appears?
  #   A: true
  #
  # ========================================

  Background: User Story
    As a developer running fspec init
    I want to see agent-specific activation instructions in the success message
    So that I know exactly how to activate fspec in my specific AI agent

  Scenario: Claude Code user sees agent-specific activation message
    Given Claude Code is the detected agent
    When fspec init completes successfully
    Then the CLI output should contain "Run /fspec in Claude Code to activate"
    And the UI component should display "Run /fspec in Claude Code to activate"

  Scenario: Cursor user sees IDE-specific activation instructions
    Given Cursor is the detected agent
    When fspec init completes successfully
    Then the CLI output should contain "Open .cursor/commands/ in Cursor to activate"
    And the UI component should display activation instructions for Cursor

  Scenario: Aider user sees CLI-specific activation instructions
    Given Aider is the detected agent
    When fspec init completes successfully
    Then the CLI output should contain "Add .aider/ to your Aider configuration to activate"
    And the UI component should display activation instructions for Aider

  Scenario: Unknown agent receives generic fallback message
    Given an unknown agent is detected
    When fspec init completes successfully
    Then the CLI output should contain "Refer to your AI agent documentation to activate fspec"
    And the UI component should display the generic fallback message
