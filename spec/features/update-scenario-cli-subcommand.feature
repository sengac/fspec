@done
@RPC-314
@wip
@cli
@feature-management
Feature: update-scenario CLI subcommand (Rust shell front-door)
  """
  Files: codelet/fspec/src/update_scenario.rs (NEW CLI bridge); codelet/fspec/tests/cli_update_scenario.rs (NEW CLI tests); codelet/fspec/tests/fixtures/help/update-scenario.txt (captured help fixture from `node dist/index.js update-scenario --help`).
  Bridge marshals positional <feature> <old-name> <new-name> into JSON and delegates to commands::update_scenario::run. No logic in bridge — JSON marshalling + CWD resolution only.
  Exit codes: 0 on success (✓ message to stdout), 1 on FspecCoreError or {success:false} with 'Error:' prefix to stderr.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want `fspec update-scenario <feature> <old-name> <new-name>` to behave byte-identically to the TypeScript implementation
    So that I can rename scenarios from a shell without depending on Node.js

  Scenario: CLI successfully renames a scenario and prints the success line
    Given a tempdir with spec/features/user-auth.feature containing a scenario "Login with valid credentials"
    When I run 'fspec update-scenario spec/features/user-auth.feature "Login with valid credentials" "Login with email and password"' in that tempdir
    Then the process exits with code 0
    And stdout contains the substring '✓ Successfully renamed scenario to '"'"'Login with email and password'"'"' in user-auth.feature'
    And the file spec/features/user-auth.feature in the tempdir contains the line '  Scenario: Login with email and password'

  Scenario: CLI surfaces a missing-file error with stderr Error prefix and exit 1
    Given a tempdir with no spec/features/missing.feature
    When I run 'fspec update-scenario spec/features/missing.feature "A" "B"' in that tempdir
    Then the process exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Feature file not found:'

  Scenario: CLI surfaces a not-found scenario error with exit 1
    Given a tempdir with spec/features/user-auth.feature containing a scenario "Login with valid credentials"
    When I run 'fspec update-scenario spec/features/user-auth.feature "Nonexistent" "Whatever"' in that tempdir
    Then the process exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Scenario '"'"'Nonexistent'"'"' not found in feature file'

  Scenario: CLI help output matches captured TypeScript fixture byte-for-byte
    Given the standalone fspec Rust binary is built
    When I run 'fspec update-scenario --help'
    Then the process exits with code 0
    And stdout matches the captured fixture at codelet/fspec/tests/fixtures/help/update-scenario.txt

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/features/user-auth.feature containing a scenario "Login with valid credentials"
    When I dispatch update-scenario through fspec_core::dispatch::dispatch_command with feature='spec/features/user-auth.feature' oldName='Login with valid credentials' newName='Renamed'
    Then the dispatcher's DispatchResult.data parses to a structure whose message contains 'Successfully renamed scenario to '"'"'Renamed'"'"' in user-auth.feature'
    And the CLI bridge module codelet/fspec/src/update_scenario.rs contains NO inline gherkin parsing or scenario-rename logic
    And the bridge module's only computation is JSON arg marshalling and CWD resolution
