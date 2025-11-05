@cli
@configuration
@init
@critical
@INIT-015
Feature: fspec init deletes config and templates instead of preserving them
  """
  Architecture notes:
  - TODO: Add key architectural decisions
  - TODO: Add dependencies and integrations
  - TODO: Add critical implementation requirements
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
    As a developer using fspec init
    I want to switch between agents without losing tool configuration
    So that my configured test and quality check commands persist across agent switches
