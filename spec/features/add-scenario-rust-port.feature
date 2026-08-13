@done
@feature-management
@cli
@RPC-190
Feature: Port add-scenario command to Rust
  """
  Core impl at rust/fspec-core/src/commands/add_scenario.rs: run(args_json,&Path) args {feature, scenario, dryRun?}. Path resolution mirrors TS endsWith/startsWith/else against project_root. Validation + duplicate detection via crate::io::gherkin::parse_feature_lenient; scenario-name comparison uses scenario.name with keyword.trim()=='Scenario' filter. Insertion is TS line-based split('\n')/slice/join, NOT AST round-trip. Response {success, valid, warning?}; CLI bridge prints ✓/⚠ and Error+Suggestion on failure. Two-front-doors: bridge marshals <feature> <scenario> + optional --dry-run into JSON only.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Feature identifier resolves to a path: ends with '.feature' or starts with 'spec/features/' is used relative to project root; otherwise it becomes spec/features/<id>.feature
  #   2. A missing feature file MUST fail with success=false, error='Feature file not found: <resolvedPath>' and suggestion="Use 'fspec create-feature' to create a new feature file"
  #   3. Invalid existing Gherkin MUST fail with success=false and error='Feature file has invalid Gherkin syntax: ...' (or 'Feature file does not contain a valid Feature' when no Feature header)
  #   4. A duplicate scenario name MUST NOT abort: the command proceeds but returns warning='A scenario named "<name>" already exists in this feature'
  #   5. The appended scenario block MUST be exactly '\n  Scenario: <name>\n    Given [precondition]\n    When [action]\n    Then [expected outcome]\n' (placeholder steps)
  #   6. Insertion point MUST be immediately before the first line whose trim starts with 'Scenario Outline:' or 'Scenario Template:'; if none exists the scenario is appended at end of file, joining via the TS slice/join('\n') algorithm
  #   7. The result MUST be re-parsed to set a 'valid' boolean, the file is always written (unless dryRun) and success=true is returned
  #   8. When dryRun is true the new content MUST NOT be written to disk
  #
  # EXAMPLES:
  #   1. adding 'Login with invalid password' to spec/features/login.feature appends a Scenario block with placeholder Given/When/Then and returns success=true valid=true
  #   2. adding a scenario to a feature that already contains a 'Scenario Outline:' inserts the new scenario before the outline, not at end of file
  #   3. adding a scenario whose name already exists still succeeds but returns a 'already exists' warning
  #   4. running 'fspec add-scenario missing "X"' against a non-existent file prints 'Error: Feature file not found: ...' plus a 'Suggestion:' line and exits 1
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to add a new scenario with placeholder Given/When/Then steps to an existing feature file
    So that I can grow a feature specification incrementally without hand-editing Gherkin or depending on Node.js

  Scenario: Appends a scenario with placeholder steps to a feature
    Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch add-scenario with feature='spec/features/login.feature' and scenario='Login with invalid password'
    Then the dispatcher returns success=true and valid=true
    And the file on disk contains the line '  Scenario: Login with invalid password'
    And the file on disk contains the placeholder steps '[precondition]', '[action]', and '[expected outcome]'

  Scenario: Resolves a bare identifier to spec/features/<id>.feature
    Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch add-scenario with feature='login' and scenario='Another scenario'
    Then the dispatcher returns success=true
    And the file spec/features/login.feature contains the line '  Scenario: Another scenario'

  Scenario: Inserts the new scenario before an existing Scenario Outline
    Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n\n  Scenario Outline: O\n    Given <v>\n'
    When I dispatch add-scenario with feature='spec/features/login.feature' and scenario='Inserted'
    Then the dispatcher returns success=true
    And in the file on disk the line '  Scenario: Inserted' appears before the line '  Scenario Outline: O'

  Scenario: Duplicate scenario name succeeds with a warning
    Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch add-scenario with feature='spec/features/login.feature' and scenario='A'
    Then the dispatcher returns success=true
    And the dispatcher warning equals 'A scenario named "A" already exists in this feature'

  Scenario: Missing feature file surfaces the not-found error
    Given a project root tempdir with NO spec/features/missing.feature file
    When I dispatch add-scenario with feature='spec/features/missing.feature' and scenario='X'
    Then the dispatcher returns success=false
    And the error contains 'Feature file not found: '
    And the suggestion equals "Use 'fspec create-feature' to create a new feature file"

  Scenario: Dry run does not write the file
    Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch add-scenario with feature='spec/features/login.feature', scenario='Ghost' and dryRun=true
    Then the dispatcher returns success=true
    And the file spec/features/login.feature does NOT contain the line '  Scenario: Ghost'
