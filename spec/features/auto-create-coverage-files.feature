@done
@file-ops
@coverage-tracking
@cli
@phase2
Feature: Auto-create Coverage Files
  """
  Architecture notes:
  - Modify create-feature command (src/commands/create-feature.ts) to call coverage file creation after feature file is written
  - Use @cucumber/gherkin to parse newly created .feature file and extract scenario names
  - JSON schema: {scenarios: array of {name, testMappings}, stats: {totalScenarios, coveredScenarios, coveragePercent, testFiles, implFiles, totalLinesCovered}}
  - Coverage file creation is atomic: only create if feature file creation succeeds
  - File naming: <feature-name>.feature.coverage (e.g., user-login.feature.coverage)
  - Location: Same directory as .feature file (spec/features/)

  Critical implementation requirements:
  - MUST check if .coverage file exists before creating
  - MUST validate existing .coverage file JSON (if exists)
  - MUST skip creation if valid .coverage exists (preserve existing coverage data)
  - MUST overwrite if invalid JSON (corrupted file)
  - MUST display appropriate message: 'Created', 'Skipped', or 'Recreated'
  - MUST extract scenario names from Gherkin AST (exact match including case/whitespace)
  - MUST initialize all scenarios with empty testMappings array
  - MUST calculate initial stats: totalScenarios=count, all others=0 or empty arrays
  - Coverage file creation MUST NOT cause feature file creation to fail (graceful degradation)
  """

  Background: User Story
    As a developer creating feature files
    I want to have .feature.coverage files automatically created when I run create-feature
    So that I don't have to manually create coverage tracking files

  Scenario: Create feature file with coverage file
    Given I am in a project with spec/features/ directory
    When I run "fspec create-feature User Login"
    Then a file "spec/features/user-login.feature" should be created
    And a file "spec/features/user-login.feature.coverage" should be created
    And the output should display "✓ Created user-login.feature.coverage"

  Scenario: Coverage file contains valid JSON schema
    Given I have created a feature file "spec/features/user-login.feature" with 1 scenario named "Login with valid credentials"
    When I read the coverage file "spec/features/user-login.feature.coverage"
    Then the JSON should have a "scenarios" array with 1 entry
    And the scenario entry should have name "Login with valid credentials"
    And the scenario entry should have empty "testMappings" array
    And the JSON should have "stats.totalScenarios" equal to 1
    And the JSON should have "stats.coveredScenarios" equal to 0
    And the JSON should have "stats.coveragePercent" equal to 0
    And the JSON should have "stats.testFiles" as empty array
    And the JSON should have "stats.implFiles" as empty array
    And the JSON should have "stats.totalLinesCovered" equal to 0

  Scenario: Coverage file tracks multiple scenarios
    Given I have created a feature file with 3 scenarios
    When I read the coverage file
    Then the "scenarios" array should have 3 entries
    And the "stats.totalScenarios" should equal 3
    And all scenarios should have empty "testMappings" arrays

  Scenario: Skip coverage creation if valid coverage file exists
    Given I have a feature file "spec/features/user-login.feature"
    And a valid coverage file "spec/features/user-login.feature.coverage" already exists
    When I run "fspec create-feature User Login"
    Then the existing coverage file should not be modified
    And the output should display "Skipped user-login.feature.coverage (already exists)"

  Scenario: Overwrite coverage file if invalid JSON exists
    Given I have a feature file "spec/features/user-login.feature"
    And a coverage file "spec/features/user-login.feature.coverage" exists with invalid JSON
    When I run "fspec create-feature User Login"
    Then the coverage file should be overwritten with valid JSON
    And the output should display "Recreated user-login.feature.coverage (previous file was invalid)"

  Scenario: Scenario names preserve exact case and whitespace
    Given I create a feature file with scenario "Login With Valid Credentials"
    When I read the coverage file
    Then the scenario name should exactly match "Login With Valid Credentials"
    And the scenario name should not be "login-with-valid-credentials"
    And the scenario name should not be "Login with valid credentials"

  Scenario: Coverage file creation does not fail feature creation
    Given I am creating a feature file "spec/features/test.feature"
    And coverage file creation encounters an error
    When the feature file creation completes
    Then the feature file "spec/features/test.feature" should exist
    And a warning should be displayed about coverage file failure
    And the command should exit with code 0
