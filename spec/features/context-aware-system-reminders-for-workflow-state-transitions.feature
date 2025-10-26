@done
@system-reminder
@work-management
@cli
@high
@REMIND-009
Feature: Context-aware system-reminders for workflow state transitions
  """
  Modify update-work-unit-status command to emit state-specific system-reminders. Create mapping of workflow states to command lists. Format output as vertical list with full syntax placeholders. Reference existing system-reminder patterns in codebase for consistency.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. System-reminders should appear when work unit status changes via update-work-unit-status command
  #   2. Each workflow state (backlog, specifying, testing, implementing, validating) has its own set of commonly used commands
  #   3. Reminders must be concise - show only 5-7 most common commands per state, not every possible command
  #   4. Each command should have brief syntax example showing arguments (e.g., 'fspec remove-rule <id> <index>' not just 'remove-rule')
  #   5. Include pointer to help documentation at end (e.g., 'For complete reference: fspec help discovery' or 'fspec <command> --help')
  #   6. Done and blocked states do not need command reminders (blocked needs issue resolution, done is complete)
  #
  # EXAMPLES:
  #   1. When work unit moves to SPECIFYING, AI sees reminder: 'Common commands: fspec add-rule <id> "rule" | fspec remove-rule <id> <index> | fspec add-example <id> "example" | fspec remove-example <id> <index> | fspec add-question <id> "@human: question?" | fspec answer-question <id> <index> --answer "..." | fspec generate-scenarios <id>. For more: fspec help discovery'
  #   2. When work unit moves to TESTING, AI sees reminder: 'Common commands: fspec link-coverage <feature> --scenario "..." --test-file <path> --test-lines <range> | fspec show-coverage <feature> | fspec show-feature <name>. Write tests that map to Gherkin scenarios. For more: fspec link-coverage --help'
  #   3. When work unit moves to IMPLEMENTING, AI sees reminder: 'Common commands: fspec link-coverage <feature> --scenario "..." --impl-file <path> --impl-lines <lines> | fspec checkpoint <id> <name> | fspec restore-checkpoint <id> <name> | fspec list-checkpoints <id>. Write minimal code to pass tests. For more: fspec checkpoint --help'
  #   4. When work unit moves to VALIDATING, AI sees reminder: 'Common commands: fspec validate | fspec validate-tags | fspec check | fspec audit-coverage <feature> | npm test | npm run check. Run ALL tests to ensure nothing broke. For more: fspec check --help'
  #   5. When work unit moves to BACKLOG (deprioritization), no command reminder is shown since backlog is a waiting state with no active work
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the reminder show full command syntax with <placeholders> for arguments, or just command names with brief descriptions?
  #   A: true
  #
  #   Q: Should commands be shown one per line (vertical list) or separated by pipe | (horizontal compact)?
  #   A: true
  #
  #   Q: Should BACKLOG state show a reminder about prioritization commands (prioritize-work-unit, show-work-unit), or skip reminders entirely since it's a waiting state?
  #   A: true
  #
  #   Q: Should VALIDATING state include non-fspec commands (npm test, npm run check) in the reminder, or only fspec commands?
  #   A: true
  #
  #   Q: Should the reminder appear every time status changes, or only the first time entering each state (cached per work unit)?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a AI agent working on fspec project
    I want to see relevant commands when entering a new workflow state
    So that I can discover and use the right commands without guessing or searching documentation

  Scenario: Show command reminder when transitioning to SPECIFYING state
    Given I have a work unit in backlog status
    When I run "fspec update-work-unit-status <id> specifying"
    Then a system-reminder should be emitted
    And the reminder should contain "fspec add-rule <id> \"rule\""
    And the reminder should contain "fspec remove-rule <id> <index>"
    And the reminder should contain "fspec add-example <id> \"example\""
    And the reminder should contain "fspec remove-example <id> <index>"
    And the reminder should contain "fspec add-question <id> \"@human: question?\""
    And the reminder should contain "fspec answer-question <id> <index> --answer \"...\""
    And the reminder should contain "fspec generate-scenarios <id>"
    And the reminder should contain "For more: fspec help discovery"
    And commands should be displayed one per line

  Scenario: Show command reminder when transitioning to TESTING state
    Given I have a work unit in specifying status
    When I run "fspec update-work-unit-status <id> testing"
    Then a system-reminder should be emitted
    And the reminder should contain "fspec link-coverage <feature> --scenario \"...\" --test-file <path> --test-lines <range>"
    And the reminder should contain "fspec show-coverage <feature>"
    And the reminder should contain "fspec show-feature <name>"
    And the reminder should contain "For more: fspec link-coverage --help"
    And commands should be displayed one per line

  Scenario: Show command reminder when transitioning to IMPLEMENTING state
    Given I have a work unit in testing status
    When I run "fspec update-work-unit-status <id> implementing"
    Then a system-reminder should be emitted
    And the reminder should contain "fspec link-coverage <feature> --scenario \"...\" --impl-file <path> --impl-lines <lines>"
    And the reminder should contain "fspec checkpoint <id> <name>"
    And the reminder should contain "fspec restore-checkpoint <id> <name>"
    And the reminder should contain "fspec list-checkpoints <id>"
    And the reminder should contain "For more: fspec checkpoint --help"
    And commands should be displayed one per line

  Scenario: Show command reminder when transitioning to VALIDATING state
    Given I have a work unit in implementing status
    When I run "fspec update-work-unit-status <id> validating"
    Then a system-reminder should be emitted
    And the reminder should contain "fspec validate"
    And the reminder should contain "fspec validate-tags"
    And the reminder should contain "fspec check"
    And the reminder should contain "fspec audit-coverage <feature>"
    And the reminder should NOT contain "npm test"
    And the reminder should NOT contain "npm run check"
    And the reminder should contain "For more: fspec check --help"
    And commands should be displayed one per line

  Scenario: No command reminder when transitioning to BACKLOG state
    Given I have a work unit in specifying status
    When I run "fspec update-work-unit-status <id> backlog"
    Then no command reminder system-reminder should be emitted

  Scenario: No command reminder when transitioning to DONE state
    Given I have a work unit in validating status
    When I run "fspec update-work-unit-status <id> done"
    Then no command reminder system-reminder should be emitted

  Scenario: No command reminder when transitioning to BLOCKED state
    Given I have a work unit in implementing status
    When I run "fspec update-work-unit-status <id> blocked"
    Then no command reminder system-reminder should be emitted

  Scenario: Show reminder every time status changes
    Given I have a work unit that has previously been in specifying status
    And I moved it to testing and saw a testing reminder
    When I move it back to specifying status
    Then a system-reminder for specifying state should be emitted again
    And it should show the same commands as the first time
