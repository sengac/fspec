@phase5
@cli
@bulk-operations
@modification
@high
@unit-test
Feature: Bulk Rename Tags Across Files
  """
  Architecture notes:
  - Renames tags across all feature files in bulk
  - Updates both feature-level and scenario-level tags
  - Validates tag existence before renaming
  - Preserves file structure and formatting
  - Optionally updates TAGS.md registry

  Critical implementation requirements:
  - MUST require both --from and --to parameters
  - MUST validate old tag exists in at least one file
  - MUST validate new tag format (@lowercase-with-hyphens)
  - MUST update all occurrences across all feature files
  - MUST support dry-run preview
  - MUST preserve tag positions (feature-level vs scenario-level)
  - MUST handle scenario-level tags separately
  - MUST validate Gherkin syntax after changes
  - MUST provide count of files and occurrences changed
  - Exit code 0 for success, 1 for errors
  """

  Background: User Story
    As a developer managing tag taxonomy
    I want to bulk rename tags across all feature files
    So that I can refactor tag naming without manual file editing

  Scenario: Rename tag across multiple feature files
    Given I have 5 feature files
    And 3 files use the tag @phase1
    When I run `fspec retag --from=@phase1 --to=@phase-one`
    Then the command should exit with code 0
    And all @phase1 tags should be changed to @phase-one
    And the 3 files should be updated
    And the output should show "Renamed @phase1 to @phase-one in 3 file(s) (5 occurrence(s))"

  Scenario: Rename feature-level tag
    Given I have a feature file with tag @deprecated at feature level
    When I run `fspec retag --from=@deprecated --to=@legacy`
    Then the command should exit with code 0
    And the feature-level tag should change from @deprecated to @legacy
    And the file structure should be preserved

  Scenario: Rename scenario-level tag
    Given I have scenarios tagged with @wip
    When I run `fspec retag --from=@wip --to=@in-progress`
    Then the command should exit with code 0
    And all scenario-level @wip tags should change to @in-progress
    And feature-level tags should remain unchanged

  Scenario: Rename tag used at both feature and scenario levels
    Given I have files with @temp at feature level
    And scenarios with @temp at scenario level
    When I run `fspec retag --from=@temp --to=@temporary`
    Then the command should exit with code 0
    And all @temp tags should change to @temporary at both levels
    And the output should show total occurrences changed

  Scenario: Attempt to rename non-existent tag
    Given I have feature files with various tags
    And no files contain the tag @nonexistent
    When I run `fspec retag --from=@nonexistent --to=@new`
    Then the command should exit with code 1
    And the output should show "Tag @nonexistent not found in any feature files"
    And no files should be modified

  Scenario: Prevent rename to invalid tag format
    Given I have files with tag @phase1
    When I run `fspec retag --from=@phase1 --to=Phase1`
    Then the command should exit with code 1
    And the output should show "Invalid tag format"
    And no files should be modified

  Scenario: Dry run preview without making changes
    Given I have 10 files using @old-tag
    When I run `fspec retag --from=@old-tag --to=@new-tag --dry-run`
    Then the command should exit with code 0
    And the output should show "Would rename @old-tag to @new-tag in 10 file(s)"
    And the output should list affected files
    And no files should be modified

  Scenario: Rename tag with special characters
    Given I have files tagged with @bug-#123
    When I run `fspec retag --from=@bug-#123 --to=@issue-123`
    Then the command should exit with code 0
    And all @bug-#123 tags should change to @issue-123
    And files should remain valid Gherkin

  Scenario: Rename tag preserves other tags
    Given I have a feature with tags @phase1 @critical @api
    When I run `fspec retag --from=@phase1 --to=@v1`
    Then the command should exit with code 0
    And the feature should have tags @v1 @critical @api
    And tag order should be preserved
    And other tags should remain unchanged

  Scenario: Rename tag validates Gherkin after changes
    Given I have 20 files using @deprecated
    When I run `fspec retag --from=@deprecated --to=@legacy`
    Then the command should exit with code 0
    And all 20 files should have valid Gherkin syntax
    And the output should show "All modified files validated successfully"

  Scenario: Rename tag with multiple occurrences in single file
    Given I have a feature with @temp at feature level
    And 3 scenarios with @temp at scenario level
    When I run `fspec retag --from=@temp --to=@temporary`
    Then the command should exit with code 0
    And all 4 occurrences should be renamed
    And the output should show "4 occurrence(s)"

  Scenario: Prevent rename without required parameters
    Given I have feature files with various tags
    When I run `fspec retag --from=@old`
    Then the command should exit with code 1
    And the output should show "Both --from and --to are required"
    And no files should be modified
