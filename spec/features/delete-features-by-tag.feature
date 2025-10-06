@phase5
@cli
@bulk-operations
@modification
@high
@unit-test
Feature: Bulk Delete Feature Files by Tag
  """
  Architecture notes:
  - Deletes entire feature files by tag filter
  - Uses list-features foundation for tag-based queries
  - Supports AND logic for multiple tags
  - Permanently removes feature files from filesystem
  - Provides dry-run preview and confirmation counts
  - Validates tags before deletion

  Critical implementation requirements:
  - MUST support single tag filter (--tag=@deprecated)
  - MUST support multiple tag filters with AND logic (--tag=@phase1 --tag=@deprecated)





  - MUST delete entire feature files (not just scenarios)
  - MUST provide count of files to be deleted before proceeding
  - MUST support --dry-run flag for preview without changes
  - MUST handle no matching files gracefully
  - MUST prevent deletion without tag filter
  - MUST provide clear confirmation messages
  - Exit code 0 for success, 1 for errors
  """

  Background: User Story
    As a developer managing feature specifications
    I want to bulk delete entire feature files by tag
    So that I can efficiently remove obsolete feature areas without manual file
    deletion



  Scenario: Delete feature files by single tag
    Given I have 5 feature files
    And 2 files are tagged with @deprecated
    When I run `fspec delete-features --tag=@deprecated`
    Then the command should exit with code 0
    And the 2 @deprecated feature files should be deleted
    And the 3 non-tagged feature files should remain
    And the output should show "Deleted 2 feature file(s)"

  Scenario: Delete feature files by multiple tags with AND logic
    Given I have feature files with various tag combinations
    And 2 files have both @phase1 and @deprecated tags
    And 3 files have only @phase1 tag
    And 1 file has only @deprecated tag
    When I run `fspec delete-features --tag=@phase1 --tag=@deprecated`
    Then the command should exit with code 0
    And only the 2 files with both tags should be deleted
    And the 4 files without both tags should remain
    And the output should show "Deleted 2 feature file(s)"

  Scenario: Dry run preview without making changes
    Given I have 10 feature files tagged with @obsolete
    When I run `fspec delete-features --tag=@obsolete --dry-run`
    Then the command should exit with code 0
    And the output should show "Would delete 10 feature file(s)"
    And the output should list the files that would be deleted
    And no files should be deleted
    And all 10 files should remain on filesystem

  Scenario: Attempt to delete with no matching files
    Given I have feature files with various tags
    And no files are tagged with @nonexistent
    When I run `fspec delete-features --tag=@nonexistent`
    Then the command should exit with code 0
    And the output should show "No feature files found matching tags"
    And no files should be deleted

  Scenario: Delete feature files with special characters in tags
    Given I have 3 files tagged with @bug-#123
    When I run `fspec delete-features --tag=@bug-#123`
    Then the command should exit with code 0
    And the 3 files with @bug-#123 should be deleted
    And the output should show "Deleted 3 feature file(s)"

  Scenario: Prevent deletion without tag filter
    Given I have 20 feature files in spec/features/
    When I run `fspec delete-features`
    Then the command should exit with code 1
    And the output should show "At least one --tag is required"
    And no files should be deleted
    And all 20 files should remain

  Scenario: Delete all files matching single tag
    Given I have 15 feature files
    And all 15 files are tagged with @remove-all
    When I run `fspec delete-features --tag=@remove-all`
    Then the command should exit with code 0
    And all 15 files should be deleted
    And spec/features/ directory should be empty
    And the output should show "Deleted 15 feature file(s)"

  Scenario: Delete files updates directory structure
    Given I have feature files in spec/features/ directory
    And 5 files are tagged with @cleanup
    And the directory contains 12 total files
    When I run `fspec delete-features --tag=@cleanup`
    Then the command should exit with code 0
    And 5 files should be deleted
    And 7 files should remain
    And the directory should contain exactly 7 files

  Scenario: Delete files with nested directory paths
    Given I have feature files with various paths
    And files are located at spec/features/file.feature
    And 3 files are tagged with @old
    When I run `fspec delete-features --tag=@old`
    Then the command should exit with code 0
    And the 3 @old files should be deleted from spec/features/
    And the remaining files should be intact

  Scenario: Confirm deletion of multiple files
    Given I have 8 feature files tagged with @phase0
    When I run `fspec delete-features --tag=@phase0`
    Then the command should exit with code 0
    And all 8 files should be deleted
    And the output should list each deleted file
    And the output should show total count "Deleted 8 feature file(s)"
