@formatting
@done
@formatter
@cli
@RPC-230
Feature: format CLI subcommand on the standalone fspec Rust binary
  """
  CLI subcommand wired into codelet/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003 §7/§11. The action arm delegates to fspec_core::commands::format::run(args_json) so business logic is not duplicated between the LLM-facing dispatcher and the shell-facing CLI.

  format exposes one optional positional [file] argument and no flags. The core returns the envelope {formattedCount} and the bridge owns all rendering: formattedCount===0 prints 'No feature files found to format' (exit 0); a file argument prints '✓ Formatted <file>'; otherwise green '✓ Formatted N feature files'; a thrown error (e.g. a missing single file) prints 'Error: <message>' to stderr and exits 1.

  format has a rich -help.ts → a normal CommandHelpConfig module renders byte-for-byte parity with node dist/index.js format --help.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec format` or `fspec format spec/features/login.feature` from a shell with the same argument offered by the TypeScript Commander.js CLI
    So that I can normalise feature-file formatting from a script without going through the LLM tool-call dispatcher

  Scenario: CLI formats all feature files and prints the green summary
    Given a temp workspace contains two well-formed feature files under spec/features
    When I run `./codelet/target/release/fspec format` from that workspace
    Then the command exits 0
    And stdout contains the substring '✓ Formatted 2 feature files'

  Scenario: CLI formats a single supplied file
    Given a temp workspace contains spec/features/login.feature
    When I run `./codelet/target/release/fspec format spec/features/login.feature` from that workspace
    Then the command exits 0
    And stdout contains the substring '✓ Formatted spec/features/login.feature'

  Scenario: CLI prints a no-files message when none are found
    Given an empty directory with no spec/features feature files is the current working directory
    When I run `./codelet/target/release/fspec format` from that directory
    Then the command exits 0
    And stdout contains the substring 'No feature files found to format'

  Scenario: CLI errors when a supplied file is missing
    Given a temp workspace with no spec/features/missing.feature file
    When I run `./codelet/target/release/fspec format spec/features/missing.feature` from that workspace
    Then the command exits with a non-zero status
    And stderr contains the substring 'Error: File not found: spec/features/missing.feature'

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with two well-formed feature files under spec/features
    When I run format once via the dispatcher and once via the CLI on identical inputs
    Then both front doors rewrite the files to identical content

  Scenario: format --help is byte-for-byte identical to the TS formatCommandHelp reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec format --help` piped to non-TTY
    Then the command exits 0
    And stdout matches the captured fixture at codelet/fspec/tests/fixtures/help/format.txt
