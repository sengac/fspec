@done
@feature-management
@cli
@RPC-190
Feature: add-scenario CLI subcommand (Rust shell front-door)
  """
  Files: rust/fspec/src/add_scenario.rs (NEW CLI bridge); rust/fspec/tests/cli_add_scenario.rs (NEW CLI tests); rust/fspec/tests/fixtures/help/add-scenario.txt (captured help fixture from `node dist/index.js add-scenario --help`).
  Bridge marshals positional <feature> + <scenario> into JSON {feature, scenario} and delegates to commands::add_scenario::run. No logic in bridge — JSON marshalling + CWD resolution only.
  Exit codes: 0 on success (✓ line + optional ⚠ warning to stdout), 1 on failure with 'Error:' + 'Suggestion:' to stderr/stdout.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want `fspec add-scenario <feature> <scenario>` to behave byte-identically to the TypeScript implementation
    So that I can add scenarios from a shell without depending on Node.js

  Scenario: CLI adds a scenario and prints the success line
    Given a tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I run 'fspec add-scenario spec/features/login.feature "Login with invalid password"' in that tempdir
    Then the process exits with code 0
    And stdout contains the substring '✓ Added scenario "Login with invalid password"'
    And the file spec/features/login.feature in the tempdir contains the line '  Scenario: Login with invalid password'

  Scenario: CLI prints a warning when the scenario name already exists
    Given a tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I run 'fspec add-scenario spec/features/login.feature "A"' in that tempdir
    Then the process exits with code 0
    And stdout contains the substring 'already exists in this feature'
    And stdout contains the substring '✓ Added scenario "A"'

  Scenario: CLI fails with exit 1 and Error prefix for a missing file
    Given a tempdir with NO spec/features/missing.feature file
    When I run 'fspec add-scenario spec/features/missing.feature "X"' in that tempdir
    Then the process exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Feature file not found:'

  Scenario: CLI help output matches captured TypeScript fixture byte-for-byte
    Given the standalone fspec Rust binary is built
    When I run 'fspec add-scenario --help'
    Then the process exits with code 0
    And stdout matches the captured fixture at rust/fspec/tests/fixtures/help/add-scenario.txt

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch add-scenario through fspec_core::dispatch::dispatch_command with feature='spec/features/login.feature' and scenario='From dispatcher'
    Then the dispatcher's DispatchResult.data parses to a structure whose success is true
    And the CLI bridge module rust/fspec/src/add_scenario.rs contains NO inline gherkin parsing or insertion logic
    And the bridge module's only computation is JSON arg marshalling and CWD resolution
