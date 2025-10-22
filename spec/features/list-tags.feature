@cli
@querying
@tag-management
@medium
@unit-test
Feature: List Registered Tags from Registry
  """
  Architecture notes:
  - Reads spec/tags.json (source of truth) and displays all registered tags
  - Supports filtering by category (--category option)
  - Shows tag name, category, and description in organized format
  - Lists categories in standard order from JSON structure
  - Handles missing tags.json with helpful error message

  Critical implementation requirements:
  - MUST load spec/tags.json
  - MUST parse categories array from JSON
  - MUST support --category filter (e.g., --category="Component Tags")
  - MUST display tags grouped by category
  - MUST show tag descriptions
  - MUST handle missing tags.json gracefully
  - Exit code 0 for success, 2 for missing file, 1 for errors

  Workflow:
  1. Load spec/tags.json
  2. Parse categories array
  3. Filter by category if --category specified
  4. Sort tags alphabetically within each category
  5. Display formatted output with counts
  6. Handle errors with clear messages

  References:
  - tags.schema.json: src/schemas/tags.schema.json
  - Tags type: src/types/tags.ts
  """

  Background: User Story
    As a developer working with fspec tag system
    I want to see all registered tags
    So that I know which tags are available for use in feature files

  Scenario: List all registered tags
    Given I have a tags.json file with tags in multiple categories
    When I run `fspec list-tags`
    Then the command should exit with code 0
    And the output should display tags grouped by category
    And the output should show "Component Tags:", "Feature Group Tags:"
    And each tag should be displayed with its description

  Scenario: Filter tags by category
    Given I have a tags.json file with tags in multiple categories
    When I run `fspec list-tags --category="Component Tags"`
    Then the command should exit with code 0
    And the output should only show tags from "Component Tags" category
    And the output should contain "@cli", "@parser", "@generator"
    And the output should not contain tags from other categories

  Scenario: Handle missing tags.json file
    Given no tags.json file exists in spec/
    When I run `fspec list-tags`
    Then the command should exit with code 2
    And the output should contain "tags.json not found"
    And the output should suggest creating tags.json or using fspec register-tag

  Scenario: Display tag count per category
    Given I have a tags.json file with tags:
      | Category           | Tag Count |
      | Component Tags     | 7         |
      | Feature Group Tags | 7         |
      | Technical Tags     | 8         |
    When I run `fspec list-tags`
    Then the output should show the count for each category
    And the output should contain "(7 tags)" for Component Tags
    And the output should contain "(7 tags)" for Feature Group Tags

  Scenario: List tags in alphabetical order within category
    Given I have Component Tags in tags.json: "@parser", "@cli", "@generator"
    When I run `fspec list-tags --category="Component Tags"`
    Then the tags should be displayed in alphabetical order:
      | @cli       |
      | @generator |
      | @parser    |

  Scenario: Handle invalid category name
    Given I have a tags.json file
    When I run `fspec list-tags --category="Invalid Category"`
    Then the command should exit with code 1
    And the output should contain "Category not found: Invalid Category"
    And the output should list available categories

  Scenario: Show all categories even if empty
    Given I have a tags.json file with only Component Tags populated
    When I run `fspec list-tags`
    Then the output should show all category headers
    And empty categories should show "(0 tags)"

  Scenario: Display tag descriptions with wrapping
    Given I have a tag with a long description:
      | Tag        | Description                                                                |
      | @long-desc | This is a very long description that explains the purpose and usage of tag |
    When I run `fspec list-tags`
    Then the description should be displayed without truncation
    And the output should be properly formatted

  Scenario: AI agent workflow - discover available tags
    Given I am an AI agent working on a new feature specification
    And I need to know which tags to use
    When I run `fspec list-tags --category="Feature Group Tags"`
    Then I should see all available feature group tags
    And I can select appropriate tags for my feature
    And when I run `fspec list-tags`
    Then I can see the complete tag vocabulary organized by category

  Scenario: Compare with validate-tags integration
    Given I have tags registered in tags.json
    When I run `fspec list-tags` and see tag "@custom-tag"
    And I use "@custom-tag" in a feature file
    And I run `fspec validate-tags`
    Then validation should pass for "@custom-tag"
    And the tag ecosystem remains consistent

  Scenario: JSON-backed workflow - read from source of truth
    Given I have a valid tags.json file with multiple categories
    When I run `fspec list-tags`
    Then the command should load tags from spec/tags.json
    And tags should be displayed grouped by category
    And each category should show its tag count
    And tags should be sorted alphabetically within categories
    And the command should exit with code 0
