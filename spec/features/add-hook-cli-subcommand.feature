@done
@RPC-184
@rust
@cli
Feature: Add hook CLI subcommand

  """
  CLI subcommand wired into codelet/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003 §7/§11. The action arm delegates to fspec_core::commands::add_hook::run(args_json, &cwd).
  Positional arguments `<event>` and `<name>`. Required option `--command <path>`. Optional `--blocking` flag (default false). Optional `--timeout <seconds>` integer. No --format flag.
  Help text intercepted via codelet/fspec-core/src/help/configs/add_hook.rs (CONFIG const) — byte-exact parity with the TS `node dist/index.js add-hook --help` reference fixture.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec add-hook <event> <name> --command <path> [--blocking] [--timeout N]` directly from a shell
    So that I can register lifecycle hooks from scripts and terminals without going through the LLM tool-call dispatcher

  Scenario: add-hook --help is byte-for-byte identical to TS reference output
    Given the fspec Rust binary has been compiled
    When I run `fspec add-hook --help` with NO_COLOR=1
    Then the command exits 0
    Then stdout is byte-for-byte identical to the captured TS help fixture at codelet/fspec/tests/fixtures/help/add-hook.txt
    Then stdout contains the section header "USAGE" followed by "  fspec add-hook <event> <name> --command <path> [options]"
    Then stdout contains the section header "ARGUMENTS"
    Then stdout contains the section header "OPTIONS"
    Then stdout contains the substring '--command <path>'
    Then stdout contains the substring '--blocking'
    Then stdout contains the substring '--timeout <seconds>'

  Scenario: CLI writes zero bytes to stdout on success and exits 0
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `fspec add-hook pre-implementing lint --command spec/hooks/lint.sh`
    Then the command exits 0
    Then stdout is exactly zero bytes
    Then spec/fspec-hooks.json was created in the directory
    Then the new file contains exactly one entry under 'pre-implementing' with name='lint' and command='spec/hooks/lint.sh'

  Scenario: CLI passes --blocking and --timeout as JSON marshalling fields
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `fspec add-hook post-implementing test --command spec/hooks/test.sh --blocking --timeout 300`
    Then the command exits 0
    Then the on-disk entry under 'post-implementing' has name='test', blocking=true, and timeout=300

  Scenario: CLI is silent when invoked against a populated config
    Given spec/fspec-hooks.json contains event 'pre-implementing' with a single entry named 'lint'
    When I run `fspec add-hook pre-implementing test --command spec/hooks/test.sh`
    Then the command exits 0
    Then stdout is exactly zero bytes
    Then the on-disk 'pre-implementing' array has exactly two entries

  Scenario: CLI delegates to the same fspec_core function as the dispatcher
    Given a project root whose spec/fspec-hooks.json contains event 'post-implementing' with hooks ['lint']
    When I dispatch add-hook through fspec_core::dispatch::dispatch_command with event='post-implementing' name='test' command='t.sh' blocking=false
    Then the dispatcher returns success=true
    Then the CLI bridge module codelet/fspec/src/add_hook.rs contains NO inline parsing or write logic — its only computation is JSON arg marshalling
