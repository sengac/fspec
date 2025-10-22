@querying
@bug-fix
@cli
@BUG-012
Feature: Fix show-epic command returning undefined
  """
  Architecture notes:
  - Uses loadEpics() from src/utils/epic-utils.ts to load epics.json
  - Epic lookup by ID (lowercase-with-hyphens format)
  - Must handle missing epics.json file gracefully
  - Exit code 1 on error, 0 on success
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Command must validate epic ID exists before attempting to display
  #   2. Must show clear error message with epic ID when not found
  #   3. Must exit with non-zero code on error
  #
  # EXAMPLES:
  #   1. Run 'fspec show-epic invalid-epic' shows error: Epic 'invalid-epic' not found
  #   2. Error message suggests 'fspec list-epics' to see available epics
  #   3. Valid epic ID works correctly: 'fspec show-epic user-management' displays epic details
  #
  # ========================================
  Background: User Story
    As a developer using fspec
    I want to see a clear error when querying non-existent epics
    So that I understand what went wrong and how to fix it

  Scenario: Show error when epic does not exist
    Given I have a project with epics configured
    And the epic "invalid-epic" does not exist
    When I run "fspec show-epic invalid-epic"
    Then the command should exit with code 1
    And the output should contain "Epic 'invalid-epic' not found"
    And the output should suggest running "fspec list-epics"

  Scenario: Show helpful suggestion in error message
    Given I have a project with epics configured
    And the epic "foundation-document-redesign" does not exist
    When I run "fspec show-epic foundation-document-redesign"
    Then the command should exit with code 1
    And the output should contain "Try: fspec list-epics"

  Scenario: Show epic details when epic exists
    Given I have a project with epics configured
    And an epic "user-management" exists with title "User Management Features"
    When I run "fspec show-epic user-management"
    Then the command should exit with code 0
    And the output should contain "Epic: user-management"
    And the output should contain "Title: User Management Features"
    And the output should not contain "undefined"
