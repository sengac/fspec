@foundation
@discovery-workflow
@cli
@draft-driven
Feature: Draft File Update Priority
  """
  Architecture notes:
  - update-foundation command checks for foundation.json.draft first
  - If draft exists, updates draft and skips validation/FOUNDATION.md regeneration
  - If draft doesn't exist, updates final foundation.json and regenerates FOUNDATION.md
  - This ensures the draft-driven discovery workflow works correctly
  - Bug Fix: BUG-010 - update-foundation was creating foundation.json before draft completion
  """

  Background: User Story
    As an AI agent using the draft-driven discovery workflow
    I want update-foundation to update the draft file when it exists
    So that I don't create the final foundation.json before discovery is complete

  Scenario: Update draft when foundation.json.draft exists
    Given I have a file "spec/foundation.json.draft" with placeholder content
    And the file "spec/foundation.json" does not exist
    When I run `fspec update-foundation projectName "My Project"`
    Then the command should exit with code 0
    And the file "spec/foundation.json.draft" should be updated
    And the file "spec/foundation.json" should not be created
    And the output should contain "foundation.json.draft"

  Scenario: Update final foundation when draft does not exist
    Given I have a file "spec/foundation.json" with valid content
    And the file "spec/foundation.json.draft" does not exist
    When I run `fspec update-foundation projectName "Updated Project"`
    Then the command should exit with code 0
    And the file "spec/foundation.json" should be updated
    And the file "spec/FOUNDATION.md" should be regenerated
    And the output should contain "foundation.json"
    And the output should contain "FOUNDATION.md"

  Scenario: Create final foundation when neither draft nor final exist
    Given I have no foundation files
    When I run `fspec update-foundation projectName "New Project"`
    Then the command should exit with code 0
    And the file "spec/foundation.json" should be created
    And the file "spec/FOUNDATION.md" should be created
    And the file "spec/foundation.json.draft" should not exist
