@phase2
@validator
@tag-management
@validation
@error-handling
@high
@integration-test
@cage-hook
Feature: Validate Feature File Tags Against Registry
  """
  Architecture notes:
  - Validates all tags in feature files exist in spec/tags.json
  - Parses feature files to extract tags (@tag syntax)
  - Reads tags.json to build registry of valid tags
  - Reports unregistered tags with file locations
  - Checks for required tag categories (phase, component, feature-group)
  - Can validate single file or all files

  Critical implementation requirements:
  - MUST read spec/tags.json to build tag registry
  - MUST parse all .feature files to extract tags
  - MUST report any tag not in tags.json
  - MUST check for required tags (phase, component, feature-group)
  - MUST show file and line number for violations
  - Exit code 0 if all valid, 1 if violations found
  - SHOULD suggest registering unregistered tags

  Tag extraction:
  - Parse tags from feature-level (@phase1 @cli @validation)
  - Parse tags from scenario-level (less common but valid)
  - Skip tags in comments or doc strings

  Required tag validation:
  - Every feature MUST have one @phase tag
  - Every feature MUST have at least one component tag
  - Every feature MUST have at least one feature-group tag

  Integration points:
  - CLI command: `fspec validate-tags [file]`
  - Called from CAGE PreToolUse hook before spec modifications
  - Called before commits to ensure tag compliance
  """

  Background: User Story
    As a developer maintaining specification tag discipline
    I want to ensure all tags are registered in tags.json
    So that tags remain meaningful and searchable across the project

  Scenario: Validate tags in a compliant feature file
    Given I have a feature file "spec/features/auth.feature" with tags "@phase1 @cli @authentication"
    And all tags are registered in "spec/tags.json"
    When I run `fspec validate-tags spec/features/auth.feature`
    Then the command should exit with code 0
    And the output should contain "✓ All tags in spec/features/auth.feature are registered"

  Scenario: Detect unregistered tag
    Given I have a feature file "spec/features/api.feature" with tags "@phase1 @api @custom-tag"
    And the tag "@custom-tag" is not in "spec/tags.json"
    When I run `fspec validate-tags spec/features/api.feature`
    Then the command should exit with code 1
    And the output should contain "Unregistered tag: @custom-tag in spec/features/api.feature"
    And the output should suggest "Register this tag in spec/tags.json or use 'fspec register-tag'"

  Scenario: Validate all feature files
    Given I have feature files with various tags
    And most tags are registered but "@experimental" is not
    When I run `fspec validate-tags`
    Then the command should exit with code 1
    And the output should list all files with unregistered tags
    And the output should contain a summary of violations

  Scenario: Detect missing required phase tag
    Given I have a feature file "spec/features/broken.feature" with tags "@cli @validation"
    And the file is missing a @phase tag
    When I run `fspec validate-tags spec/features/broken.feature`
    Then the command should exit with code 1
    And the output should contain "Missing required phase tag (@phase1, @phase2, etc.)"
    And the output should suggest "Add a phase tag to the feature"

  Scenario: Detect missing required component tag
    Given I have a feature file with tags "@phase1 @validation"
    And the file is missing a component tag (@cli, @parser, etc.)
    When I run `fspec validate-tags spec/features/broken.feature`
    Then the command should exit with code 1
    And the output should contain "Missing required component tag"
    And the output should suggest available component tags

  Scenario: Detect missing required feature-group tag
    Given I have a feature file with tags "@phase1 @cli"
    And the file is missing a feature-group tag
    When I run `fspec validate-tags spec/features/broken.feature`
    Then the command should exit with code 1
    And the output should contain "Missing required feature-group tag"

  Scenario: Handle missing tags.json file
    Given no "spec/tags.json" file exists
    When I run `fspec validate-tags`
    Then the command should exit with code 2
    And the output should contain "tags.json not found: spec/tags.json"
    And the output should suggest "Create spec/tags.json to track tags"

  Scenario: Validate tags after creating new feature
    Given I create a new feature with `fspec create-feature "New Feature"`
    And the template includes placeholder tags
    When I run `fspec validate-tags spec/features/new-feature.feature`
    Then the output should warn about placeholder tags
    And the output should suggest "Replace @component and @feature-group with actual tags"

  Scenario: Report multiple violations in one file
    Given I have a feature file with tags "@phase1 @unknown1 @unknown2"
    And both "@unknown1" and "@unknown2" are unregistered
    When I run `fspec validate-tags spec/features/multi.feature`
    Then the output should list both unregistered tags
    And the output should contain "Found 2 unregistered tags"

  Scenario: Validate tags in multiple files with summary
    Given I have 10 feature files
    And 8 files have valid tags
    And 2 files have unregistered tags
    When I run `fspec validate-tags`
    Then the output should contain "✓ 8 files passed"
    And the output should contain "✗ 2 files have tag violations"
    And the output should list the failing files

  Scenario: CAGE integration - prevent invalid tag commits
    Given I am working in a CAGE-monitored project
    When a PreToolUse hook runs `fspec validate-tags`
    And unregistered tags are detected
    Then the hook can warn the AI agent
    And the AI agent can fix tags before proceeding
    And specifications remain compliant with tag registry

  Scenario: JSON-backed workflow - validate against tags.json registry
    Given I have tags.json with registered tags in multiple categories
    And I have feature files using both registered and unregistered tags
    When I run `fspec validate-tags`
    Then the command should load tag registry from spec/tags.json
    And validate all tags against the JSON registry
    And report unregistered tags with file locations
    And check for required tag categories (phase, component, feature-group)
    And the command should exit with code 1 if violations found

  Scenario: Validate scenario-level tags are registered
    Given I have a feature file with scenario-level tags:
      """
      @phase1
      @cli
      @authentication
      Feature: User Login

        @smoke
        @regression
        Scenario: Successful login
          Given I am on the login page
          When I enter valid credentials
          Then I should be logged in
      """
    And the tags "@smoke" and "@regression" are registered in spec/tags.json
    When I run `fspec validate-tags spec/features/login.feature`
    Then the command should exit with code 0
    And scenario-level tags should be validated against the registry

  Scenario: Detect unregistered scenario-level tag
    Given I have a feature file with an unregistered scenario tag:
      """
      @phase1
      @cli
      @authentication
      Feature: User Login

        @smoke
        @unregistered-scenario-tag
        Scenario: Test scenario
          Given a step
          When another step
          Then result
      """
    And the tag "@unregistered-scenario-tag" is not in spec/tags.json
    When I run `fspec validate-tags spec/features/login.feature`
    Then the command should exit with code 1
    And the output should contain "Unregistered tag: @unregistered-scenario-tag"
    And the output should show the scenario name where the tag was found

  Scenario: Validate both feature-level and scenario-level tags
    Given I have a feature file with tags at both levels:
      """
      @phase1
      @cli
      @authentication
      Feature: User Login

        @smoke
        Scenario: Basic login
          Given I am on the login page
          When I enter credentials
          Then I am logged in

        @regression
        @edge-case
        Scenario: Login with expired session
          Given I have an expired session
          When I attempt to login
          Then I am prompted to re-authenticate
      """
    And all feature-level and scenario-level tags are registered
    When I run `fspec validate-tags spec/features/login.feature`
    Then the command should exit with code 0
    And all tags at all levels should be validated

  Scenario: Detect mix of registered and unregistered scenario tags
    Given I have a feature file with multiple scenarios:
      """
      @phase1
      @cli
      @authentication
      Feature: User Login

        @smoke
        Scenario: Valid scenario
          Given a step
          When another step
          Then result

        @unregistered-tag1
        @unregistered-tag2
        Scenario: Invalid scenario
          Given a step
          When another step
          Then result
      """
    And "@smoke" is registered but "@unregistered-tag1" and "@unregistered-tag2" are not
    When I run `fspec validate-tags spec/features/login.feature`
    Then the command should exit with code 1
    And the output should list both unregistered tags
    And the output should specify which scenarios contain the invalid tags

  Scenario: Validate scenario tags do not require phase/component/feature-group tags
    Given I have a feature file with properly tagged feature and minimal scenario tags:
      """
      @phase1
      @cli
      @authentication
      Feature: User Login

        @smoke
        Scenario: Quick test
          Given a step
          When another step
          Then result
      """
    When I run `fspec validate-tags spec/features/login.feature`
    Then the command should exit with code 0
    And scenario tags should not be required to have phase/component/feature-group tags
    And only feature-level tags should be checked for required categories
