@COV-037
@phase4
@cli
@querying
@documentation
@medium
@unit-test
Feature: Show Acceptance Criteria by Tag
  """
  Architecture notes:
  - Reads feature files matching tags and extracts complete acceptance criteria
  - Shows feature name, background (user story), and all scenarios with steps
  - Supports multiple output formats: text (colored), markdown, JSON
  - Useful for documentation, reviews, and extracting requirements
  - Foundation for exporting acceptance criteria to external tools

  Critical implementation requirements:
  - MUST parse feature files and extract complete structure
  - MUST show feature name, background, scenarios, and steps
  - MUST filter by tags (same logic as get-scenarios)
  - MUST support multiple output formats (text, markdown, json)
  - MUST preserve Gherkin structure in output
  - MUST handle features with no scenarios gracefully
  - Exit code 0 for success, 1 for errors

  References:
  - Gherkin parser: @cucumber/gherkin
  - Output should be suitable for documentation generation
  """

  Background: User Story
    As a developer or product manager
    I want to view all acceptance criteria for features matching tags
    So that I can review requirements, generate documentation, or export to
    other tools

  Scenario: Show acceptance criteria for single tag
    Given I have feature files tagged @phase1
    And each feature has background and scenarios
    When I run `fspec show-acceptance-criteria --tag=@phase1`
    Then the command should exit with code 0
    And the output should show all feature names
    And the output should show background user stories
    And the output should show all scenarios with their steps

  Scenario: Show acceptance criteria with multiple tags
    Given I have features tagged @phase1 @critical
    And I have features with only @phase1
    When I run `fspec show-acceptance-criteria --tag=@phase1 --tag=@critical`
    Then the output should only show features with both tags
    And each feature should show complete acceptance criteria

  Scenario: Format output as markdown
    Given I have a feature file "login.feature" tagged @auth
    When I run `fspec show-acceptance-criteria --tag=@auth --format=markdown`
    Then the output should be valid markdown
    And the output should include feature as H1 heading
    And the output should include scenarios as H2 headings
    And the output should include steps as bullet points
    And the background should be in blockquote format

  Scenario: Format output as JSON
    Given I have feature files tagged @critical
    When I run `fspec show-acceptance-criteria --tag=@critical --format=json`
    Then the output should be valid JSON
    And each feature should have name, background, and scenarios properties
    And each scenario should have name and steps properties
    And the JSON should be parseable by other tools

  Scenario: Show acceptance criteria when no features match
    Given I have feature files without @deprecated tag
    When I run `fspec show-acceptance-criteria --tag=@deprecated`
    Then the command should exit with code 0
    And the output should show "No features found matching tags: @deprecated"

  Scenario: Include feature-level tags in output
    Given I have a feature file with tags @phase1 @critical @auth
    When I run `fspec show-acceptance-criteria --tag=@phase1 --format=text`
    Then the output should show the feature tags
    And tags should be displayed at the top of each feature

  Scenario: Handle features with no background
    Given I have a feature file without a Background section
    When I run `fspec show-acceptance-criteria --tag=@test`
    Then the output should show the feature
    And the background section should be omitted
    And scenarios should still be displayed

  Scenario: Handle features with no scenarios
    Given I have a feature file with only a Feature line
    When I run `fspec show-acceptance-criteria --tag=@empty`
    Then the output should show the feature name
    And the output should indicate "No scenarios defined"

  Scenario: Show steps with proper indentation (text format)
    Given I have a scenario with Given, When, Then, And, But steps
    When I run `fspec show-acceptance-criteria --format=text`
    Then Given/When/Then steps should be displayed prominently
    And And/But steps should be indented consistently
    And the output should be readable and well-formatted

  Scenario: Export to file with --output option
    Given I have features tagged @phase1
    When I run `fspec show-acceptance-criteria --tag=@phase1 --format=markdown --output=phase1-acs.md`
    Then a file "phase1-acs.md" should be created
    And the file should contain all acceptance criteria in markdown format
    And the command output should show "Acceptance criteria written to phase1-acs.md"

  Scenario: Show architecture notes from doc strings
    Given I have a feature with architecture notes in triple-quoted doc string
    When I run `fspec show-acceptance-criteria --format=text`
    Then the architecture notes should be displayed
    And they should appear after the feature name
    And they should be visually distinct from scenarios

  Scenario: Count total scenarios shown
    Given I have features tagged @critical with 15 total scenarios
    When I run `fspec show-acceptance-criteria --tag=@critical`
    Then the output should show "Showing acceptance criteria for 15 scenarios"
    And the count should be displayed at the beginning
