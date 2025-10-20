@done
@feature-management
@cli
@phase2
@BUG-022
Feature: Feature file naming for bug work units
  """
  Fix should be in src/commands/generate-scenarios.ts - the command responsible for creating feature files from work units
  Need to add capability extraction logic - analyze bug description to identify underlying capability being tested
  Need to add existing feature search - use fspec list-features or directly scan spec/features/ directory with feature file parsing
  When adding to existing file, use fspec add-scenario command to properly insert scenario with correct formatting and tags
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. AI should extract capability from bug context and search for existing feature files before creating new ones
  #   2. AI agent should determine if an existing feature file is suitable for the bug scenario
  #   3. Add or update bug scenario in existing feature file (and update associated tests)
  #   4. Feature file names must be capability-oriented (what the system CAN DO), not task-oriented (the bug description)
  #   5. AI should analyze existing feature descriptions, scenarios, and tags to determine capability match
  #   6. If no suitable existing feature file exists, create new file with capability-oriented name derived from bug context
  #
  # EXAMPLES:
  #   1. Bug: 'fspec help displays hardcoded version 0.0.1' → AI searches features, finds 'help-command.feature' → Adds bug-fix scenario to existing file
  #   2. Bug: 'fspec help displays hardcoded version 0.0.1' → AI searches features, no match found → Creates 'cli-version-display.feature' (capability-oriented name)
  #   3. Bug: 'validate command crashes on empty file' → AI finds 'gherkin-validation.feature' → Adds edge-case scenario to existing file
  #   4. Bug: 'format removes valid doc strings' → AI finds 'gherkin-formatting.feature' → Updates existing scenario or adds new regression scenario
  #
  # QUESTIONS (ANSWERED):
  #   Q: When generating feature files from bug work units, what rules should determine the capability name? Should we extract it from the bug description or ask the developer?
  #   A: true
  #
  #   Q: Should the system check for existing feature files covering the same capability before creating a new file? How should it determine if a match exists?
  #   A: true
  #
  #   Q: If an existing feature file is found that covers the capability, should we add a bug-fix scenario to that file or create a separate file?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer using fspec to fix bugs
    I want to create feature files with capability-oriented names
    So that bug tests document capabilities, not problems

  Scenario: Bug scenario added to existing matching feature file
    Given I have a bug work unit "BUG-001" with description "fspec help displays hardcoded version 0.0.1 instead of package.json version"
    And an existing feature file "spec/features/help-command.feature" exists covering CLI help functionality
    When I run "fspec generate-scenarios BUG-001"
    Then the system should identify "help-command.feature" as matching the capability
    And a new scenario should be added to "spec/features/help-command.feature"
    And the scenario should describe the version display bug fix
    And the feature file name should remain "help-command.feature" (capability-oriented)
    And no new feature file should be created

  Scenario: New capability-oriented feature file created when no match found
    Given I have a bug work unit "BUG-002" with description "fspec help displays hardcoded version 0.0.1 instead of package.json version"
    And no existing feature file covers CLI version display capability
    When I run "fspec generate-scenarios BUG-002"
    Then the system should determine no existing feature matches the capability
    And a new feature file should be created with capability-oriented name
    And the feature file should be named "cli-version-display.feature" or similar capability name
    And the feature file should NOT be named "fspec-help-displays-hardcoded-version.feature"
    And the feature file should contain a scenario describing the version display capability

  Scenario: Edge case scenario added to existing validation feature
    Given I have a bug work unit "BUG-003" with description "validate command crashes on empty file"
    And an existing feature file "spec/features/gherkin-validation.feature" exists
    When I run "fspec generate-scenarios BUG-003"
    Then the system should identify "gherkin-validation.feature" as matching the capability
    And a new edge-case scenario should be added to "spec/features/gherkin-validation.feature"
    And the scenario should describe validation behavior for empty files
    And the feature file name should remain "gherkin-validation.feature"

  Scenario: Regression scenario added to existing formatting feature
    Given I have a bug work unit "BUG-004" with description "format command removes valid doc strings"
    And an existing feature file "spec/features/gherkin-formatting.feature" exists
    And the feature already has a scenario covering doc string formatting
    When I run "fspec generate-scenarios BUG-004"
    Then the system should identify "gherkin-formatting.feature" as matching the capability
    And either update the existing doc string scenario or add a new regression scenario
    And the feature file name should remain "gherkin-formatting.feature"
    And the scenario should describe doc string preservation behavior
