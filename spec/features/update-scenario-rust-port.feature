@done
@RPC-314
@wip
@file-ops
@feature-management
Feature: Port update-scenario command to Rust
  """
  Single source of truth: pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>; both dispatcher and CLI bridge converge here (RPC-003 two-front-doors)
  Use parse_feature_lenient (io/gherkin.rs) for parity with @cucumber/gherkin tolerance; line-based edit via split('\n')/join('\n'); recoverable errors returned as inner JSON envelope {success:false,error} like list_scenario_tags.rs
  Coverage rename uses crate::types::coverage::CoverageFile (serde flatten extra preserves unknown fields); re-serialised pretty (2-space) to match TS JSON.stringify(_,null,2)
  """

  Background: User Story
    As a fspec developer
    I want to rename a scenario in a feature file via the Rust port of update-scenario
    So that scenarios can be renamed while preserving coverage mappings, behaviour-identical to the TypeScript command

  Scenario: Rename a scenario and update its coverage entry
    Given a feature file "spec/features/user-auth.feature" containing a scenario "Login with valid credentials"
    And a coverage file "spec/features/user-auth.feature.coverage" with a scenario entry "Login with valid credentials" carrying test mappings
    When I dispatch update-scenario with feature "spec/features/user-auth.feature" old-name "Login with valid credentials" new-name "Login with email and password"
    Then the response has success true
    And the response message is "Successfully renamed scenario to 'Login with email and password' in user-auth.feature"
    And the feature file header line reads "  Scenario: Login with email and password"
    And the coverage entry is renamed to "Login with email and password" with its test mappings preserved

  Scenario: Header indentation and keyword are preserved on rename
    Given a feature file "spec/features/outline.feature" containing a scenario outline "Old outline name" indented by two spaces
    When I dispatch update-scenario with feature "spec/features/outline.feature" old-name "Old outline name" new-name "New outline name"
    Then the response has success true
    And the feature file header line reads "  Scenario Outline: New outline name"

  Scenario: Renaming a scenario in a missing feature file fails
    Given no feature file exists at "spec/features/missing.feature"
    When I dispatch update-scenario with feature "spec/features/missing.feature" old-name "A" new-name "B"
    Then the response has success false
    And the response error contains "Feature file not found:"

  Scenario: Renaming a scenario that is not present fails
    Given a feature file "spec/features/user-auth.feature" containing a scenario "Login with valid credentials"
    When I dispatch update-scenario with feature "spec/features/user-auth.feature" old-name "Nonexistent" new-name "Whatever"
    Then the response has success false
    And the response error is "Scenario 'Nonexistent' not found in feature file"

  Scenario: Renaming to an existing scenario name fails and leaves the file unchanged
    Given a feature file "spec/features/user-auth.feature" containing scenarios "First scenario" and "Second scenario"
    When I dispatch update-scenario with feature "spec/features/user-auth.feature" old-name "First scenario" new-name "Second scenario"
    Then the response has success false
    And the response error is "Scenario 'Second scenario' already exists in this feature"

  Scenario: Renaming succeeds even when no coverage file exists
    Given a feature file "spec/features/no-coverage.feature" containing a scenario "Only scenario"
    And no coverage file exists at "spec/features/no-coverage.feature.coverage"
    When I dispatch update-scenario with feature "spec/features/no-coverage.feature" old-name "Only scenario" new-name "Renamed scenario"
    Then the response has success true
    And the feature file header line reads "  Scenario: Renamed scenario"
