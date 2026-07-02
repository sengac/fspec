@done
@validation
@coverage-tracking
@cli
@RPC-231
Feature: fspec generate-coverage CLI subcommand
  """
  CLI bridge: codelet/fspec/src/generate_coverage.rs — clap-derived struct mirroring the TS
  Commander.js registration (src/commands/generate-coverage.ts:198-208). Surface:
  `fspec generate-coverage [--dry-run]`.
  Stdout (success): the full rendered report (counts line + system-reminder) printed verbatim; exit 0.
  Stderr (failure): 'Error: <message>'; exit 1.
  Two-front-doors invariant: the bridge marshals args into JSON {dryRun?} and forwards to
  fspec_core commands::generate_coverage::run — NO domain logic in the bridge.
  Help fixture captured from `node dist/index.js generate-coverage --help`.
  Supervisor wires: Mode::GenerateCoverage variant, intercept arm, forward! macro, configs::generate_coverage mod.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want the generate-coverage subcommand to parse the same flags as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven coverage setup script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec generate-coverage --help`
    Then the exit code is 0
    And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/generate-coverage.txt

  Scenario: CLI creates a missing sidecar and prints the success report
    Given a project root tempdir with a feature file "user-login.feature" and no coverage sidecar
    When I run `fspec generate-coverage` in that tempdir
    Then the exit code is 0
    And stdout contains the substring "Created 1"
    And the file spec/features/user-login.feature.coverage is created in that tempdir

  Scenario: CLI forwards the --dry-run flag without writing files
    Given a project root tempdir with a feature file "user-login.feature" and no coverage sidecar
    When I run `fspec generate-coverage --dry-run` in that tempdir
    Then the exit code is 0
    And stdout contains the substring "Would create 1 coverage files (DRY RUN)"
    And no coverage sidecar file is created in that tempdir

  Scenario: CLI reports a missing features directory with exit 1
    Given an empty project root tempdir with no spec/features directory
    When I run `fspec generate-coverage` in that tempdir
    Then the exit code is 1
    And stderr contains the substring "Error:"

  Scenario: Default combined TUI mode is preserved when no subcommand is provided
    Given the fspec Rust binary has generate-coverage registered as a clap subcommand alongside other ported subcommands
    When I run `fspec --help`
    Then the help output lists generate-coverage as an available subcommand

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with a feature file "user-login.feature" and no coverage sidecar
    When I dispatch generate-coverage through fspec_core::dispatch::dispatch_command against that workspace
    And I run `fspec generate-coverage` against an identical workspace
    Then both invocations create the coverage sidecar
    And the CLI bridge module codelet/fspec/src/generate_coverage.rs contains NO inline scanning or rendering logic — its only computation is JSON arg marshalling
