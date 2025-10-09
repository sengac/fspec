@phase5
@cli
@tag-management
@modification
@medium
@unit-test
Feature: Delete Tag from Registry
  """
  Architecture notes:
  - Removes tag entries from spec/TAGS.md registry
  - Validates tag is not in use before deletion
  - Updates TAGS.md while preserving structure
  - Prevents deletion of tags currently used in feature files
  - Supports force delete with warning

  Critical implementation requirements:
  - MUST verify tag exists in TAGS.md before deletion
  - MUST check if tag is used in any feature files
  - MUST prevent deletion of used tags (unless --force)
  - MUST preserve markdown formatting and structure
  - MUST remove complete tag entry (header + description)
  - MUST provide clear error messages
  - Exit code 0 for success, 1 for errors
  """

  Background: User Story
    As a developer managing tag taxonomy
    I want to remove obsolete tags from the registry
    So that the tag system stays clean and reflects only tags currently in use
    or planned

  Scenario: Delete unused tag
    Given I have a tag @deprecated registered in TAGS.md
    And the tag @deprecated is not used in any feature files
    When I run `fspec delete-tag @deprecated`
    Then the command should exit with code 0
    And the tag @deprecated should be removed from TAGS.md
    And the TAGS.md structure should be preserved
    And the output should show "Successfully deleted tag @deprecated"

  Scenario: Attempt to delete tag in use
    Given I have a tag @phase1 registered in TAGS.md
    And the tag @phase1 is used in 5 feature files
    When I run `fspec delete-tag @phase1`
    Then the command should exit with code 1
    And the output should show "Tag @phase1 is used in 5 feature file(s)"
    And the output should list the feature files using the tag
    And the tag should remain in TAGS.md
    And the output should suggest using --force to delete anyway

  Scenario: Force delete tag in use
    Given I have a tag @deprecated registered in TAGS.md
    And the tag @deprecated is used in 2 feature files
    When I run `fspec delete-tag @deprecated --force`
    Then the command should exit with code 0
    And the tag @deprecated should be removed from TAGS.md
    And the output should show warning about files still using the tag
    And the output should list the 2 feature files

  Scenario: Attempt to delete non-existent tag
    Given I do not have a tag @nonexistent in TAGS.md
    When I run `fspec delete-tag @nonexistent`
    Then the command should exit with code 1
    And the output should show "Tag @nonexistent not found in registry"
    And TAGS.md should remain unchanged

  Scenario: Delete tag preserves other tags in same category
    Given I have tags @phase1, @phase2, @phase3 in category "Tag Categories"
    When I run `fspec delete-tag @phase2`
    Then the command should exit with code 0
    And @phase1 and @phase3 should remain in TAGS.md
    And the category "Tag Categories" should remain intact

  Scenario: Delete tag from specific category
    Given I have a tag @custom registered in category "Technical Tags"
    And the tag is not used in any feature files
    When I run `fspec delete-tag @custom`
    Then the command should exit with code 0
    And the tag should be removed from "Technical Tags" category
    And other tags in "Technical Tags" should remain

  Scenario: Delete tag updates tag statistics
    Given I have 10 tags registered in TAGS.md
    And one tag @obsolete is unused
    When I run `fspec delete-tag @obsolete`
    Then the command should exit with code 0
    And the tag count should be 9
    And the tag should not appear in tag statistics

  Scenario: Handle TAGS.md with invalid format
    Given I have a TAGS.md file with invalid structure
    When I run `fspec delete-tag @sometag`
    Then the command should exit with code 1
    And the output should show "Could not parse TAGS.md structure"
    And the file should remain unchanged

  Scenario: Delete last tag in category leaves category intact
    Given I have only one tag @lonely in category "Custom Category"
    And the tag is not used in any feature files
    When I run `fspec delete-tag @lonely`
    Then the command should exit with code 0
    And the tag should be removed
    And the category "Custom Category" should remain (empty)

  Scenario: Dry run shows what would be deleted
    Given I have a tag @test registered in TAGS.md
    And the tag is not used in any feature files
    When I run `fspec delete-tag @test --dry-run`
    Then the command should exit with code 0
    And the output should show "Would delete tag @test"
    And the tag should remain in TAGS.md
    And the output should show the category it would be removed from
