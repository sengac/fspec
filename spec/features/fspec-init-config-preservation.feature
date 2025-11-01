@workflow
@multi-agent-support
@critical
@configuration
@cli
@done
@INIT-015
Feature: fspec init deletes config and templates instead of preserving them
  """

  Key architectural decisions:
  - writeAgentConfig must use read-modify-write pattern instead of complete overwrite
  - Only the 'agent' field should be updated, all other config fields preserved
  - removeOtherAgentFiles function should be removed entirely (previous requirement that's no longer valid)
  - Config structure: {agent: string, tools?: {test?: {command?: string}, qualityCheck?: {commands?: string[]}}, ...}

  Dependencies and integrations:
  - src/utils/agentRuntimeConfig.ts - writeAgentConfig function (lines 65-77)
  - src/commands/init.ts - removeOtherAgentFiles function (lines 193-223)
  - spec/fspec-config.json - Configuration file location

  Critical implementation requirements:
  - MUST preserve existing config fields when updating agent field
  - MUST NOT delete templates for other installed agents
  - MUST read existing config, merge agent field, write back
  - Tests expecting template deletion behavior need to be updated/removed

  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. writeAgentConfig in agentRuntimeConfig.ts (lines 65-77) creates new config object with only agent field, overwriting existing config
  #   2. removeOtherAgentFiles in init.ts (lines 193-223) deletes doc templates and slash commands for agents not being installed
  #   3. Config file may contain: agent, tools.test.command, tools.qualityCheck.commands, and other user settings
  #   4. Template deletion was a previous requirement that needs to be removed - users want to keep templates for all installed agents
  #
  # EXAMPLES:
  #   1. User has config with tools configured, runs fspec init to switch from claude to cursor, config is overwritten with only {agent: cursor}, tool config lost
  #   2. User has both Claude and Cursor templates installed, switches to Aider, Claude and Cursor templates are deleted
  #   3. CORRECT: User has config {agent: claude, tools: {...}}, runs fspec init cursor, config becomes {agent: cursor, tools: {...}}, templates preserved
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should writeAgentConfig merge with existing config or read-modify-write? Current behavior is complete overwrite.
  #   A: true
  #
  #   Q: Should removeOtherAgentFiles be removed entirely, or modified to only remove when explicitly requested?
  #   A: true
  #
  #   Q: Are there tests that expect template deletion behavior that will need to be updated or removed?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer using fspec
    I want to switch between AI agents using fspec init
    So that my existing configuration and templates are preserved

  Scenario: Preserve existing tool configuration when switching agents
    Given I have a config file with agent 'claude' and tools configuration
    When I run 'fspec init cursor'
    Then the config file should have agent set to 'cursor'
    And the tools configuration should be preserved

  Scenario: Preserve templates for all installed agents when switching agents
    Given I have templates for both 'claude' and 'cursor' agents installed
    When I run 'fspec init aider'
    Then the 'claude' templates should still exist
    And the 'cursor' templates should still exist
    And the 'aider' templates should be created

  Scenario: Switch agents while preserving all existing configuration
    Given I have a config file with agent 'claude', test command 'npm test', and quality check commands
    When I run 'fspec init cursor'
    Then the config file should have agent set to 'cursor'
    And the test command should still be 'npm test'
    And the quality check commands should be preserved
