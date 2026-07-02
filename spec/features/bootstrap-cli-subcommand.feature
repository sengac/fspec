@done
@rust
@bootstrap
@cli
@RPC-200
Feature: Port bootstrap command to Rust (CLI subcommand)
  """
  Clap surface for the ported bootstrap command. The CLI bridge codelet/fspec/src/bootstrap.rs is a thin façade that marshals JSON args and forwards to fspec_core::commands::bootstrap::run(args_json, project_root); it contains no documentation-building or transform logic. bootstrap takes no positional arguments and no flags. Help output is byte-parity with the captured TypeScript fixture at codelet/fspec/tests/fixtures/help/bootstrap.txt. SHARED-FILE CHANGES are owned by the supervisor (see bootstrap-rust-port.feature docstring).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. bootstrap takes no arguments and no flags; invoking it always emits the complete documentation
  #   10. The CLI prints the result to stdout and exits 0; on a thrown error it prints "Error running bootstrap: <message>" and exits 1
  #   11. Both front doors (dispatcher JSON, clap CLI) converge on fspec_core::commands::bootstrap::run(args_json, project_root); the CLI bridge does JSON marshalling only
  #
  # EXAMPLES:
  #   1. Running `fspec bootstrap` from a shell prints the complete documentation to stdout and exits 0
  #   2. Running `fspec bootstrap --help` prints help byte-for-byte identical to the captured TS fixture
  #
  # ========================================
  Background: User Story
    As a fspec maintainer porting the CLI to Rust
    I want to run `bootstrap` in the Rust binary
    So that I get byte-for-byte the same complete fspec documentation with config and event-storm transforms applied as the TypeScript command

  Scenario: Clap exposes bootstrap as a subcommand and prints byte-parity help
    Given the fspec Rust binary has been compiled
    When I run `fspec bootstrap --help` piped to non-TTY
    Then the command exits 0
    Then stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/bootstrap.txt
    Then stdout starts with a blank line followed by "BOOTSTRAP"

  Scenario: CLI prints the complete documentation and exits 0
    Given a temp working directory with no fspec-config.json and no foundation.json
    When I run `fspec bootstrap` from that directory
    Then the command exits 0
    Then stdout contains the substring "# fspec Command - Kanban-Based Project Management"
    Then stdout contains the substring "LIFECYCLE HOOKS"

  Scenario: bootstrap defines no positional arguments and no flags
    Given the fspec Rust binary has been compiled
    When I run `fspec bootstrap --help` piped to non-TTY
    Then the command exits 0
    Then the help output does not advertise any --skip-help, --minimal, or --skip-sections flags

  Scenario: Default combined TUI mode is preserved when no subcommand is provided
    Given the fspec Rust binary has bootstrap registered as a clap subcommand alongside the existing subcommands
    When I run `fspec --help`
    Then the command exits 0
    Then the help output lists bootstrap as an available subcommand
