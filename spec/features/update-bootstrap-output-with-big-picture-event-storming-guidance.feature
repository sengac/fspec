@discovery
@done
@high
@cli
@automation
@foundation-management
@FOUND-015
Feature: Update bootstrap output with Big Picture Event Storming guidance
  """
  Architecture notes:
  - Modifies src/commands/bootstrap.ts to emit system-reminder when Event Storm needed
  - Detection logic: Check foundation.json exists, eventStorm empty, find FOUND-XXX work units
  - System-reminder emitted AFTER CLAUDE.md content, BEFORE final "fspec mode" message
  - Uses existing wrapInSystemReminder utility for consistent formatting
  - Reuses existing file reading utilities (readFile, existsSync, JSON.parse)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. System-reminder emitted ONLY when foundation.json exists
  #   2. System-reminder emitted ONLY when eventStorm field is empty or missing
  #   3. System-reminder references work unit ID if FOUND-XXX Event Storm work unit exists and not done
  #   4. System-reminder appears AFTER CLAUDE.md content, BEFORE final message
  #   5. System-reminder provides commands, CLAUDE.md reference, and explains why Event Storm matters
  #
  # EXAMPLES:
  #   1. AI runs 'fspec bootstrap', foundation.json exists with empty eventStorm, FOUND-XXX work unit exists in backlog, system-reminder emitted with work unit ID and next steps
  #   2. AI runs 'fspec bootstrap', foundation.json exists with empty eventStorm, NO work unit exists, system-reminder emitted suggesting to create work unit or run commands directly
  #   3. AI runs 'fspec bootstrap', foundation.json exists with populated eventStorm field, NO system-reminder emitted
  #   4. AI runs 'fspec bootstrap', NO foundation.json exists, NO system-reminder emitted
  #
  # ========================================
  Background: User Story
    As a AI agent using fspec
    I want to be reminded to conduct Big Picture Event Storming when foundation exists but eventStorm field is empty
    So that I don't skip critical domain architecture discovery step

  Scenario: Reminder emitted when eventStorm empty and work unit exists
    Given foundation.json exists with empty eventStorm field
    And a FOUND-XXX Event Storm work unit exists in backlog status
    When I run "fspec bootstrap"
    Then a system-reminder should be emitted
    And the reminder should reference the work unit ID
    And the reminder should provide next steps to work on the work unit
    And the reminder should list foundation Event Storm commands
    And the reminder should reference CLAUDE.md documentation
    And the reminder should explain why Event Storm matters
    And the reminder should appear after CLAUDE.md content

  Scenario: Reminder emitted when eventStorm empty and no work unit
    Given foundation.json exists with empty eventStorm field
    And NO Event Storm work unit exists
    When I run "fspec bootstrap"
    Then a system-reminder should be emitted
    And the reminder should suggest creating a work unit OR running commands directly
    And the reminder should list foundation Event Storm commands
    And the reminder should reference CLAUDE.md documentation
    And the reminder should explain why Event Storm matters

  Scenario: No reminder when eventStorm already populated
    Given foundation.json exists with populated eventStorm field
    When I run "fspec bootstrap"
    Then NO system-reminder should be emitted about Event Storm
    And bootstrap output should show normal CLAUDE.md content

  Scenario: No reminder when foundation.json does not exist
    Given foundation.json does NOT exist
    When I run "fspec bootstrap"
    Then NO system-reminder should be emitted about Event Storm
    And bootstrap output should show normal CLAUDE.md content
