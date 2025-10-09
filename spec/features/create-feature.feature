@phase1
@cli
@generator
@feature-management
@gherkin
@template
@cross-platform
@critical
@unit-test
@integration-test
Feature: Create Feature File with Template
  """
  Architecture notes:
  - Generates new .feature files in spec/features/ directory
  - Uses template with placeholders for Feature name, Background, and Scenario
  - Converts feature names to kebab-case for file naming
  - Validates spec/features/ directory exists, creates if needed
  - Template includes proper Gherkin structure with tag placeholders

  Critical implementation requirements:
  - MUST create valid Gherkin syntax files
  - MUST use kebab-case for file names (e.g., "User Login" →
  "user-login.feature")
  - MUST include template tags (@phase1, @component, @feature-group
  placeholders)

  - MUST include Background section with user story template
  - MUST include one example Scenario with Given/When/Then
  - MUST NOT overwrite existing files without confirmation
  - File MUST pass gherkin validation after creation

  Template structure:
  - Tags: @phase1 @component @feature-group
  - Feature: <name>
  - Doc string (\"\"\") with architecture notes placeholder
  - Background: User Story (As a... I want... So that...)
  - Scenario: Example scenario with Given/When/Then steps

  Integration points:
  - CLI command: `fspec create-feature <name>`
  - Called by AI agents to scaffold new specifications
  - Output file ready for immediate editing and validation
  """

  Background: User Story
    As a developer using AI agents for spec-driven development
    I want to create new feature files with proper Gherkin structure
    So that AI can write valid specifications without manual setup

  Scenario: Create feature file with valid name
    Given I am in a project with a spec/features/ directory
    When I run `fspec create-feature "User Authentication"`
    Then a file "spec/features/user-authentication.feature" should be created
    And the file should contain a valid Gherkin feature structure
    And the file should contain "Feature: User Authentication"
    And the file should include a Background section with user story template
    And the file should include a Scenario placeholder
    And the file should include tag placeholders "@phase1 @component @feature-group"
    And the file should pass gherkin validation

  Scenario: Convert feature name to kebab-case
    Given I am in a project with a spec/features/ directory
    When I run `fspec create-feature "Real Time Event Monitoring"`
    Then a file "spec/features/real-time-event-monitoring.feature" should be created
    And the file should contain "Feature: Real Time Event Monitoring"

  Scenario: Create spec/features/ directory if it doesn't exist
    Given I am in a project without a spec/features/ directory
    When I run `fspec create-feature "New Feature"`
    Then the directory "spec/features/" should be created
    And a file "spec/features/new-feature.feature" should be created

  Scenario: Prevent overwriting existing file
    Given I have an existing file "spec/features/user-login.feature"
    When I run `fspec create-feature "User Login"`
    Then the command should exit with code 1
    And the output should contain "File already exists: spec/features/user-login.feature"
    And the output should suggest "Use a different name or delete the existing file"
    And the existing file should not be modified

  Scenario: Handle special characters in feature name
    Given I am in a project with a spec/features/ directory
    When I run `fspec create-feature "API/REST Endpoints"`
    Then a file "spec/features/api-rest-endpoints.feature" should be created
    And the file should contain "Feature: API/REST Endpoints"

  Scenario: Create feature with minimal name
    Given I am in a project with a spec/features/ directory
    When I run `fspec create-feature "Login"`
    Then a file "spec/features/login.feature" should be created
    And the file should contain "Feature: Login"

  Scenario: Show success message with file path
    Given I am in a project with a spec/features/ directory
    When I run `fspec create-feature "User Permissions"`
    Then the command should exit with code 0
    And the output should contain "✓ Created spec/features/user-permissions.feature"
    And the output should suggest "Edit the file to add your scenarios"

  Scenario: Created file has proper template structure
    Given I am in a project with a spec/features/ directory
    When I run `fspec create-feature "Data Export"`
    Then the file "spec/features/data-export.feature" should contain proper Gherkin structure
    And the file should have tag placeholders
    And the file should have architecture notes section
    And the file should have user story template in Background
    And the file should have example scenario with steps

  Scenario: AI agent workflow - create and validate
    Given I am an AI agent creating a new specification
    When I run `fspec create-feature "Shopping Cart"`
    Then a file "spec/features/shopping-cart.feature" should be created
    And when I run `fspec validate spec/features/shopping-cart.feature`
    Then the validation should pass
    And I can immediately edit the file to add real scenarios
