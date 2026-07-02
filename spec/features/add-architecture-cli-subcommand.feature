@feature-management
@cli
@done
@RPC-167
Feature: add-architecture CLI subcommand (Rust shell front-door)
  """
  Files: codelet/fspec/src/add_architecture.rs (NEW CLI bridge); codelet/fspec/tests/cli_add_architecture.rs (NEW CLI tests); codelet/fspec/tests/fixtures/help/add-architecture.txt (captured help fixture from `node dist/index.js add-architecture --help`).
  Bridge marshals positional <feature> + <text> into JSON {feature, text} and delegates to commands::add_architecture::run. No logic in bridge — JSON marshalling and CWD resolution only.
  Exit codes: 0 on success (stdout '✓ <message>'); 1 on FspecCoreError with 'Error:' prefix to stderr.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want `fspec add-architecture <feature> <text>` to behave byte-identically to the TypeScript implementation
    So that I can add architecture documentation from a shell without depending on Node.js

  Scenario: CLI successfully adds architecture docs and prints the success line
    Given a tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I run 'fspec add-architecture spec/features/login.feature "Uses bcrypt for password hashing"' in that tempdir
    Then the process exits with code 0
    And stdout contains the substring '✓ Added architecture documentation to spec/features/login.feature'
    And the file spec/features/login.feature in the tempdir contains the line '  Uses bcrypt for password hashing'

  Scenario: CLI resolves a bare feature name by basename
    Given a tempdir with spec/features/dashboard.feature containing 'Feature: Dashboard\n  Scenario: A\n    Given x\n'
    When I run 'fspec add-architecture dashboard "Uses a worker pool"' in that tempdir
    Then the process exits with code 0
    And stdout contains the substring '✓ Added architecture documentation to dashboard'
    And the file spec/features/dashboard.feature in the tempdir contains the line '  Uses a worker pool'

  Scenario: CLI surfaces empty-text error with stderr Error prefix and exit 1
    Given a tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I run 'fspec add-architecture spec/features/login.feature ""' in that tempdir
    Then the process exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Architecture text cannot be empty'

  Scenario: CLI surfaces not-found error with exit 1
    Given a tempdir with NO spec/features/missing.feature file
    When I run 'fspec add-architecture spec/features/missing.feature "Uses bcrypt"' in that tempdir
    Then the process exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Feature file not found: spec/features/missing.feature'

  Scenario: CLI help output matches captured TypeScript fixture byte-for-byte
    Given the standalone fspec Rust binary is built
    When I run 'fspec add-architecture --help'
    Then the process exits with code 0
    And stdout matches the captured fixture at codelet/fspec/tests/fixtures/help/add-architecture.txt

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch add-architecture through fspec_core::dispatch::dispatch_command with feature='spec/features/login.feature' and text='Uses bcrypt'
    Then the dispatcher's DispatchResult.data parses to a structure whose message contains 'Added architecture documentation to spec/features/login.feature'
    And the CLI bridge module codelet/fspec/src/add_architecture.rs contains NO inline gherkin parsing or line-splice mutation logic
    And the bridge module's only computation is JSON arg marshalling and CWD resolution
