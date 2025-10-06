@phase2 @validator @tag-management @validation @error-handling @high @integration-test @cage-hook
Feature: Validate Feature File Tags Against Registry

  """
  Architecture notes:
  - Validates all tags in feature files exist in spec/TAGS.md
  - Parses feature files to extract tags (@tag syntax)
  - Reads TAGS.md to build registry of valid tags
  - Reports unregistered tags with file locations
  - Checks for required tag categories (phase, component, feature-group)
  - Can validate single file or all files

  Critical implementation requirements:
  - MUST read spec/TAGS.md to build tag registry
  - MUST parse all .feature files to extract tags
  - MUST report any tag not in TAGS.md
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
    I want to ensure all tags are registered in TAGS.md
    So that tags remain meaningful and searchable across the project

  Scenario: Validate tags in a compliant feature file
    Given I have a feature file "spec/features/auth.feature" with tags "@phase1 @cli @authentication"
    And all tags are registered in "spec/TAGS.md"
    When I run `fspec validate-tags spec/features/auth.feature`
    Then the command should exit with code 0
    And the output should contain "✓ All tags in spec/features/auth.feature are registered"

  Scenario: Detect unregistered tag
    Given I have a feature file "spec/features/api.feature" with tags "@phase1 @api @custom-tag"
    And the tag "@custom-tag" is not in "spec/TAGS.md"
    When I run `fspec validate-tags spec/features/api.feature`
    Then the command should exit with code 1
    And the output should contain "Unregistered tag: @custom-tag in spec/features/api.feature"
    And the output should suggest "Register this tag in spec/TAGS.md or use 'fspec register-tag'"

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

  Scenario: Handle missing TAGS.md file
    Given no "spec/TAGS.md" file exists
    When I run `fspec validate-tags`
    Then the command should exit with code 2
    And the output should contain "TAGS.md not found: spec/TAGS.md"
    And the output should suggest "Create spec/TAGS.md to track tags"

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
