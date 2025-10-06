@phase5
@cli
@bulk-operations
@modification
@high
@unit-test
Feature: Bulk Delete Scenarios by Tag
  """
  Architecture notes:
  - Deletes multiple scenarios across feature files by tag filter
  - Uses get-scenarios foundation for tag-based queries
  - Supports AND logic for multiple tags
  - Preserves feature file structure and non-matching scenarios
  - Validates Gherkin syntax after each file modification
  - Provides dry-run preview and confirmation counts

  Critical implementation requirements:
  - MUST support single tag filter (--tag=@phase1)
  - MUST support multiple tag filters with AND logic (--tag=@phase1 --tag=@critical)






  - MUST preserve feature file structure after scenario deletion
  - MUST validate Gherkin syntax after modifications
  - MUST provide count of scenarios to be deleted before proceeding
  - MUST support --dry-run flag for preview without changes
  - MUST skip features with no matching scenarios
  - MUST handle features where all scenarios are deleted
  - Exit code 0 for success, 1 for errors
  """

  Background: User Story
    As a developer managing feature specifications
    I want to bulk delete scenarios by tag across multiple files
    So that I can efficiently remove obsolete or deprecated scenarios without
    manual file editing



  Scenario: Delete scenarios by single tag from one feature file
    Given I have a feature file with 5 scenarios
    And 2 scenarios are tagged with @deprecated
    When I run `fspec delete-scenarios --tag=@deprecated`
    Then the command should exit with code 0
    And the 2 @deprecated scenarios should be removed
    And the 3 non-tagged scenarios should remain
    And the feature file structure should be preserved
    And the output should show "Deleted 2 scenario(s) from 1 file(s)"

  Scenario: Delete scenarios by single tag across multiple files
    Given I have 3 feature files with scenarios
    And 5 scenarios across files are tagged with @obsolete
    When I run `fspec delete-scenarios --tag=@obsolete`
    Then the command should exit with code 0
    And all 5 @obsolete scenarios should be removed
    And the output should show "Deleted 5 scenario(s) from 3 file(s)"
    And all feature files should remain valid Gherkin

  Scenario: Delete scenarios by multiple tags with AND logic
    Given I have scenarios tagged with various combinations
    And 2 scenarios have both @phase1 and @deprecated tags
    And 3 scenarios have only @phase1 tag
    And 1 scenario has only @deprecated tag
    When I run `fspec delete-scenarios --tag=@phase1 --tag=@deprecated`
    Then the command should exit with code 0
    And only the 2 scenarios with both tags should be removed
    And the 4 scenarios without both tags should remain
    And the output should show "Deleted 2 scenario(s)"

  Scenario: Dry run preview without making changes
    Given I have 10 scenarios tagged with @test
    When I run `fspec delete-scenarios --tag=@test --dry-run`
    Then the command should exit with code 0
    And the output should show "Would delete 10 scenario(s) from X file(s)"
    And the output should list the scenarios that would be deleted
    And no files should be modified
    And all 10 scenarios should remain in files

  Scenario: Skip feature files with no matching scenarios
    Given I have 5 feature files
    And only 2 files contain scenarios tagged @old
    When I run `fspec delete-scenarios --tag=@old`
    Then the command should exit with code 0
    And only the 2 files with @old scenarios should be modified
    And the 3 files without @old scenarios should remain unchanged
    And the output should show files modified count

  Scenario: Handle feature with all scenarios deleted
    Given I have a feature file with 3 scenarios
    And all 3 scenarios are tagged with @remove
    When I run `fspec delete-scenarios --tag=@remove`
    Then the command should exit with code 0
    And all 3 scenarios should be removed
    And the feature file should contain only the Feature header
    And the feature file should remain valid Gherkin
    And the output should show "Deleted 3 scenario(s) from 1 file(s)"

  Scenario: Delete scenarios preserves feature tags
    Given I have a feature file tagged with @feature-tag
    And it has 2 scenarios tagged @scenario-tag
    When I run `fspec delete-scenarios --tag=@scenario-tag`
    Then the command should exit with code 0
    And the @feature-tag should remain on the feature
    And the 2 scenarios should be removed
    And the feature structure should be preserved

  Scenario: Delete scenarios preserves Background section
    Given I have a feature with a Background section
    And 2 scenarios tagged @cleanup
    When I run `fspec delete-scenarios --tag=@cleanup`
    Then the command should exit with code 0
    And the Background section should remain intact
    And only the 2 @cleanup scenarios should be removed
    And the feature should remain valid Gherkin

  Scenario: Attempt to delete with no matching scenarios
    Given I have feature files with various scenarios
    And no scenarios are tagged with @nonexistent
    When I run `fspec delete-scenarios --tag=@nonexistent`
    Then the command should exit with code 0
    And the output should show "No scenarios found matching tags"
    And no files should be modified

  Scenario: Delete scenarios with special characters in tags
    Given I have scenarios tagged with @bug-#123
    When I run `fspec delete-scenarios --tag=@bug-#123`
    Then the command should exit with code 0
    And scenarios with @bug-#123 should be removed
    And the feature files should remain valid Gherkin

  Scenario: Validate Gherkin syntax after bulk deletion
    Given I have 20 scenarios tagged @temp across 5 files
    When I run `fspec delete-scenarios --tag=@temp`
    Then the command should exit with code 0
    And all 5 modified files should have valid Gherkin syntax
    And the Gherkin parser should successfully parse all files
    And the output should show "All modified files validated successfully"

  Scenario: Delete scenarios updates file line counts
    Given I have a feature file with 100 lines
    And 10 scenarios in the file are tagged @remove
    And the scenarios total 50 lines
    When I run `fspec delete-scenarios --tag=@remove`
    Then the command should exit with code 0
    And the file should be approximately 50 lines
    And the file structure should remain valid
