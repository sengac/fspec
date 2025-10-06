@phase1
@cli
@querying
@feature-management
@gherkin
@cross-platform
@high
@unit-test
@integration-test
Feature: List Feature Files
  """
  Architecture notes:
  - Lists all .feature files in spec/features/ directory
  - Supports filtering by tags using --tag option
  - Parses each feature file to extract metadata (name, tags, scenario count)
  - Displays files in alphabetical order by default
  - Uses @cucumber/gherkin-parser to read feature metadata
  - Supports multiple --tag options for AND filtering

  Critical implementation requirements:
  - MUST list all .feature files in spec/features/
  - MUST show feature name and file path
  - MUST support --tag filtering (e.g., --tag=@phase1)
  - MUST handle missing spec/features/ directory gracefully
  - MUST show scenario count for each feature
  - SHOULD display tags for each feature
  - Output MUST be clear and scannable

  Output format:
  - One line per feature file
  - Format: "  [file-path] - Feature Name (N scenarios) [@tag1 @tag2]"
  - Summary line showing total count

  Integration points:
  - CLI command: `fspec list-features [--tag=@tag]`
  - Called by AI agents to discover existing specifications
  - Called by CAGE to understand specification landscape
  """

  Background: User Story
    As a developer using AI agents for spec-driven development
    I want to see all existing feature files
    So that I understand what specifications already exist and avoid duplicates



  Scenario: List all feature files
    Given I have feature files in "spec/features/":
      | file                       | feature name              | scenarios |
      | gherkin-validation.feature | Gherkin Syntax Validation | 14        |
      | create-feature.feature     | Create Feature File       | 10        |
      | list-features.feature      | List Feature Files        | 8         |
    When I run `fspec list-features`
    Then the command should exit with code 0
    And the output should list all 3 feature files
    And the output should contain "spec/features/create-feature.feature - Create Feature File"
    And the output should contain "spec/features/gherkin-validation.feature - Gherkin Syntax Validation"
    And the output should contain "spec/features/list-features.feature - List Feature Files"
    And the output should contain "Found 3 feature files"

  Scenario: List features with scenario counts
    Given I have a feature file "spec/features/login.feature" with 5 scenarios
    When I run `fspec list-features`
    Then the output should contain "spec/features/login.feature - User Login (5 scenarios)"

  Scenario: Handle empty spec/features directory
    Given I have an empty "spec/features/" directory
    When I run `fspec list-features`
    Then the command should exit with code 0
    And the output should contain "No feature files found in spec/features/"

  Scenario: Handle missing spec/features directory
    Given no "spec/features/" directory exists
    When I run `fspec list-features`
    Then the command should exit with code 2
    And the output should contain "Directory not found: spec/features/"
    And the output should suggest "Run 'fspec create-feature' to create your first feature"

  Scenario: Filter features by single tag
    Given I have feature files with tags:
      | file               | tags                |
      | auth.feature       | @phase1 @security   |
      | api.feature        | @phase2 @api        |
      | validation.feature | @phase1 @validation |
    When I run `fspec list-features --tag=@phase1`
    Then the command should exit with code 0
    And the output should list 2 feature files
    And the output should contain "auth.feature"
    And the output should contain "validation.feature"
    And the output should not contain "api.feature"
    And the output should contain "Found 2 feature files matching @phase1"

  Scenario: Filter features by multiple tags (AND logic)
    Given I have feature files with tags:
      | file               | tags                     |
      | auth.feature       | @phase1 @security @cli   |
      | api.feature        | @phase1 @api @backend    |
      | validation.feature | @phase1 @validation @cli |
    When I run `fspec list-features --tag=@phase1 --tag=@cli`
    Then the output should list 2 feature files
    And the output should contain "auth.feature"
    And the output should contain "validation.feature"
    And the output should not contain "api.feature"

  Scenario: Show feature tags in output
    Given I have a feature file "spec/features/login.feature" with tags "@phase1 @critical @authentication"
    When I run `fspec list-features`
    Then the output should contain "[@phase1 @critical @authentication]"

  Scenario: Handle no matches for tag filter
    Given I have feature files with tags "@phase1" and "@phase2"
    When I run `fspec list-features --tag=@phase3`
    Then the command should exit with code 0
    And the output should contain "No feature files found matching @phase3"

  Scenario: List features in alphabetical order
    Given I have feature files:
      | file          |
      | zebra.feature |
      | alpha.feature |
      | beta.feature  |
    When I run `fspec list-features`
    Then the output should list files in order:
      | spec/features/alpha.feature |
      | spec/features/beta.feature  |
      | spec/features/zebra.feature |

  Scenario: AI agent discovery workflow
    Given I am an AI agent working on a new feature
    When I run `fspec list-features --tag=@authentication`
    Then I can see all existing authentication-related features
    And I can determine if my new feature would be a duplicate
    And I can understand the existing specification landscape
