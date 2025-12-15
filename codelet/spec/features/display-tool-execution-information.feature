@high
@cli
@interactive
@tool-display
@CLI-003
Feature: Display tool execution information
  """
  Based on codelet runner.ts:1038-1040 pattern. Intercept rig MultiTurnStreamItem stream and display tool calls before execution. Uses format: [Planning to use tool: <name>]
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. System must display tool name when agent plans to use a tool
  #   2. Tool display format must match codelet: [Planning to use tool: <tool_name>]
  #   3. Tool display must appear before tool execution in the stream
  #
  # EXAMPLES:
  #   1. Agent calls Read tool to read a file, display shows: [Planning to use tool: read]
  #   2. Agent calls multiple tools (grep, read, edit), each tool displays individually before execution
  #   3. User asks 'list all rust files', agent shows [Planning to use tool: glob] before executing
  #
  # ========================================
  Background: User Story
    As a developer using codelet
    I want to see which tools the agent is planning to use
    So that I can understand what the agent is doing and follow its execution flow

  Scenario: Display tool name before execution
    Given I am running codelet in interactive mode
    When the agent decides to call the Read tool
    Then I should see "[Planning to use tool: read]" in the output
    And the message should appear before the tool executes

  Scenario: Display multiple tool calls in sequence
    Given the agent plans to use multiple tools (grep, read, edit)
    When the agent executes the tools
    Then I should see "[Planning to use tool: grep]" before grep executes
    And I should see "[Planning to use tool: read]" before read executes
    And I should see "[Planning to use tool: edit]" before edit executes
