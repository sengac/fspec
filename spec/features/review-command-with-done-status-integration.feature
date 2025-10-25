@work-management
@cli
@high
@CLI-012
Feature: Review command with done status integration
  """
  Builds on CLI-011's /review slash command foundation. Creates fspec CLI command that performs same ULTRATHINK analysis. Integrates system-reminder into update-work-unit-status.ts when transitioning to 'done'. Review command reads work unit metadata, feature files, coverage data, and performs comprehensive analysis similar to .claude/commands/review.md workflow.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Review command must be 'fspec review <work-unit-id>' CLI command, not just a slash command
  #   2. System-reminder must be emitted when transitioning to 'done' status in update-work-unit-status command
  #   3. Review command must use same analysis approach as CLI-011's /review slash command (ULTRATHINK, coverage, ACDD compliance)
  #   4. Review must work for work units in ANY status (not just done)
  #   5. System-reminder must include exact command: 'fspec review <work-unit-id>' with actual work unit ID
  #
  # EXAMPLES:
  #   1. User runs 'fspec update-work-unit-status AUTH-001 done', system-reminder suggests 'Consider reviewing before finalizing: fspec review AUTH-001'
  #   2. User runs 'fspec review CLI-011', output shows: Issues Found (critical/warnings), ACDD Compliance, Coverage Analysis, Summary with priority actions
  #   3. User runs 'fspec review SPEC-001' (work unit in testing status), review shows current progress, missing test coverage, suggests next steps
  #   4. User runs 'fspec review BUG-007' (completed bug fix), review validates fix quality, checks regression test coverage, confirms coding standards
  #
  # QUESTIONS (ANSWERED):
  #   Q: When exactly should the system-reminder to suggest review appear? (A) When transitioning TO done status, (B) When work unit IS IN done status and user runs another command, or (C) Both scenarios?
  #   A: true
  #
  #   Q: What information should 'fspec review' provide? (A) Work unit summary, (B) Linked test/impl files from coverage, (C) Git diff for this work unit, (D) Validation checklist, (E) All of the above, or (F) Something else?
  #   A: true
  #
  #   Q: Should review be (A) Informational only - just display data for manual review, (B) Interactive checklist - with items to mark complete, or (C) Automated - run validation commands and report results?
  #   A: true
  #
  #   Q: Should reviews be tracked/recorded? (A) Keep record that work unit was reviewed, (B) Require review confirmation before allowing done status, or (C) Optional review with reminder only (not mandatory)?
  #   A: true
  #
  #   Q: What about work units already in done status? Should they support 'fspec review <work-unit-id>' for retrospective review? Can review be run multiple times?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer or AI agent completing work
    I want to review completed work units before finalizing them
    So that I ensure quality and completeness before marking work as truly done

  Scenario: System-reminder suggests review when transitioning to done status
    Given I have a work unit AUTH-001 in validating status
    When I run 'fspec update-work-unit-status AUTH-001 done'
    Then the status should update to done
    And a system-reminder should be displayed suggesting 'fspec review AUTH-001'

  Scenario: Review completed work unit with comprehensive analysis
    Given I have a completed work unit CLI-011 in done status
    And the work unit has linked feature files and coverage data
    When I run 'fspec review CLI-011'
    Then the output should show Issues Found section with critical issues and warnings
    And the output should show ACDD Compliance section
    And the output should show Coverage Analysis section
    And the output should show Summary with priority actions

  Scenario: Review in-progress work unit shows current state
    Given I have a work unit SPEC-001 in testing status
    And the work unit has incomplete test coverage
    When I run 'fspec review SPEC-001'
    Then the output should show current workflow progress
    And the output should identify missing test coverage
    And the output should suggest next steps for completion
