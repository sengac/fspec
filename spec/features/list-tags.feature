@phase2
@cli
@querying
@tag-management
@medium
@unit-test
Feature: List Registered Tags from Registry
  """
  Architecture notes:
  - Reads spec/TAGS.md and displays all registered tags
  - Supports filtering by category (--category option)
  - Shows tag name, category, and description in organized format
  - Lists categories in standard order: Phase, Component, Feature Group, Technical,






  etc.
  - Handles missing TAGS.md with helpful error message

  Critical implementation requirements:
  - MUST parse TAGS.md to extract all tags
  - MUST support --category filter (e.g., --category="Phase Tags")
  - MUST display tags grouped by category
  - MUST show tag descriptions
  - MUST handle missing TAGS.md gracefully

  References:
  - TAGS.md structure: spec/TAGS.md
  - Tag categories: Phase, Component, Feature Group, Technical, Platform,
  Priority,
  Status, Testing, CAGE
  """

  Background: User Story
    As a developer working with fspec tag system
    I want to see all registered tags
    So that I know which tags are available for use in feature files



  Scenario: List all registered tags
    Given I have a TAGS.md file with tags in multiple categories
    When I run `fspec list-tags`
    Then the command should exit with code 0
    And the output should display tags grouped by category
    And the output should show "Phase Tags:", "Component Tags:", "Feature Group Tags:"
    And each tag should be displayed with its description

  Scenario: Filter tags by category
    Given I have a TAGS.md file with tags in multiple categories
    When I run `fspec list-tags --category="Phase Tags"`
    Then the command should exit with code 0
    And the output should only show tags from "Phase Tags" category
    And the output should contain "@phase1", "@phase2", "@phase3"
    And the output should not contain tags from other categories

  Scenario: Handle missing TAGS.md file
    Given no TAGS.md file exists in spec/
    When I run `fspec list-tags`
    Then the command should exit with code 2
    And the output should contain "TAGS.md not found: spec/TAGS.md"
    And the output should suggest creating TAGS.md

  Scenario: Display tag count per category
    Given I have a TAGS.md file with tags:
      | Category           | Tag Count |
      | Phase Tags         | 3         |
      | Component Tags     | 7         |
      | Feature Group Tags | 7         |
      | Technical Tags     | 8         |
    When I run `fspec list-tags`
    Then the output should show the count for each category
    And the output should contain "(3 tags)" for Phase Tags
    And the output should contain "(7 tags)" for Component Tags

  Scenario: List tags in alphabetical order within category
    Given I have Phase Tags: "@phase3", "@phase1", "@phase2"
    When I run `fspec list-tags --category="Phase Tags"`
    Then the tags should be displayed in alphabetical order:
      | @phase1 |
      | @phase2 |
      | @phase3 |

  Scenario: Handle invalid category name
    Given I have a TAGS.md file
    When I run `fspec list-tags --category="Invalid Category"`
    Then the command should exit with code 1
    And the output should contain "Category not found: Invalid Category"
    And the output should list available categories

  Scenario: Show all categories even if empty
    Given I have a TAGS.md file with only Phase Tags populated
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
    Given I have tags registered in TAGS.md
    When I run `fspec list-tags` and see tag "@custom-tag"
    And I use "@custom-tag" in a feature file
    And I run `fspec validate-tags`
    Then validation should pass for "@custom-tag"
    And the tag ecosystem remains consistent
