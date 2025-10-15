@system-reminder
@coverage-tracking
@cli
@phase1
Feature: Add system-reminder to generate-coverage command

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Coverage files must be populated manually using link-coverage
  #   2. System reminder must be displayed after generate-coverage command
  #   3. Reminder must explain that coverage files are empty until linked
  #   4. Reminder must show example link-coverage commands
  #   5. Yes, reminder should be shown for both modes with contextual examples
  #   6. Yes, should mention show-coverage as verification step
  #   7. Yes, reminder should clearly explain generate-coverage creates empty files and link-coverage populates them
  #   8. Yes, should mention show-coverage as verification step
  #   9. Yes, reminder should clearly explain generate-coverage creates empty files and link-coverage populates them
  #   10. Yes, reminder should clearly explain generate-coverage creates empty files and link-coverage populates them
  #   11. Yes, reminder should clearly explain generate-coverage creates empty files and link-coverage populates them
  #
  # EXAMPLES:
  #   1. User runs generate-coverage with no arguments and sees reminder
  #   2. User runs generate-coverage --dry-run and sees reminder
  #   3. User runs generate-coverage feature-name and sees reminder with feature-specific examples
  #   4. Reminder includes link-coverage command syntax with test file example
  #   5. Reminder includes link-coverage command syntax with implementation file example
  #   6. Reminder explains three-step ACDD workflow: write specs → link tests → link implementation
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the reminder be shown for both single-feature and all-features modes?
  #   A: true
  #
  #   Q: Should the reminder include show-coverage command as next step?
  #   A: true
  #
  #   Q: Should the reminder be suppressible with a --quiet flag?
  #   A: true
  #
  #   Q: Should the reminder explain the difference between generate-coverage and link-coverage?
  #   A: true
  #
  # ASSUMPTIONS:
  #   1. No, reminder should always be shown as it's critical for ACDD workflow
  #   2. No, reminder should always be shown as it's critical for ACDD workflow
  #   3. No, reminder should always be shown as it's critical for ACDD workflow
  #
  # ========================================
  Background: User Story
    As a developer using fspec with ACDD workflow
    I want to be reminded to manually link coverage after running generate-coverage
    So that I don't forget the critical step of linking tests and implementation to scenarios

  Scenario: Display reminder after generate-coverage with no arguments
    Given I have feature files in spec/features/ directory
    When I run `fspec generate-coverage`
    Then the command should succeed
    And the output should display a system-reminder
    And the reminder should explain that coverage files are created empty
    And the reminder should mention that link-coverage must be used to populate them
    And the reminder should include example link-coverage command for linking tests
    And the reminder should include example link-coverage command for linking implementation
    And the reminder should mention show-coverage as a verification step

  Scenario: Display reminder after generate-coverage --dry-run
    Given I have feature files in spec/features/ directory
    When I run `fspec generate-coverage --dry-run`
    Then the command should succeed
    And the output should display a system-reminder
    And the reminder should explain the three-step ACDD workflow
    And the reminder should reference link-coverage command

  Scenario: Reminder explains difference between generate-coverage and link-coverage
    Given I have feature files in spec/features/ directory
    When I run `fspec generate-coverage`
    Then the system-reminder should clearly state:
      """
      generate-coverage creates EMPTY coverage files
      link-coverage POPULATES coverage files with mappings
      """
    And the reminder should emphasize that generation and linking are separate steps

  Scenario: Reminder shows complete ACDD workflow with coverage commands
    Given I have feature files in spec/features/ directory
    When I run `fspec generate-coverage`
    Then the system-reminder should include:
      """
      ACDD Coverage Workflow:
      1. Write specifications (feature files)
      2. Generate coverage files: fspec generate-coverage
      3. Write tests: Write failing tests for scenarios
      4. Link tests: fspec link-coverage <feature> --scenario "<name>" --test-file <path> --test-lines <range>
      5. Implement code: Write minimal code to pass tests
      6. Link implementation: fspec link-coverage <feature> --scenario "<name>" --test-file <path> --impl-file <path> --impl-lines <lines>
      7. Verify coverage: fspec show-coverage <feature>
      """
