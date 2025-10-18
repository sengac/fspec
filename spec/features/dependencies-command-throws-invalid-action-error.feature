@done
@bug-fix
@dependency-management
@cli
@phase1
@BUG-019
Feature: Dependencies command throws 'Invalid action' error

  """
  Root cause: registerDependenciesCommand() uses multi-action router pattern with .argument('<action>') expecting 'list', 'add', 'remove', etc. but help docs suggest direct work-unit-id usage
  Solution: Simplify to single-purpose command accepting .command('dependencies <work-unit-id>') and call showDependencies() directly with AI-friendly error handling
  Error handling: Wrap errors in system-reminder tags to make failures highly visible to AI agents in Claude Code
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Command must accept work-unit-id as first positional argument
  #   2. Command must display all dependency types: blocks, blockedBy, dependsOn, relatesTo
  #   3. Command must provide AI-friendly error wrapped in system-reminder if work unit does not exist
  #
  # EXAMPLES:
  #   1. AI runs 'fspec dependencies MCP-001' and sees 'Dependencies for MCP-001:' with empty lists (no dependencies)
  #   2. AI runs 'fspec dependencies MCP-004' and sees 'Depends on: MCP-001, MCP-002'
  #   3. AI runs 'fspec dependencies INVALID-999' and gets system-reminder with suggestions to run 'fspec list-work-units'
  #
  # ========================================

  Background: User Story
    As a AI agent using fspec CLI
    I want to query work unit dependencies with simple syntax
    So that I can understand dependency relationships without parsing complex multi-action commands

  Scenario: Query dependencies for work unit with no dependencies
    Given I have a work unit "MCP-001" with no dependency relationships
    When I run `fspec dependencies MCP-001`
    Then the command should exit with code 0
    And the output should contain "Dependencies for MCP-001:"
    And the output should not contain any dependency lists

  Scenario: Query dependencies for work unit with dependsOn relationships
    Given I have a work unit "MCP-004" that depends on "MCP-001" and "MCP-002"
    When I run `fspec dependencies MCP-004`
    Then the command should exit with code 0
    And the output should contain "Dependencies for MCP-004:"
    And the output should contain "Depends on: MCP-001, MCP-002"

  Scenario: Query dependencies for non-existent work unit
    Given I do not have a work unit "INVALID-999"
    When I run `fspec dependencies INVALID-999`
    Then the command should exit with code 1
    And the output should contain a system-reminder with dependency query failure message
    And the system-reminder should suggest running "fspec list-work-units"
    And the output should contain "Work unit 'INVALID-999' does not exist"
