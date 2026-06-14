@done
@RPC-315 @wip @cli @feature-management
Feature: update-step CLI subcommand (Rust shell front-door)

  """
  Files: codelet/fspec/src/update_step.rs (NEW CLI bridge); codelet/fspec/tests/cli_update_step.rs (NEW CLI tests); codelet/fspec/tests/fixtures/help/update-step.txt (captured help fixture from `node dist/index.js update-step --help`).
  Bridge marshals positional <feature> <scenario> <current-step> + optional --text/--keyword into JSON and delegates to commands::update_step::run. No logic in bridge — JSON marshalling + CWD resolution only.
  Exit codes: 0 on success (✓ message to stdout), 1 on FspecCoreError or {success:false} with 'Error:' prefix to stderr.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want `fspec update-step <feature> <scenario> <current-step> [--text] [--keyword]` to behave byte-identically to the TypeScript implementation
    So that I can update steps from a shell without depending on Node.js

  Scenario: CLI successfully updates step text and prints the success line
    Given a tempdir with spec/features/user-auth.feature with scenario "Valid login" containing step "Given I am on the login page"
    When I run 'fspec update-step spec/features/user-auth.feature "Valid login" "Given I am on the login page" --text "I navigate to the login page"' in that tempdir
    Then the process exits with code 0
    And stdout contains the substring '✓ Successfully updated step in scenario '"'"'Valid login'"'"' in user-auth.feature'
    And the file spec/features/user-auth.feature in the tempdir contains the line '    Given I navigate to the login page'

  Scenario: CLI changes a step keyword and prints the success line
    Given a tempdir with spec/features/user-auth.feature with scenario "Valid login" containing step "Given I am logged out"
    When I run 'fspec update-step spec/features/user-auth.feature "Valid login" "Given I am logged out" --keyword When' in that tempdir
    Then the process exits with code 0
    And the file spec/features/user-auth.feature in the tempdir contains the line '    When I am logged out'

  Scenario: CLI rejects missing updates with stderr Error prefix and exit 1
    Given a tempdir with spec/features/user-auth.feature with scenario "Valid login" containing step "Given I am on the login page"
    When I run 'fspec update-step spec/features/user-auth.feature "Valid login" "Given I am on the login page"' in that tempdir
    Then the process exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'No updates specified. Use --text and/or --keyword'

  Scenario: CLI surfaces a missing-file error with exit 1
    Given a tempdir with no spec/features/missing.feature
    When I run 'fspec update-step spec/features/missing.feature "S" "Given x" --text "Given y"' in that tempdir
    Then the process exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Feature file not found:'

  Scenario: CLI help output matches captured TypeScript fixture byte-for-byte
    Given the standalone fspec Rust binary is built
    When I run 'fspec update-step --help'
    Then the process exits with code 0
    And stdout matches the captured fixture at codelet/fspec/tests/fixtures/help/update-step.txt

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/features/user-auth.feature with scenario "Valid login" containing step "Given I am on the login page"
    When I dispatch update-step through fspec_core::dispatch::dispatch_command with feature='spec/features/user-auth.feature' scenario='Valid login' currentStep='Given I am on the login page' text='I navigate to the login page'
    Then the dispatcher's DispatchResult.data parses to a structure whose message contains 'Successfully updated step in scenario '"'"'Valid login'"'"' in user-auth.feature'
    And the CLI bridge module codelet/fspec/src/update_step.rs contains NO inline gherkin parsing or step-update logic
    And the bridge module's only computation is JSON arg marshalling and CWD resolution
