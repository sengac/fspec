@done
@querying
@cli
@RPC-207
Feature: compare-implementations CLI subcommand on the standalone fspec Rust binary
  """
  CLI subcommand wired into codelet/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003 §7/§11. The action arm delegates to fspec_core::commands::compare_implementations::run(args_json) so business logic is not duplicated between the LLM-facing dispatcher and the shell-facing CLI.

  compare-implementations exposes one REQUIRED --tag flag plus two optional flags: --show-coverage and --json. No positional arguments.

  Exit-code contract: 0 on success; 1 when fspec_core::commands::compare_implementations::run returns FspecCoreError (missing/malformed work-units.json). The error line is '✗ Comparison failed:' on stderr (parity with the TS output.error('✗ Comparison failed:', ...)). Default (no --json) prints the green '✓ Compared N work units tagged with <tag>' summary; --json prints the 2-space JSON envelope. Help has a rich -help.ts → a normal CommandHelpConfig module renders byte-for-byte parity.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec compare-implementations --tag=@cli` from a shell with the same flags offered by the TypeScript Commander.js CLI
    So that I can audit implementation consistency from a script without going through the LLM tool-call dispatcher

  Scenario: CLI default output prints the green summary line
    Given a temp workspace contains spec/work-units.json with one work unit tagged @cli
    When I run `./codelet/target/release/fspec compare-implementations --tag @cli` from that workspace
    Then the command exits 0
    And stdout contains the substring '✓ Compared 1 work units tagged with @cli'

  Scenario: CLI --json prints 2-space JSON envelope to stdout
    Given a temp workspace contains spec/work-units.json with two work units tagged @cli
    When I run `./codelet/target/release/fspec compare-implementations --tag @cli --json` from that workspace
    Then the command exits 0
    And stdout parses as JSON with workUnits, comparison, namingConventionDifferences, and coverage fields
    And the JSON.workUnits array has 2 elements

  Scenario: CLI --show-coverage includes deduplicated coverage file paths
    Given a temp workspace contains spec/work-units.json with one work unit tagged @cli and one .feature.coverage file referencing one test file and one impl file
    When I run `./codelet/target/release/fspec compare-implementations --tag @cli --show-coverage --json` from that workspace
    Then the command exits 0
    And the JSON.coverage array has one entry
    And the JSON coverage[0].testFiles array has one element

  Scenario: CLI exits 1 when work-units.json is missing
    Given an empty directory with no spec/ subdirectory is the current working directory
    When I run `./codelet/target/release/fspec compare-implementations --tag @cli` from that directory
    Then the command exits with a non-zero status
    And stderr contains the substring '✗ Comparison failed:'

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/work-units.json containing two work units tagged @cli
    When I run compare-implementations once via the dispatcher and once via the CLI --json on identical inputs
    Then both front doors produce the same JSON envelope

  Scenario: compare-implementations --help is byte-for-byte identical to the TS formatCommandHelp reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec compare-implementations --help` piped to non-TTY
    Then the command exits 0
    And stdout matches the captured fixture at codelet/fspec/tests/fixtures/help/compare-implementations.txt
