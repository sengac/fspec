@done
@feature-management
@generator
@high
@FEAT-017
Feature: Duplicate scenario detection in generate-scenarios command
  """
  Command accepts --ignore-possible-duplicates flag to bypass duplicate checking when user determines matches are false positives.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Existing code already exists to search for duplicate scenarios and provide results to LLM
  #   2. System-reminder must provide CLEAR instructions on what to do next
  #   3. Must show which existing feature files to investigate when duplicates are detected
  #   4. Must provide instructions on how to ignore warning if it's not valid
  #   5. Command must accept --ignore-possible-duplicates flag to bypass duplicate check
  #
  # EXAMPLES:
  #   1. User runs 'fspec generate-scenarios WORK-001', duplicate scenarios are found above threshold, system-reminder shows list of feature files to investigate with clear next steps
  #   2. User reviews system-reminder, investigates suggested feature files, determines it's a false positive, runs 'fspec generate-scenarios WORK-001 --ignore-possible-duplicates' to proceed
  #   3. User runs 'fspec generate-scenarios WORK-001', no duplicates found, scenarios are generated normally without any warnings
  #
  # ========================================
  Background: User Story
    As a developer using fspec
    I want to detect duplicate scenarios before generating new ones
    So that I avoid creating duplicate scenarios across feature files

  Scenario: Detect duplicate scenarios above threshold and display system-reminder
    Given I have a work unit "WORK-001" with example mapping data
    And existing feature files contain similar scenarios above the similarity threshold
    When I run "fspec generate-scenarios WORK-001"
    Then the command should detect duplicate scenarios
    And a system-reminder should be displayed
    And the system-reminder should list feature files to investigate
    And the system-reminder should include clear next steps
    And the system-reminder should provide instructions to use --ignore-possible-duplicates flag

  Scenario: Bypass duplicate check with --ignore-possible-duplicates flag
    Given I have reviewed the duplicate scenarios warning
    And I have investigated the suggested feature files
    And I have determined the matches are false positives
    When I run "fspec generate-scenarios WORK-001 --ignore-possible-duplicates"
    Then the duplicate check should be bypassed
    And scenarios should be generated normally
    And no duplicate warning should be displayed

  Scenario: Generate scenarios normally when no duplicates found
    Given I have a work unit "WORK-001" with example mapping data
    And no existing feature files contain similar scenarios
    When I run "fspec generate-scenarios WORK-001"
    Then the duplicate check should complete
    And scenarios should be generated without warnings
    And no system-reminder should be displayed
