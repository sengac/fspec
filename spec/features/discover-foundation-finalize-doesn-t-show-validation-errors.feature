@done
@validation
@foundation
@cli
@phase1
@BUG-016
Feature: discover-foundation --finalize doesn't show validation errors

  """
  Architecture notes:
  - Bug in CLI action handler for discover-foundation command
  - Lines 280-288 in src/commands/discover-foundation.ts only print generic error
  - The validationErrors field (lines 132-139) contains detailed messages but is never displayed
  - Fix: Add console.log(result.validationErrors) before exit when validation fails
  - Error message includes missing fields and commands to fix (add-persona, add-capability, update-foundation)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When finalize fails validation, MUST show detailed error messages
  #   2. Error messages MUST list missing/invalid fields
  #   3. Error messages MUST show commands to fix each type of issue
  #   4. Error messages MUST be visible to both users and AI agents
  #
  # EXAMPLES:
  #   1. User runs 'fspec discover-foundation --finalize' with empty personas array, sees error: 'Missing required: personas[0].name' with command to fix
  #   2. User runs finalize with placeholder persona data, sees error listing ALL missing fields in personas
  #   3. User runs finalize with empty capabilities array, sees error: 'Missing required: solutionSpace.capabilities' with example command
  #
  # ========================================

  Background: User Story
    As a AI agent using draft-driven discovery workflow
    I want to see detailed validation errors when finalize fails
    So that I know exactly what fields to fix without guessing

  Scenario: Show detailed error when personas array has placeholder data
    Given I have a foundation.json.draft with all basic fields filled
    And the personas array contains placeholder "[QUESTION: Who uses this?]" text
    When I run `fspec discover-foundation --finalize`
    Then the command should exit with code 1
    And the output should contain "Foundation validation failed"
    And the output should contain "Missing required: personas"
    And the output should show command to fix: "fspec add-persona"

  Scenario: Show detailed error when capabilities array is empty
    Given I have a foundation.json.draft with all basic fields filled
    And the solutionSpace.capabilities array is empty
    When I run `fspec discover-foundation --finalize`
    Then the command should exit with code 1
    And the output should contain "Foundation validation failed"
    And the output should contain "Missing required"
    And the output should show command to fix: "fspec add-capability"

  Scenario: Show all missing fields in one error message
    Given I have a foundation.json.draft with multiple placeholder fields
    And the personas array contains placeholder text
    And the capabilities array is empty
    When I run `fspec discover-foundation --finalize`
    Then the command should exit with code 1
    And the output should contain "Foundation validation failed"
    And the output should list all missing fields
    And the output should show all relevant fix commands
