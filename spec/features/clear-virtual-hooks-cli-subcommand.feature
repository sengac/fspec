@done
@querying
@cli
@rust
@RPC-205
Feature: clear-virtual-hooks CLI subcommand
  """
  Shell-facing surface for the Rust port of `fspec clear-virtual-hooks <workUnitId>`. Lives at codelet/fspec/src/clear_virtual_hooks.rs as the standard two-front-doors CLI bridge — it owns clap parsing, marshals camelCase JSON into the shared `clear_virtual_hooks::run` core function (defined in codelet/fspec-core/src/commands/clear_virtual_hooks.rs and proven by spec/features/clear-virtual-hooks-rust-port.feature), prints the rendered text response to stdout, and surfaces InvalidArgs as a single `Error: <msg>` line to stderr followed by exit code 1.

  The CLI signature mirrors the TS Commander.js definition exactly: one required positional `<workUnitId>` and no flags. Help is intercepted in `codelet/fspec/src/main.rs::intercept_ts_help` BEFORE clap parses argv — the intercept arm calls `format_command_help(&configs::clear_virtual_hooks::CONFIG)` which produces output byte-for-byte identical to `node dist/index.js clear-virtual-hooks --help`. The byte-parity contract is enforced by `codelet/fspec/tests/fixtures/help/clear-virtual-hooks.txt`.
  """

  Background: User Story
    As a shell user of the standalone fspec Rust binary
    I want to run `fspec clear-virtual-hooks <workUnitId>` and see chalk-style success or failure feedback
    So that I can wipe a work unit's virtual hooks without going through the LLM dispatcher

  Scenario: CLI prints success message when clearing hooks
    Given a project root whose spec/work-units.json contains AUTH-001 with two virtualHooks
    When I run `./codelet/target/release/fspec clear-virtual-hooks AUTH-001` in that project root
    Then the command exits 0
    And stdout contains the substring "✓ Cleared 2 virtual hook(s) from AUTH-001"

  Scenario: CLI succeeds with clearedCount=0 when the unit has no hooks
    Given a project root whose spec/work-units.json contains AUTH-001 with no virtualHooks field
    When I run `./codelet/target/release/fspec clear-virtual-hooks AUTH-001` in that project root
    Then the command exits 0
    And stdout contains the substring "✓ Cleared 0 virtual hook(s) from AUTH-001"

  Scenario: CLI exits 1 with chalk failure prefix when the work unit does not exist
    Given a project root whose spec/work-units.json contains AUTH-001 with no virtualHooks
    When I run `./codelet/target/release/fspec clear-virtual-hooks AUTH-999` in that project root
    Then the command exits 1
    And stderr contains the substring "Work unit 'AUTH-999' does not exist"

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher (no duplicated business logic)
    Given a project root whose spec/work-units.json contains AUTH-001 with one virtualHook 'lint'
    When I dispatch clear-virtual-hooks through fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001'
    Then the dispatcher's DispatchResult.data parses to a JSON object with clearedCount=1
    And the CLI bridge module codelet/fspec/src/clear_virtual_hooks.rs contains NO inline rendering, file IO, or work-unit-lookup logic — its only computation is JSON arg marshalling

  Scenario: clear-virtual-hooks --help is byte-for-byte identical to TS formatCommandHelp reference output
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec clear-virtual-hooks --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/clear-virtual-hooks.txt
    And stdout starts with a blank line followed by 'CLEAR-VIRTUAL-HOOKS'
    And stdout contains the section header 'COMMON PATTERNS'
