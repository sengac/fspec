@done
@validation
@coverage-tracking
@cli
@RPC-240
Feature: fspec link-coverage CLI subcommand
  """
  CLI bridge: rust/fspec/src/link_coverage.rs — clap-derived struct mirroring the TS
  Commander.js registration (src/commands/link-coverage.ts:226-250). Surface:
  `fspec link-coverage <feature-name> --scenario <name> [--test-file <path>] [--test-lines <range>]
  [--impl-file <path>] [--impl-lines <lines>] [--skip-validation] [--skip-step-validation]`.
  Stdout (success): result.message (plus removal hint and yellow warnings) printed; exit 0.
  Stderr (failure): 'Error: <message>'; exit 1.
  Two-front-doors invariant: the bridge marshals args into JSON and forwards to
  fspec_core commands::link_coverage::run — NO domain logic in the bridge.
  Help fixture captured from `node dist/index.js link-coverage --help`.
  Supervisor wires: Mode::LinkCoverage variant, intercept arm, forward! macro, configs::link_coverage mod.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want the link-coverage subcommand to parse the same flags as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven coverage-linking workflow keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec link-coverage --help`
    Then the exit code is 0
    And the stdout matches the canonical help fixture at rust/fspec/tests/fixtures/help/link-coverage.txt
    And stdout contains the substring "--scenario"

  Scenario: CLI links a test mapping and prints the success message
    Given a project root tempdir has a feature file and coverage sidecar for scenario "Login" and a test file with matching @step comments
    When I run `fspec link-coverage user-login --scenario Login --test-file src/auth.test.ts --test-lines 45-62` in that tempdir
    Then the exit code is 0
    And stdout contains the substring "Linked test mapping"

  Scenario: CLI without a valid flag combination exits 1
    Given a project root tempdir has a coverage sidecar with scenario "Login"
    When I run `fspec link-coverage user-login --scenario Login --impl-file src/login.ts` in that tempdir
    Then the exit code is 1
    And stderr contains the substring "Error:"

  Scenario: CLI exits 1 when the coverage sidecar is missing
    Given an empty project root tempdir with no coverage sidecar
    When I run `fspec link-coverage user-login --scenario Login --test-file src/auth.test.ts --test-lines 1-2 --skip-validation` in that tempdir
    Then the exit code is 1
    And stderr contains the substring "Error:"

  Scenario: Default combined TUI mode is preserved when no subcommand is provided
    Given the fspec Rust binary has link-coverage registered as a clap subcommand alongside other ported subcommands
    When I run `fspec --help`
    Then the help output lists link-coverage as an available subcommand

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir has a feature file and coverage sidecar for scenario "Login" and a matching test file
    When I dispatch link-coverage through fspec_core::dispatch::dispatch_command against that workspace
    And I run `fspec link-coverage user-login --scenario Login --test-file src/auth.test.ts --test-lines 45-62` against an identical workspace
    Then both invocations report success
    And the CLI bridge module rust/fspec/src/link_coverage.rs contains NO inline mutation, validation, or rendering logic — its only computation is JSON arg marshalling
