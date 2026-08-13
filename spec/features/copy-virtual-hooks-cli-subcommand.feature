@done
@querying
@cli
@rust
@RPC-209
Feature: copy-virtual-hooks CLI subcommand
  """
  Shell-facing surface for the Rust port of `fspec copy-virtual-hooks --from <src> --to <dst> [--hook-name <name>]`. Lives at rust/fspec/src/copy_virtual_hooks.rs as the standard two-front-doors CLI bridge. It owns clap parsing, enforces the friendly "--from option is required" / "--to option is required" errors at the bridge layer (matching the TS Commander.js action handler), marshals camelCase JSON into the shared `copy_virtual_hooks::run` core function (defined in rust/fspec-core/src/commands/copy_virtual_hooks.rs and proven by spec/features/copy-virtual-hooks-rust-port.feature), prints the rendered text response to stdout, and surfaces InvalidArgs as a single `Error: <msg>` line to stderr followed by exit code 1.

  The CLI signature mirrors the TS Commander.js definition exactly: three options `--from <workUnitId>`, `--to <workUnitId>`, `--hook-name <name>`. Help is intercepted in `rust/fspec/src/main.rs::intercept_ts_help` BEFORE clap parses argv — the intercept arm calls `format_command_help(&configs::copy_virtual_hooks::CONFIG)` which produces output byte-for-byte identical to `node dist/index.js copy-virtual-hooks --help`. The byte-parity contract is enforced by `rust/fspec/tests/fixtures/help/copy-virtual-hooks.txt`.
  """

  Background: User Story
    As a shell user of the standalone fspec Rust binary
    I want to run `fspec copy-virtual-hooks --from <src> --to <dst>` and see chalk-style success or failure feedback
    So that I can replicate virtual hooks between work units without going through the LLM dispatcher

  Scenario: CLI prints success message when copying all hooks
    Given a project root whose spec/work-units.json contains AUTH-001 with two virtualHooks and AUTH-002 with no hooks
    When I run `./rust/target/release/fspec copy-virtual-hooks --from AUTH-001 --to AUTH-002` in that project root
    Then the command exits 0
    And stdout contains the substring "✓ Copied 2 virtual hook(s) from AUTH-001 to AUTH-002"

  Scenario: CLI prints success message when copying a single named hook
    Given a project root whose spec/work-units.json contains AUTH-001 with virtualHooks 'lint' and 'eslint', and AUTH-002 with no hooks
    When I run `./rust/target/release/fspec copy-virtual-hooks --from AUTH-001 --to AUTH-002 --hook-name eslint` in that project root
    Then the command exits 0
    And stdout contains the substring "✓ Copied 1 virtual hook(s) from AUTH-001 to AUTH-002"

  Scenario: CLI exits 1 when --from is omitted
    Given an empty project root with no spec/ subdirectory
    When I run `./rust/target/release/fspec copy-virtual-hooks --to AUTH-002` in that project root
    Then the command exits 1
    And stderr contains the substring "--from option is required"

  Scenario: CLI exits 1 when --to is omitted
    Given an empty project root with no spec/ subdirectory
    When I run `./rust/target/release/fspec copy-virtual-hooks --from AUTH-001` in that project root
    Then the command exits 1
    And stderr contains the substring "--to option is required"

  Scenario: CLI exits 1 when source has no hooks
    Given a project root whose spec/work-units.json contains AUTH-001 with no virtualHooks and AUTH-002 with no virtualHooks
    When I run `./rust/target/release/fspec copy-virtual-hooks --from AUTH-001 --to AUTH-002` in that project root
    Then the command exits 1
    And stderr contains the substring "No virtual hooks configured for source work unit AUTH-001"

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher (no duplicated business logic)
    Given a project root whose spec/work-units.json contains AUTH-001 with one virtualHook 'lint' and AUTH-002 with no hooks
    When I dispatch copy-virtual-hooks through fspec_core::dispatch::dispatch_command with from='AUTH-001' and to='AUTH-002'
    Then the dispatcher's DispatchResult.data parses to a JSON object with copiedCount=1
    And the CLI bridge module rust/fspec/src/copy_virtual_hooks.rs contains NO inline rendering, file IO beyond cwd resolution, or work-unit-lookup logic — its only computation is JSON arg marshalling plus the --from/--to presence guard

  Scenario: copy-virtual-hooks --help is byte-for-byte identical to TS formatCommandHelp reference output
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec copy-virtual-hooks --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/copy-virtual-hooks.txt
    And stdout starts with a blank line followed by 'COPY-VIRTUAL-HOOKS'
    And stdout contains the section header 'COMMON PATTERNS'
