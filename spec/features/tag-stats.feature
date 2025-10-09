@phase2
@cli
@querying
@tag-management
@medium
@unit-test
Feature: Show Tag Usage Statistics
  """
  Architecture notes:
  - Reads all feature files and extracts tag usage
  - Counts tag occurrences across all feature files
  - Groups statistics by tag categories (from tags.json)
  - Displays total features, total tags, and per-tag counts
  - Helps identify unused tags and most common tags
  - Useful for maintaining tag registry and identifying orphaned tags

  Critical implementation requirements:
  - MUST parse all feature files in spec/features/
  - MUST count each tag occurrence (features can have multiple tags)
  - MUST load tags.json for category grouping
  - MUST group statistics by category from tags.json
  - MUST show total count and per-tag breakdown
  - MUST identify registered tags that are never used
  - Output MUST be sorted by count (descending) within each category
  - Exit code 0 for success, 2 for errors

  Workflow:
  1. Load spec/tags.json (optional, continue if missing)
  2. Glob all spec/features/**/*.feature files
  3. Parse each feature file with @cucumber/gherkin
  4. Extract tags from each feature
  5. Count tag occurrences
  6. Group by categories from tags.json
  7. Identify unused registered tags
  8. Display formatted statistics

  References:
  - Tag registry: spec/tags.json
  - Tags type: src/types/tags.ts
  - Gherkin parser: @cucumber/gherkin
  """

  Background: User Story
    As a developer managing feature specifications
    I want to see tag usage statistics across all features
    So that I can maintain a clean tag registry and identify unused tags

  Scenario: Show overall tag statistics
    Given I have 5 feature files with various tags
    When I run `fspec tag-stats`
    Then the command should exit with code 0
    And the output should show total number of feature files
    And the output should show total number of unique tags used
    And the output should show total tag occurrences

  Scenario: Show per-category tag statistics
    Given I have feature files using tags from multiple categories
    When I run `fspec tag-stats`
    Then the output should group statistics by category
    And each category should show tag name and count
    And tags should be sorted by count in descending order within each category

  Scenario: Show most used tags
    Given I have feature files where @phase1 is used 4 times and @phase2 is used 2 times
    When I run `fspec tag-stats`
    Then @phase1 should appear before @phase2 in the output
    And the count for @phase1 should be 4
    And the count for @phase2 should be 2

  Scenario: Identify unused registered tags
    Given I have tags.json with 10 registered tags
    And only 7 of those tags are used in feature files
    When I run `fspec tag-stats`
    Then the output should show a section for unused tags
    And the unused tags section should list 3 tags
    And the unused tags should be the ones not present in any feature file

  Scenario: Show statistics when no feature files exist
    Given I have no feature files in spec/features/
    When I run `fspec tag-stats`
    Then the command should exit with code 0
    And the output should show 0 feature files
    And the output should show 0 tags used

  Scenario: Handle feature files with no tags
    Given I have a feature file with no tags
    And I have a feature file with 3 tags
    When I run `fspec tag-stats`
    Then the command should exit with code 0
    And the statistics should only count the tags from the tagged file

  Scenario: Count tags from unregistered tags correctly
    Given I have feature files using both registered and unregistered tags
    When I run `fspec tag-stats`
    Then the output should show all tags found in feature files
    And unregistered tags should be grouped in an "Unregistered" section
    And the count for each unregistered tag should be accurate

  Scenario: Handle tags.json not found
    Given spec/tags.json does not exist
    And I have feature files with tags
    When I run `fspec tag-stats`
    Then the command should exit with code 0
    And the output should show warning that tags.json was not found
    And all tags should be shown in "Unregistered" category
    And the statistics should still be accurate

  Scenario: Display zero count for categories with no usage
    Given I have tags.json with "Testing Tags" category
    And no feature files use any testing tags
    When I run `fspec tag-stats`
    Then the output should show "Testing Tags" category
    And all testing tags should show count of 0 in the unused section

  Scenario: Handle invalid feature files gracefully
    Given I have 3 valid feature files with tags
    And I have 1 feature file with invalid Gherkin syntax
    When I run `fspec tag-stats`
    Then the command should exit with code 0
    And the statistics should count tags from the 3 valid files
    And the output should show a warning about the invalid file

  Scenario: JSON-backed workflow - read categories from tags.json
    Given I have tags.json with multiple tag categories
    And I have feature files using tags from different categories
    When I run `fspec tag-stats`
    Then the command should load tag categories from spec/tags.json
    And statistics should be grouped by categories from tags.json
    And each category should show accurate tag counts
    And unused registered tags should be identified correctly
    And the command should exit with code 0
