@validation
@cli
@rust
@wip
@RPC-320
Feature: Validate Gherkin CLI subcommand
  """
  CLI subcommand is wired into codelet/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003 §7/§11. The action arm delegates to fspec_core::commands::validate::run(args_json, &cwd) so business logic is not duplicated between the LLM-facing dispatcher and the shell-facing CLI.

  The subcommand mirrors the TypeScript Commander.js registration at src/commands/validate.ts:256-265: an optional positional [file] argument and a -v/--verbose boolean flag (default false). No other flags.

  The CLI bridge maps the core result to process exit codes: 0 (all valid), 1 (one or more invalid), 2 (no feature files found or unexpected error). The display block is written to stdout; the 'No feature files found' / unexpected-error message is written to stderr (parity with validateCommand at src/commands/validate.ts:20-78).

  RPC-329: the embedded raw parser-error TEXT for malformed input diverges from @cucumber/gherkin and is tracked separately. Scenarios assert structural facts (valid/invalid markers, exit codes, Line N presence, Suggestion presence, content-heuristic messages) NOT the exact raw parser message.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec validate [file]` directly from a shell with the same [file] + --verbose surface offered by the TypeScript Commander.js CLI
    So that I can validate feature files from a script or terminal without going through the LLM tool-call dispatcher

  Scenario: CLI validates a single valid file and exits 0
    Given spec/features/login.feature is a syntactically valid feature file in the working directory
    When I run `./codelet/target/release/fspec validate spec/features/login.feature` from that directory
    Then the command exits 0
    Then stdout contains the substring '✓ spec/features/login.feature is valid'

  Scenario: CLI exits 1 against a syntactically broken file
    Given spec/features/broken.feature contains broken Gherkin syntax in the working directory
    When I run `./codelet/target/release/fspec validate spec/features/broken.feature` from that directory
    Then the command exits with code 1
    Then stdout contains the substring 'has syntax errors:'
    Then stdout contains the substring 'Line '

  Scenario: CLI exits 2 when no feature files are found
    Given spec/features/ exists but contains zero .feature files in the working directory
    When I run `./codelet/target/release/fspec validate` from that directory
    Then the command exits with code 2
    Then stderr contains the substring 'No feature files found in spec/features/'

  Scenario: Clap exposes validate as a subcommand and prints help byte-identical to the TS reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec validate --help` from a shell
    Then the command exits 0
    Then stdout is byte-for-byte identical to the captured TS formatCommandHelp reference fixture
    Then stdout contains the substring '--verbose'
    Then stdout contains the substring 'file'
