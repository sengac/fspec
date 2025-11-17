@done
@high
@foundation-management
@cli
@automation
@FOUND-013
Feature: Auto-create Big Picture Event Storming work unit after foundation finalization
  """
  Architecture notes:
  - Modifies src/commands/discover-foundation.ts to auto-create work unit after successful finalization
  - Uses existing getNextWorkUnitId() utility for FOUND prefix ID generation
  - Work unit includes description with foundation Event Storm commands and CLAUDE.md reference
  - Implementation adds logic after foundation.json write (line ~370-390 in discoverFoundation function)
  - No new commands needed - reuses existing Event Storming infrastructure (SOLID/DRY principles)
  - Work unit creation happens ONLY when --finalize flag used AND validation passes
  - Console output styled with chalk.green() to match existing output format
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Work unit is created ONLY when --finalize flag is used
  #   2. Work unit is created ONLY when validation passes
  #   3. Work unit ID uses FOUND prefix with next available number
  #   4. Work unit description includes Event Storming commands and CLAUDE.md reference
  #   5. Console output confirms work unit creation with ID and command to view details
  #
  # EXAMPLES:
  #   1. User runs 'fspec discover-foundation --finalize', validation passes, work unit FOUND-XXX is created in backlog with Big Picture Event Storming guidance
  #   2. User runs 'fspec discover-foundation' without --finalize flag, NO work unit is created
  #   3. User runs 'fspec discover-foundation --finalize' but validation fails, NO work unit is created
  #   4. AI agent reads created work unit description and sees list of foundation Event Storm commands (add-foundation-bounded-context, add-aggregate-to-foundation, etc.)
  #
  # ========================================
  Background: User Story
    As a AI agent using fspec
    I want to be prompted to conduct Big Picture Event Storming after foundation discovery
    So that I capture domain architecture in foundation.json eventStorm field

  Scenario: Work unit created when foundation finalized successfully
    Given foundation discovery has been completed
    And all required foundation fields are populated
    When I run "fspec discover-foundation --finalize"
    And validation passes
    Then a new work unit should be created with FOUND prefix
    And the work unit status should be "backlog"
    And the work unit type should be "task"
    And the work unit title should contain "Big Picture Event Storming"
    And the work unit description should include foundation Event Storm commands
    And the work unit description should reference CLAUDE.md documentation
    And console output should confirm work unit creation
    And console output should show command to view work unit details

  Scenario: Work unit NOT created without finalize flag
    Given foundation discovery has been completed
    And all required foundation fields are populated
    When I run "fspec discover-foundation" without --finalize flag
    Then NO work unit should be created
    And work-units.json should remain unchanged

  Scenario: Work unit NOT created when validation fails
    Given foundation discovery has been completed
    And foundation draft has validation errors
    When I run "fspec discover-foundation --finalize"
    And validation fails
    Then NO work unit should be created
    And work-units.json should remain unchanged
    And error message should explain validation failure

  Scenario: Work unit description contains Event Storming guidance
    Given foundation has been finalized successfully
    And a Big Picture Event Storming work unit was created
    When AI agent reads the work unit description
    Then the description should list "add-foundation-bounded-context" command
    And the description should list "add-aggregate-to-foundation" command
    And the description should list "add-domain-event-to-foundation" command
    And the description should list "show-foundation-event-storm" command
    And the description should explain why Big Picture Event Storming matters
    And the description should reference "spec/CLAUDE.md" for detailed guidance
