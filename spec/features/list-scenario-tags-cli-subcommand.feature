@done
@querying
@cli
@rust
@RPC-249
Feature: List scenario tags CLI subcommand
  """
  CLI subcommand is wired into rust/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003 §7/§11. The action arm delegates to fspec_core::commands::list_scenario_tags::run(args_json, &cwd) so business logic is not duplicated between the LLM-facing dispatcher and the shell-facing CLI.

  The subcommand exposes two required positional arguments `<file>` and `<scenario>` plus the boolean `--show-categories` flag — mirroring the TypeScript Commander.js registration at src/commands/list-scenario-tags.ts:182-200. No other flags are surfaced; --format, --workspace, etc. are intentionally out of scope for RPC-249.

  Per the RPC-249 architecture-note divergence, all recoverable errors (Scenario not found, Invalid Gherkin syntax, File not found) are surfaced through the FspecCoreError envelope whose Display impl prefixes the canonical reason with `Invalid args for fspec command list-scenario-tags: <reason>`. CLI tests assert on the canonical reason substring rather than the wrapper prefix.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec list-scenario-tags <file> <scenario>` directly from a shell with the same positional-argument surface offered by the TypeScript Commander.js CLI
    So that I can audit the tag set of a single scenario from a script or terminal without going through the LLM tool-call dispatcher

  Scenario: Clap exposes list-scenario-tags as a subcommand and prints flag-aware --help
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec list-scenario-tags --help` from a shell
    Then the command exits 0
    Then stdout contains clap-generated help describing the list-scenario-tags subcommand
    Then stdout contains the positional placeholder "<FILE>"
    Then stdout contains the positional placeholder "<SCENARIO>"
    Then stdout contains the substring '--show-categories'
    Then stdout does NOT contain the substring '--workspace'

  Scenario: CLI exits 2 when required positional arguments are missing
    Given an empty directory is set as the current working directory
    When I run `./rust/target/release/fspec list-scenario-tags` (no positionals) from that directory
    Then the command exits with code 2
    Then stderr names the missing required argument

  Scenario: CLI prints tag list and exits 0 when scenario has tags
    Given the working directory contains spec/features/user-login.feature with a Scenario 'Login with valid credentials' tagged '@smoke @critical'
    When I run `./rust/target/release/fspec list-scenario-tags spec/features/user-login.feature "Login with valid credentials"`
    Then the command exits 0
    Then stdout contains the substring "Tags on scenario 'Login with valid credentials':"
    Then stdout contains the exact line "  @smoke"
    Then stdout contains the exact line "  @critical"

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher (no duplicated business logic)
    Given a project root whose spec/features/user-login.feature has a Scenario 'Login with valid credentials' tagged '@smoke'
    When I dispatch list-scenario-tags through fspec_core::dispatch::dispatch_command with file='spec/features/user-login.feature', scenario='Login with valid credentials', and format='json'
    Then the dispatcher's DispatchResult.data parses to a JSON object with tags array of length 1
    Then the CLI bridge module rust/fspec/src/list_scenario_tags.rs contains NO inline Gherkin parsing, tag accumulation, or category lookup logic — its only computation is JSON arg marshalling

  Scenario: list-scenario-tags --help is byte-for-byte identical to TS minimal formatCommandHelp reference output
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec list-scenario-tags --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the TS reference fixture at rust/fspec/tests/fixtures/help/list-scenario-tags.txt
    And stdout starts with a blank line followed by 'LIST-SCENARIO-TAGS'
    And stdout contains '<file> (required)' and '<scenario> (required)' lines
    And stdout does NOT contain 'WHEN TO USE' or 'NOTES' section headers
