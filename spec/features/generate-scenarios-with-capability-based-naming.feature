@done
@cli
@example-mapping
@bug-fix
@critical
Feature: Generate Scenarios with Capability-Based Naming
  """
  Architecture notes:
  - Uses work unit title as default feature file name (kebab-cased)
  - Allows override via --feature flag for custom names
  - Prevents using work unit ID as file name (anti-pattern)
  - Validates work unit has title before defaulting

  Critical implementation requirements:
  - MUST NOT default to work unit ID as feature file name
  - MUST use work unit title (kebab-cased) as default
  - MUST throw error if work unit has no title and --feature not provided
  - MUST support --feature flag to override default naming
  - Error messages MUST guide user to use capability-based names

  References:
  - spec/CLAUDE.md - File Naming section (lines 372-439)
  - src/commands/generate-scenarios.ts:78-81 (bug location)
  """

  Background: User Story
    As an AI agent using fspec for ACDD workflow
    I want generate-scenarios to create feature files with capability-based names
    So that feature files are named after "what IS" (capability) not "what the current state is" (work unit ID)

  Scenario: Generate scenarios using work unit title as default
    Given I have a work unit "AUTH-001" with title "User Authentication"
    And the work unit has 3 examples from Example Mapping
    When I run "fspec generate-scenarios AUTH-001"
    Then a feature file "spec/features/user-authentication.feature" should be created
    And the file should contain scenarios generated from the examples
    And the file should NOT be named "auth-001.feature"

  Scenario: Generate scenarios with explicit feature name override
    Given I have a work unit "AUTH-001" with title "User Login"
    And the work unit has 2 examples from Example Mapping
    When I run "fspec generate-scenarios AUTH-001 --feature=user-authentication"
    Then a feature file "spec/features/user-authentication.feature" should be created
    And the file should contain scenarios generated from the examples

  Scenario: Error when work unit has no title and no --feature flag provided
    Given I have a work unit "AUTH-001" with no title set
    And the work unit has examples from Example Mapping
    When I run "fspec generate-scenarios AUTH-001"
    Then the command should fail with exit code 1
    And the error message should contain "Cannot determine feature file name"
    And the error message should suggest using "--feature flag with a capability-based name"
