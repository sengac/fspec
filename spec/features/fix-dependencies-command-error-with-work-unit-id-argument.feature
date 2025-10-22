@work-unit-management
@bug-fix
@dependencies
@cli
@BUG-024
Feature: Fix dependencies command error with work unit ID argument
  """
  Root Cause: Legacy dependencies() function (line 1017-1072 in src/commands/dependencies.ts) is still exported and callable. This function expects (action, workUnitId, options) but CLI passes (workUnitId, options) directly to it. Solution: Remove the legacy function since registerDependenciesCommand() now uses showDependencies() instead.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Legacy dependencies() function must be removed to prevent CLI routing conflicts
  #
  # EXAMPLES:
  #   1. User runs 'fspec dependencies RES-001' and receives dependency list instead of 'Invalid action' error
  #   2. Command shows 'Dependencies for RES-001:' followed by relationship lists (blocks, blockedBy, dependsOn, relatesTo)
  #
  # ========================================
  Background: User Story
    As a AI agent using fspec for project management
    I want to view work unit dependencies using 'fspec dependencies <id>'
    So that I can understand dependency relationships and plan work order

  Scenario: Command accepts work unit ID without throwing 'Invalid action' error
    Given I have a work unit "RES-001" with dependencies
    When I run "fspec dependencies RES-001"
    Then the command should exit with code 0
    And the output should NOT contain "Invalid action"
    And the output should contain "Dependencies for RES-001:"

  Scenario: Command displays all relationship types correctly
    Given I have a work unit "RES-001" with the following relationships:
      | type      | target  |
      | dependsOn | MCP-002 |
      | blocks    | MCP-005 |
      | relatesTo | DOC-001 |
    When I run "fspec dependencies RES-001"
    Then the output should contain "Dependencies for RES-001:"
    And the output should contain "Depends on: MCP-002"
    And the output should contain "Blocks: MCP-005"
    And the output should contain "Related to: DOC-001"
