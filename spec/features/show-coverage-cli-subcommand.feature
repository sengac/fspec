@done
@coverage
@cli
@RPC-300
Feature: show-coverage CLI subcommand on the standalone fspec Rust binary

  """
  The CLI bridge module codelet/fspec/src/show_coverage.rs marshals argv into JSON
  args and delegates to codelet_fspec_core::commands::show_coverage::run — the
  same function the LLM-facing dispatcher invokes.

  Exit-code contract: 0 on success (rendered report to stdout), 1 on any failure
  (Error: <message> to stderr).

  The clap subcommand exposes the positional [feature-name] argument plus an
  optional --format <text|json> flag. The TS show-coverage-help.ts advertises both
  --format and --output; the --output flag is acknowledged in the help fixture
  but NOT wired in the TS source (no fs.writeFile call) and is correspondingly
  ignored by the Rust bridge.

  No project_root override: the CLI bridge uses env::current_dir() as the
  project root, mirroring TS process.cwd().
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want fspec show-coverage to render coverage reports for one feature or the whole project
    So that I can inspect coverage in CI/scripts without launching Node

  Scenario: Clap exposes show-coverage as a subcommand and prints help on --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec show-coverage --help` from a shell
    Then the command exits 0
    And stdout contains the substring 'show-coverage'
    And stdout contains the substring 'Display coverage report'

  Scenario: show-coverage with a missing feature exits 1 and writes the TS-parity error to stderr
    Given an empty directory containing spec/features/ but no missing.feature.coverage is set as the current working directory
    When I run `./codelet/target/release/fspec show-coverage missing` from that directory
    Then the command exits 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Coverage file not found: missing.feature.coverage'

  Scenario: CLI per-feature mode renders markdown report to stdout for a fully covered feature
    Given a temp workspace contains spec/features/auth.feature.coverage with 1 fully covered scenario whose referenced test and impl files exist on disk
    When I run `./codelet/target/release/fspec show-coverage auth` from that workspace
    Then the command exits 0
    And stdout contains the line '# Coverage Report: auth.feature'
    And stdout contains the line '**Coverage**: 100% (1/1 scenarios)'
    And stdout does NOT contain the substring '## Warnings'

  Scenario: CLI per-feature JSON mode renders 2-space-indented JSON for the requested feature
    Given a temp workspace contains spec/features/auth.feature.coverage with 1 scenario and a stats object
    When I run `./codelet/target/release/fspec show-coverage auth --format json` from that workspace
    Then the command exits 0
    And stdout parses as JSON whose root keys in declaration order are 'fileName', 'scenarios', 'stats', 'warnings'
    And stdout uses 2-space indentation

  Scenario: CLI project-wide mode aggregates and renders Project Coverage Report
    Given a temp workspace contains spec/features/a.feature.coverage with 1 fully covered scenario AND spec/features/b.feature.coverage with 1 uncovered scenario
    When I run `./codelet/target/release/fspec show-coverage` from that workspace
    Then the command exits 0
    And stdout contains the line '# Project Coverage Report'
    And stdout contains the line '**Overall Coverage**: 50% (1/2 scenarios)'

  Scenario: CLI project-wide mode exits 1 when spec/features/ is missing
    Given an empty directory is set as the current working directory
    When I run `./codelet/target/release/fspec show-coverage` from that directory
    Then the command exits 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Features directory not found: spec/features/'

  Scenario: CLI project-wide mode exits 1 when spec/features/ exists but is empty
    Given a temp workspace contains spec/features/ with no *.feature.coverage files
    When I run `./codelet/target/release/fspec show-coverage` from that workspace
    Then the command exits 1
    And stderr contains the substring 'No coverage files found in spec/features/'

  Scenario: CLI tolerates a trailing .feature on the positional feature-name
    Given a temp workspace contains spec/features/login.feature.coverage with 1 fully covered scenario
    When I run `./codelet/target/release/fspec show-coverage login.feature` from that workspace
    Then the command exits 0
    And stdout contains the line '# Coverage Report: login.feature'

  Scenario: show-coverage --help is byte-for-byte identical to TS formatCommandHelp reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec show-coverage --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/show-coverage.txt
    And stdout starts with a blank line followed by the line 'SHOW-COVERAGE'

  Scenario: Default combined TUI mode is preserved when no subcommand is provided
    Given the fspec Rust binary has show-coverage registered as a clap subcommand alongside daemon, client, status, list-work-units, list-prefixes, and list-features
    When I run `./codelet/target/release/fspec --help`
    Then the command exits 0
    And the help output lists show-coverage as an available subcommand
    And the long-about description still documents that running fspec with no subcommand enters combined TUI mode

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a temp workspace contains spec/features/auth.feature.coverage with 1 fully covered scenario
    When I dispatch show-coverage through fspec_core::dispatch::dispatch_command with featureName='auth' and format='json' against that workspace
    And I run `./codelet/target/release/fspec show-coverage auth --format json` against the same workspace
    Then both invocations produce byte-equal JSON content
    And the CLI bridge module codelet/fspec/src/show_coverage.rs contains NO inline coverage parsing, stats aggregation, or markdown rendering — its only computation is JSON arg marshalling
