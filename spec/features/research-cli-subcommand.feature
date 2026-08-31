@done
@rust
@research
@cli
@RPC-286
Feature: Research CLI subcommand
  """
  CLI subcommand wired into rust/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003
  §7/§11. The action arm delegates to codelet_fspec_core::commands::research::run(args_json, &cwd) so the
  list/discovery business logic is not duplicated between the LLM-facing dispatcher and the shell-facing CLI.

  TS Commander.js registration (src/commands/research.ts:276-407) declares:
  .command('research [args...]') .option('--tool <name>') .option('--work-unit <id>') .allowUnknownOption()
  The clap variant therefore exposes --tool, --work-unit and a trailing variadic [args...] capture, and must
  tolerate unknown flags so tool-specific args forward through untouched.

  ⚠️ SCOPE FLAG: actual EXECUTE-mode tool invocation (network/NAPI/dynamic-JS plugins/script spawning) is
  DEFERRED pending a supervisor scope decision — see research-rust-port.feature docstring. This feature
  specifies only the deterministic shell surface: LIST output formatting, --help byte-parity, and the
  unknown-tool error path (exit 1). Help text is served by intercept_ts_help from the byte-exact fixture, not
  clap's auto-generated block.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec research` directly from a shell with the same surface offered by the TypeScript
    Commander.js CLI
    So that I can enumerate available research tools and their configuration status from a terminal or script
    without going through the LLM tool-call dispatcher

  Scenario: CLI lists available research tools with the Commander header
    Given an empty project root tempdir
    When I run `fspec research` in that directory
    Then the command exits with code 0
    And stdout contains "Available Research Tools:"
    And stdout contains "jira"
    And stdout contains "perplexity"
    And stdout contains "stakeholder"
    And stdout does not contain "--tool=ast"

  Scenario: CLI tool listing includes per-tool usage guidance
    Given an empty project root tempdir
    When I run `fspec research` in that directory
    Then the command exits with code 0
    And stdout contains "Usage: fspec research --tool=stakeholder <args>"

  Scenario: CLI fails with a not-found error for an unknown tool
    Given an empty project root tempdir
    When I run `fspec research --tool does-not-exist` in that directory
    Then the command exits with code 1
    And stderr contains "Research tool not found: does-not-exist"

  Scenario: research --help matches the TS formatCommandHelp reference
    Given the standalone fspec binary
    When I run `fspec research --help`
    Then the command exits with code 0
    And stdout is byte-for-byte identical to tests/fixtures/help/research.txt
