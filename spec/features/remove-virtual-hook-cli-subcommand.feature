@done
@cli
@rust
@RPC-283
Feature: Remove Virtual Hook Cli Subcommand

  """
  CLI subcommand is wired into codelet/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003 §7/§11. The action arm delegates to fspec_core::commands::remove_virtual_hook::run(args_json, &cwd) so business logic is not duplicated between the LLM-facing dispatcher and the shell-facing CLI.

  The subcommand exposes two required positional arguments (`<workUnitId>`, `<hookName>`) and NO flags — mirroring the TypeScript Commander.js registration at src/commands/remove-virtual-hook.ts:80-86 which declares the two positionals with no `.option(...)` calls.

  Success path prints two lines to stdout — `✓ Removed virtual hook '<hookName>' from <workUnitId>` and `  Remaining virtual hooks: <n>` — and exits 0. Error path prints `✗ Failed to remove virtual hook: <reason>` to stderr (TS chalk-red parity) and exits 1.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES (CLI subcommand subset):
  #   1. Positional args: workUnitId, hookName — both required
  #   2. No flags
  #   3. Success → exit 0, two-line stdout output
  #   4. Domain error → exit 1, stderr begins with '✗ Failed to remove virtual hook:'
  #   5. clap usage error → exit 2
  #   6. CLI delegates to fspec_core::commands::remove_virtual_hook::run — no inlined business logic
  #   7. --help byte-identical to TS reference fixture
  #
  # ========================================

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec remove-virtual-hook <workUnitId> <hookName>` directly from a shell with the same positional-argument surface offered by the TypeScript Commander.js CLI
    So that I can detach a named virtual hook from a script or terminal without going through the LLM tool-call dispatcher

  Scenario: Clap exposes remove-virtual-hook as a subcommand and prints flag-aware --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec remove-virtual-hook --help` from a shell
    Then the command exits 0
    And stdout contains clap-generated help describing the remove-virtual-hook subcommand
    And stdout contains the positional placeholder "<workUnitId>"
    And stdout contains the positional placeholder "<hookName>"
    And stdout does NOT contain the substring '--blocking'
    And stdout does NOT contain the substring '--git-context'
    # Note: remove-virtual-hook accepts zero options; the help OPTIONS section
    # advertises "No options available". The substrings --blocking and --git-context
    # may legitimately appear in COMMON PATTERNS example commands referencing
    # add-virtual-hook, so we instead assert the OPTIONS section explicitly says
    # "No options available".
    And stdout contains the substring 'No options available'

  Scenario: CLI removes an existing hook and prints the canonical success lines
    Given spec/work-units.json contains AUTH-001 with virtualHooks=[{name:'eslint',event:'post-implementing',command:'eslint .',blocking:true}]
    When I run `./codelet/target/release/fspec remove-virtual-hook AUTH-001 eslint`
    Then the command exits 0
    And stdout contains the substring "✓ Removed virtual hook 'eslint' from AUTH-001"
    And stdout contains the substring '  Remaining virtual hooks: 0'
    And the on-disk virtualHooks array for AUTH-001 has length 0

  Scenario: CLI fails with exit 1 when the work unit has no virtual hooks
    Given spec/work-units.json contains AUTH-001 with no virtualHooks field
    When I run `./codelet/target/release/fspec remove-virtual-hook AUTH-001 eslint`
    Then the command exits 1
    And stderr contains the substring '✗ Failed to remove virtual hook:'
    And stderr contains the substring 'No virtual hooks configured for AUTH-001'

  Scenario: CLI fails with exit 1 when the work unit does not exist
    Given spec/work-units.json contains AUTH-001
    When I run `./codelet/target/release/fspec remove-virtual-hook AUTH-999 eslint`
    Then the command exits 1
    And stderr contains the substring '✗ Failed to remove virtual hook:'
    And stderr contains the substring "Work unit 'AUTH-999' does not exist"

  Scenario: CLI fails with exit 1 when the named hook is not found
    Given spec/work-units.json contains AUTH-001 with virtualHooks=[{name:'eslint',event:'post-implementing',command:'eslint .',blocking:true}]
    When I run `./codelet/target/release/fspec remove-virtual-hook AUTH-001 missing`
    Then the command exits 1
    And stderr contains the substring "Virtual hook 'missing' not found in AUTH-001"

  Scenario: CLI deletes the associated script file on removal
    Given spec/work-units.json contains AUTH-001 with virtualHooks=[{name:'eslint',event:'post-implementing',command:'spec/hooks/.virtual/AUTH-001-eslint.sh',blocking:true,gitContext:true}]
    And the file spec/hooks/.virtual/AUTH-001-eslint.sh exists on disk
    When I run `./codelet/target/release/fspec remove-virtual-hook AUTH-001 eslint`
    Then the command exits 0
    And the file spec/hooks/.virtual/AUTH-001-eslint.sh no longer exists

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher (no duplicated business logic)
    Given a project root whose spec/work-units.json contains AUTH-001 with virtualHooks=[{name:'eslint',event:'post-implementing',command:'eslint .',blocking:true}]
    When I dispatch remove-virtual-hook through fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' hookName='eslint'
    Then the dispatcher's DispatchResult.data parses to a JSON object with remainingCount=0
    And the CLI bridge module codelet/fspec/src/remove_virtual_hook.rs contains NO inline script-removal, retain, or work-unit-lookup logic — its only computation is JSON arg marshalling

  Scenario: remove-virtual-hook --help is byte-for-byte identical to TS formatCommandHelp reference output
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec remove-virtual-hook --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/remove-virtual-hook.txt
    And stdout starts with a blank line followed by 'REMOVE-VIRTUAL-HOOK'
    And stdout contains the section header 'COMMON PATTERNS'
