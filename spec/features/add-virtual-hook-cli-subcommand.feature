@done
@cli
@rust
@RPC-195
Feature: Add Virtual Hook Cli Subcommand
  """
  CLI subcommand is wired into rust/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003 §7/§11. The action arm delegates to fspec_core::commands::add_virtual_hook::run(args_json, &cwd) so business logic is not duplicated between the LLM-facing dispatcher and the shell-facing CLI.

  The subcommand exposes three required positional arguments (`<workUnitId>`, `<event>`, `<command>`) plus two boolean flags `--blocking` and `--git-context` — mirroring the TypeScript Commander.js registration at src/commands/add-virtual-hook.ts:95-110. Both flags default to false when omitted (clap `default_value_t = false` for parity with Commander.js `.option('--blocking', '...', false)`).

  Success path prints two lines to stdout — `✓ Virtual hook added to <workUnitId>` and `  Total virtual hooks: <hookCount>` — and exits 0. Error path prints `✗ Failed to add virtual hook: <reason>` to stderr (TS chalk-red parity) and exits 1.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES (CLI subcommand subset):
  #   1. Positional args: workUnitId, event, command — all required
  #   2. Optional flags: --blocking (bool, default false), --git-context (bool, default false)
  #   3. Success → exit 0, two-line stdout output
  #   4. Domain error → exit 1, stderr begins with '✗ Failed to add virtual hook:'
  #   5. clap usage error (e.g. missing positional) → exit 2
  #   6. CLI delegates to fspec_core::commands::add_virtual_hook::run — no inlined business logic
  #   7. --help byte-identical to TS reference fixture
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec add-virtual-hook <workUnitId> <event> <command>` directly from a shell with the same positional + flag surface offered by the TypeScript Commander.js CLI
    So that I can attach a work-unit-scoped virtual hook from a script or terminal without going through the LLM tool-call dispatcher

  Scenario: Clap exposes add-virtual-hook as a subcommand and prints flag-aware --help
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec add-virtual-hook --help` from a shell
    Then the command exits 0
    And stdout contains clap-generated help describing the add-virtual-hook subcommand
    And stdout contains the positional placeholder "<workUnitId>"
    And stdout contains the positional placeholder "<event>"
    And stdout contains the positional placeholder "<command>"
    And stdout contains the substring '--blocking'
    And stdout contains the substring '--git-context'

  Scenario: CLI adds a simple hook and prints the canonical success lines
    Given spec/work-units.json contains AUTH-001 with no virtualHooks field
    When I run `./rust/target/release/fspec add-virtual-hook AUTH-001 post-implementing "npm test" --blocking`
    Then the command exits 0
    And stdout contains the substring '✓ Virtual hook added to AUTH-001'
    And stdout contains the substring '  Total virtual hooks: 1'
    And the on-disk virtualHooks array for AUTH-001 has length 1

  Scenario: CLI fails with exit 1 when the work unit does not exist
    Given spec/work-units.json contains AUTH-001
    When I run `./rust/target/release/fspec add-virtual-hook AUTH-999 post-implementing "npm test"`
    Then the command exits 1
    And stderr contains the substring '✗ Failed to add virtual hook:'
    And stderr contains the substring "Work unit 'AUTH-999' does not exist"

  Scenario: CLI with --git-context generates a shell script and stores its relative path
    Given an empty project root directory with an AUTH-001 work unit
    When I run `./rust/target/release/fspec add-virtual-hook AUTH-001 post-implementing "eslint src/" --git-context --blocking`
    Then the command exits 0
    And the file spec/hooks/.virtual/AUTH-001-eslint.sh exists
    And the file spec/hooks/.virtual/AUTH-001-eslint.sh has Unix permission bits 0o755
    And the on-disk virtualHooks[0].command equals 'spec/hooks/.virtual/AUTH-001-eslint.sh'
    And the on-disk virtualHooks[0].gitContext equals true

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher (no duplicated business logic)
    Given a project root whose spec/work-units.json contains AUTH-001 with no virtualHooks
    When I dispatch add-virtual-hook through fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' event='post-implementing' command='npm test'
    Then the dispatcher's DispatchResult.data parses to a JSON object with hookCount=1
    And the CLI bridge module rust/fspec/src/add_virtual_hook.rs contains NO inline script-generation, hook-name-derivation, or work-unit-lookup logic — its only computation is JSON arg marshalling

  Scenario: add-virtual-hook --help is byte-for-byte identical to TS formatCommandHelp reference output
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec add-virtual-hook --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/add-virtual-hook.txt
    And stdout starts with a blank line followed by 'ADD-VIRTUAL-HOOK'
    And stdout contains the section header 'COMMON PATTERNS'
