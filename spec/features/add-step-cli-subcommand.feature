@done
@feature-management
@cli
@RPC-192
Feature: add-step CLI subcommand (Rust shell front-door)

  """
  Files: codelet/fspec/src/add_step.rs (NEW CLI bridge); codelet/fspec/tests/cli_add_step.rs (NEW CLI tests); codelet/fspec/tests/fixtures/help/add-step.txt (captured help fixture from `node dist/index.js add-step --help`).
  Bridge marshals positional <feature> <scenario> <type> <text> into JSON {feature, scenario, type, text} and delegates to commands::add_step::run. No logic in bridge — JSON marshalling + CWD resolution only.
  Exit codes: 0 on success (✓ line to stdout), 1 on failure with 'Error:' + 'Suggestion:' to stderr/stdout.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want `fspec add-step <feature> <scenario> <type> <text>` to behave byte-identically to the TypeScript implementation
    So that I can add steps from a shell without depending on Node.js

  Scenario: CLI adds a step and prints the success line
    Given a tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: Login\n    Given x\n'
    When I run 'fspec add-step spec/features/login.feature "Login" given "I am on the login page"' in that tempdir
    Then the process exits with code 0
    And stdout contains the substring '✓ Added given step to scenario "Login"'
    And the file spec/features/login.feature in the tempdir contains the line 'Given I am on the login page'

  Scenario: CLI rejects an invalid step type with exit 1
    Given a tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: Login\n    Given x\n'
    When I run 'fspec add-step spec/features/login.feature "Login" maybe "whatever"' in that tempdir
    Then the process exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Invalid step type: "maybe"'

  Scenario: CLI rejects an unknown scenario with exit 1
    Given a tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: Login\n    Given x\n'
    When I run 'fspec add-step spec/features/login.feature "Nope" given "x"' in that tempdir
    Then the process exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Scenario not found: "Nope"'

  Scenario: CLI help output matches captured TypeScript fixture byte-for-byte
    Given the standalone fspec Rust binary is built
    When I run 'fspec add-step --help'
    Then the process exits with code 0
    And stdout matches the captured fixture at codelet/fspec/tests/fixtures/help/add-step.txt

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: Login\n    Given x\n'
    When I dispatch add-step through fspec_core::dispatch::dispatch_command with feature='spec/features/login.feature', scenario='Login', type='when' and text='I act'
    Then the dispatcher's DispatchResult.data parses to a structure whose success is true
    And the CLI bridge module codelet/fspec/src/add_step.rs contains NO inline gherkin parsing, placeholder, or insertion logic
    And the bridge module's only computation is JSON arg marshalling and CWD resolution
