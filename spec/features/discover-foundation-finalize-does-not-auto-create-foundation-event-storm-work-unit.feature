@cli
@foundation-management
@work-unit-automation
@critical
@command
@foundation
@bug-fix
@BUG-084
Feature: discover-foundation --finalize does not auto-create Foundation Event Storm work unit

  """
  Bug in src/commands/discover-foundation.ts --finalize logic. Missing auto-creation of FOUND prefix and Foundation Event Storm work unit. Fix: After successful finalization, check if FOUND prefix exists (create if not), then create work unit with proper description. CRITICAL: Tests MUST use isolated tmpdir, never write to actual spec/work-units.json
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When discover-foundation --finalize succeeds, it MUST auto-create a FOUND-XXX work unit for Foundation Event Storm
  #   2. The FOUND prefix MUST be auto-registered if it doesn't exist
  #   3. Tests MUST use isolated temporary directories and NEVER write to the actual spec/work-units.json file
  #
  # EXAMPLES:
  #   1. First finalize creates FOUND-001 work unit with title 'Foundation Event Storm' in isolated test directory
  #   2. FOUND prefix is auto-registered with description 'Foundation Event Storm tasks' if it doesn't exist
  #   3. Running finalize twice does NOT create duplicate work units (idempotency check in isolated test)
  #
  # ========================================

  Background: User Story
    As a developer using fspec for foundation discovery
    I want to have Foundation Event Storm work unit auto-created after finalization
    So that I can immediately proceed with Event Storm workflow without manual setup

  Scenario: First finalize auto-creates FOUND-001 work unit in isolated test directory
    Given I am in an isolated temporary test directory
    And I have a completed foundation.json.draft file
    And the FOUND prefix does not exist yet
    When I run "fspec discover-foundation --finalize"
    Then the command should succeed
    And a work unit "FOUND-001" should be created with title "Foundation Event Storm"
    And the work unit should have status "backlog"
    And the work unit description should mention Event Storm workflow
    And the test MUST NOT write to the actual spec/work-units.json file

  Scenario: FOUND prefix is auto-registered if it doesn't exist
    Given I am in an isolated temporary test directory
    And I have a completed foundation.json.draft file
    And the FOUND prefix does not exist in prefixes
    When I run "fspec discover-foundation --finalize"
    Then the FOUND prefix should be registered
    And the prefix description should be "Foundation Event Storm tasks"
    And the test MUST NOT write to the actual spec/work-units.json file

  Scenario: Running finalize twice does NOT create duplicate work units (idempotency)
    Given I am in an isolated temporary test directory
    And I have already run "fspec discover-foundation --finalize" once successfully
    And work unit "FOUND-001" already exists
    When I run "fspec discover-foundation --finalize" again
    Then the command should succeed
    And NO new work unit should be created
    And FOUND-001 should still be the only FOUND work unit
    And the test MUST NOT write to the actual spec/work-units.json file
