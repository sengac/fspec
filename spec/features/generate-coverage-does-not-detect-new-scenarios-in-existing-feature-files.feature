@cli
@coverage
@phase1
@BUG-007
Feature: generate-coverage does not detect new scenarios in existing feature files

  Background: User Story
    As a developer following ACDD workflow
    I want to have generate-coverage detect and add new scenarios to existing .coverage files
    So that I can link test coverage for new scenarios without manual JSON editing

  Scenario: work-unit-dependency-management.feature has 42 scenarios, but .coverage file only has 39 scenario entries
    Given REPRODUCTION: work-unit-dependency-management.feature has 42 scenarios, but .coverage file only has 39 scenario entries
    When I perform the operation
    Then the result should be as expected

  Scenario: Scenario 'Identify bottleneck work units blocking the most work' (line 501-518) NOT in coverage file
    Given [precondition]
    When [action]
    Then [expected outcome]

  Scenario: Scenario 'Auto-suggest dependency relationships based on work unit metadata' (line 520-536) NOT in coverage file
    Given [precondition]
    When [action]
    Then [expected outcome]

  Scenario: Scenario 'Detect orphaned work units with no epic or dependencies' (line 538-553) NOT in coverage file
    Given [precondition]
    When [action]
    Then [expected outcome]

  Scenario: 'fspec generate-coverage' runs without error but does NOT add missing scenarios
    Given I am COMMAND RESULT: 'fspec generate-coverage'
    When I runs without error but does NOT add missing scenarios
    Then the operation should succeed

  Scenario: 'fspec link-coverage work-unit-dependency-management --scenario "Identify bottleneck..."' fails with 'Scenario not found'
    Given I have an invalid condition
    When I execute ERROR WHEN LINKING: 'fspec link-coverage work-unit-dependency-management --scenario "Identify bottleneck..."'
    Then it should fails with 'Scenario not found'

  Scenario: spec/features/work-unit-dependency-management.feature - grep shows 42 scenarios: 'grep -c "^  Scenario:" spec/features/work-unit-dependency-management.feature' returns 42
    Given I am FILE: spec/features/work-unit-dependency-management.feature - grep
    When I shows 42 scenarios: 'grep -c "^  Scenario:" spec/features/work-unit-dependency-management.feature' returns 42
    Then the operation should succeed

  Scenario: spec/features/work-unit-dependency-management.feature.coverage - only has 39 entries in scenarios array (verified by reading JSON)
    Given FILE: spec/features/work-unit-dependency-management.feature.coverage - only has 39 entries in scenarios array (verified by reading JSON)
    When I perform the operation
    Then the result should be as expected

  Scenario: Feature file line 501: '@COV-046' tag, Scenario: 'Identify bottleneck work units blocking the most work' - MISSING from coverage
    Given [precondition]
    When [action]
    Then [expected outcome]

  Scenario: Feature file line 520: '@COV-047' tag, Scenario: 'Auto-suggest dependency relationships based on work unit metadata' - MISSING from coverage
    Given [precondition]
    When [action]
    Then [expected outcome]

  Scenario: Feature file line 538: '@COV-048' tag, Scenario: 'Detect orphaned work units with no epic or dependencies' - MISSING from coverage
    Given [precondition]
    When [action]
    Then [expected outcome]

  Scenario: 1) Add new scenario to existing .feature file 2) Run 'fspec generate-coverage' 3) Check .coverage file - new scenario NOT added 4) Try 'fspec link-coverage <feature> --scenario <name>' - ERROR: Scenario not found
    Given I am COMMAND TO REPRODUCE: 1)
    When I Add new scenario to existing .feature file 2) Run 'fspec generate-coverage' 3) Check .coverage file - new scenario NOT added 4) Try 'fspec link-coverage <feature> --scenario <name>' - ERROR: Scenario not found
    Then the operation should succeed
