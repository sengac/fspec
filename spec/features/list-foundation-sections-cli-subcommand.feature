@done
@querying
@cli
@rust
@RPC-246
Feature: List foundation sections CLI subcommand
  """
  CLI subcommand is wired into codelet/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003 §7/§11. The intercept_ts_help() pre-clap routine in main.rs emits Commander.js-default-style help (NOT the rich CommandHelpConfig format used by list-hooks/list-attachments) because the TS reference src/commands/list-foundation-sections.ts uses bare Commander.js without a custom -help.ts file.

  Byte-parity contract: stdout of `./codelet/target/release/fspec list-foundation-sections --help` matches `node dist/index.js list-foundation-sections --help` exactly (5 lines + final newline).
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec list-foundation-sections --help` directly from a shell with the same Commander.js-default help format offered by the TypeScript CLI
    So that I can discover the --format flag and command description from a script or terminal with byte-for-byte parity

  Scenario: list-foundation-sections --help is byte-for-byte identical to TS Commander.js reference output
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec list-foundation-sections --help` piped to non-TTY
    Then the command exits 0
    And the TS reference binary `node dist/index.js list-foundation-sections --help` produces a 6-line block: Usage line, blank, description, blank, Options header, --format and -h lines
    And stdout is byte-for-byte identical to the TS reference output
    And stdout starts with the line `Usage: fspec list-foundation-sections [options]`
    And stdout contains the line `  --format <format>  Output format: text (default) or json (default: "text")`
    And stdout contains the line `  -h, --help         Display help for command`
