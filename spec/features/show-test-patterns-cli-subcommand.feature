@done
@querying
@cli
@RPC-307
Feature: show-test-patterns CLI subcommand on the standalone fspec Rust binary
  """
  CLI subcommand wired into rust/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003 §7/§11. The action arm delegates to fspec_core::commands::show_test_patterns::run(args_json) so business logic is not duplicated between the LLM-facing dispatcher and the shell-facing CLI.

  show-test-patterns exposes one REQUIRED --tag flag plus two optional flags: --include-coverage and --json. No positional arguments.

  Exit-code contract: 0 on success; 1 when fspec_core::commands::show_test_patterns::run returns FspecCoreError (missing tag, missing/malformed work-units.json) or the dispatcher envelope returns success=false. Error messages go to stderr prefixed with 'Error:' (TS uses output.error('✗ Analysis failed:', ...) — Rust preserves the 'Error:' prefix for parity with other ports).
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec show-test-patterns --tag=@cli` from a shell with the same flags offered by the TypeScript Commander.js CLI
    So that I can audit testing patterns from a script without going through the LLM tool-call dispatcher

  Scenario: Clap exposes show-test-patterns as a subcommand and prints flag help
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec show-test-patterns --help` from a shell
    Then the command exits 0
    And stdout contains the substring 'show-test-patterns'
    And stdout contains the substring '--tag'

  Scenario: CLI without --tag flag exits 1
    Given an empty directory with no spec/ subdirectory is the current working directory
    When I run `./rust/target/release/fspec show-test-patterns` from that directory
    Then the command exits with a non-zero status

  Scenario: CLI default output prints the green summary line
    Given a temp workspace contains spec/work-units.json with two work units tagged @cli
    When I run `./rust/target/release/fspec show-test-patterns --tag @cli` from that workspace
    Then the command exits 0
    And stdout contains the substring 'Analyzed testing patterns for 2 work units tagged with @cli'

  Scenario: CLI --json prints 2-space JSON envelope to stdout
    Given a temp workspace contains spec/work-units.json with one work unit tagged @cli
    When I run `./rust/target/release/fspec show-test-patterns --tag @cli --json` from that workspace
    Then the command exits 0
    And stdout parses as JSON with workUnits, testFiles, patterns, and format fields
    And the JSON.format field equals 'json'

  Scenario: CLI --include-coverage includes deduplicated test file paths
    Given a temp workspace contains spec/work-units.json with one work unit tagged @cli and two .feature.coverage files referencing three unique testMappings file paths
    When I run `./rust/target/release/fspec show-test-patterns --tag @cli --include-coverage --json` from that workspace
    Then the command exits 0
    And the JSON.testFiles array has 3 unique elements

  Scenario: CLI exits 1 when work-units.json is missing
    Given an empty directory with no spec/ subdirectory is the current working directory
    When I run `./rust/target/release/fspec show-test-patterns --tag @cli` from that directory
    Then the command exits with a non-zero status
    And stderr contains the substring 'Error:'

  Scenario: show-test-patterns --help is byte-for-byte identical to the TS formatCommandHelp reference
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec show-test-patterns --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/show-test-patterns.txt
    And stdout starts with a blank line followed by 'SHOW-TEST-PATTERNS'

  Scenario: Default combined TUI mode is preserved when no subcommand is provided
    Given the fspec Rust binary has show-test-patterns registered as a clap subcommand alongside daemon, client, status, and other ported subcommands
    When I run `./rust/target/release/fspec --help`
    Then the help output lists show-test-patterns as an available subcommand
    And the long-about description still documents that running fspec with no subcommand enters combined TUI mode

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a temp workspace contains spec/work-units.json with two work units tagged @cli
    When I dispatch show-test-patterns through fspec_core::dispatch::dispatch_command with tag='@cli' against that workspace
    And I run `./rust/target/release/fspec show-test-patterns --tag @cli --json` against the same workspace
    Then both invocations produce a JSON envelope with workUnits.length=2
    And the CLI bridge module rust/fspec/src/show_test_patterns.rs contains NO inline filtering or rendering logic — its only computation is JSON arg marshalling
