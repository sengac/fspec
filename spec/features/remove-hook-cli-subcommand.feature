@done
@RPC-275
@rust
@cli
Feature: Remove hook CLI subcommand
  """
  CLI subcommand wired into codelet/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003 §7/§11. The action arm delegates to fspec_core::commands::remove_hook::run(args_json, &cwd).
  Positional arguments `<event>` and `<name>` (both required). NO options.
  Help text intercepted via codelet/fspec-core/src/help/configs/remove_hook.rs (CONFIG const) — byte-exact parity with the TS `node dist/index.js remove-hook --help` reference fixture.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec remove-hook <event> <name>` directly from a shell
    So that I can deregister lifecycle hooks from scripts and terminals without going through the LLM tool-call dispatcher

  Scenario: remove-hook --help is byte-for-byte identical to TS reference output
    Given the fspec Rust binary has been compiled
    When I run `fspec remove-hook --help` with NO_COLOR=1
    Then the command exits 0
    Then stdout is byte-for-byte identical to the captured TS help fixture at codelet/fspec/tests/fixtures/help/remove-hook.txt
    Then stdout contains the section header "USAGE" followed by "  fspec remove-hook <event> <name>"
    Then stdout contains the section header "ARGUMENTS"
    Then stdout contains the section header "OPTIONS" followed by "  No options available"
    Then stdout does NOT contain the substring '--command'
    Then stdout does NOT contain the substring '--blocking'
    Then stdout does NOT contain the substring '--timeout'

  Scenario: CLI writes zero bytes to stdout on success and exits 0
    Given spec/fspec-hooks.json contains event 'post-implementing' with entries named 'lint' and 'test'
    When I run `fspec remove-hook post-implementing lint`
    Then the command exits 0
    Then stdout is exactly zero bytes
    Then the on-disk 'post-implementing' array has exactly one entry named 'test'

  Scenario: CLI exits non-zero with error when spec/fspec-hooks.json is missing
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `fspec remove-hook pre-implementing lint`
    Then the command exits 1
    Then stderr starts with 'Error:'
    Then spec/fspec-hooks.json was NOT created in the directory

  Scenario: CLI exits non-zero with error when spec/fspec-hooks.json is invalid JSON
    Given spec/fspec-hooks.json exists in the working directory but contains invalid JSON syntax
    When I run `fspec remove-hook pre-implementing lint`
    Then the command exits 1
    Then stderr starts with 'Error:'
    Then the raw bytes of spec/fspec-hooks.json are unchanged

  Scenario: CLI is silent when called with a no-op (missing key/name)
    Given spec/fspec-hooks.json contains event 'pre-implementing' with a single entry named 'lint'
    When I run `fspec remove-hook pre-implementing nonexistent`
    Then the command exits 0
    Then stdout is exactly zero bytes
    Then the on-disk 'pre-implementing' array is unchanged (one entry named 'lint')

  Scenario: CLI delegates to the same fspec_core function as the dispatcher
    Given a project root whose spec/fspec-hooks.json contains event 'post-implementing' with hooks ['lint','test']
    When I dispatch remove-hook through fspec_core::dispatch::dispatch_command with event='post-implementing' name='lint'
    Then the dispatcher returns success=true
    Then the CLI bridge module codelet/fspec/src/remove_hook.rs contains NO inline parsing or write logic — its only computation is JSON arg marshalling
