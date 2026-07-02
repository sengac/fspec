@feature-management
@cli
@done
@RPC-171
Feature: add-background CLI subcommand (Rust shell front-door)
  """
  Files: codelet/fspec/src/add_background.rs (NEW CLI bridge); codelet/fspec/tests/cli_add_background.rs (NEW CLI tests); codelet/fspec/tests/fixtures/help/add-background.txt (captured help fixture from `node dist/index.js add-background --help`).
  Bridge marshals positional <feature> + <text> into JSON {feature, text} and delegates to commands::add_background::run. No logic in bridge — JSON marshalling and CWD resolution only.
  Exit codes: 0 on success (stdout '✓ <message>'); 1 on FspecCoreError with 'Error:' prefix to stderr.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want `fspec add-background <feature> <text>` to behave byte-identically to the TypeScript implementation
    So that I can add a Background section from a shell without depending on Node.js

  Scenario: CLI successfully adds a Background and prints the success line
    Given a tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I run 'fspec add-background spec/features/login.feature "As a user\nI want to log in\nSo that I access my account"' in that tempdir
    Then the process exits with code 0
    And stdout contains the substring '✓ Added background to spec/features/login.feature'
    And the file spec/features/login.feature in the tempdir contains the line '  Background: User Story'

  Scenario: CLI resolves a bare feature name by basename
    Given a tempdir with spec/features/dashboard.feature containing 'Feature: Dashboard\n  Scenario: A\n    Given x\n'
    When I run 'fspec add-background dashboard "As a user\nI want a dashboard\nSo that I see overview"' in that tempdir
    Then the process exits with code 0
    And stdout contains the substring '✓ Added background to dashboard'
    And the file spec/features/dashboard.feature in the tempdir contains the line '  Background: User Story'

  Scenario: CLI surfaces empty-text error with stderr Error prefix and exit 1
    Given a tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I run 'fspec add-background spec/features/login.feature ""' in that tempdir
    Then the process exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Background text cannot be empty'

  Scenario: CLI surfaces not-found error with exit 1
    Given a tempdir with NO spec/features/missing.feature file
    When I run 'fspec add-background spec/features/missing.feature "As a user"' in that tempdir
    Then the process exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Feature file not found: spec/features/missing.feature'

  Scenario: CLI help output matches captured TypeScript fixture byte-for-byte
    Given the standalone fspec Rust binary is built
    When I run 'fspec add-background --help'
    Then the process exits with code 0
    And stdout matches the captured fixture at codelet/fspec/tests/fixtures/help/add-background.txt

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch add-background through fspec_core::dispatch::dispatch_command with feature='spec/features/login.feature' and text='As a user'
    Then the dispatcher's DispatchResult.data parses to a structure whose message contains 'Added background to spec/features/login.feature'
    And the CLI bridge module codelet/fspec/src/add_background.rs contains NO inline gherkin parsing or line-splice mutation logic
    And the bridge module's only computation is JSON arg marshalling and CWD resolution
