@done
@rust
@querying
@cli
@RPC-252
Feature: List Virtual Hooks Cli Subcommand
  """
  CLI subcommand is wired into codelet/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003 §7/§11. The action arm delegates to fspec_core::commands::list_virtual_hooks::run(args_json, &cwd) so business logic is not duplicated between the LLM-facing dispatcher and the shell-facing CLI.

  The subcommand exposes a single required positional argument `<work_unit_id>` and NO flags — mirroring the TypeScript Commander.js registration at src/commands/list-virtual-hooks.ts:49-53 which declares `.command('list-virtual-hooks').argument('<workUnitId>', 'Work unit ID')` with no `.option(...)` calls. This is intentional: --format / --workspace are out of scope for RPC-252.

  The text-format output begins with a leading newline (parity with the TS `output.log(\`\nVirtual Hooks for ${workUnitId}:\n\`)` at src/commands/list-virtual-hooks.ts:65). CLI tests assert substring containment of the header line; they do not enforce bit-stable leading whitespace.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec list-virtual-hooks <workUnitId>` directly from a shell with the same positional-argument surface offered by the TypeScript Commander.js CLI
    So that I can audit the virtual hooks registered for a work unit from a script or terminal without going through the LLM tool-call dispatcher

  Scenario: Clap exposes list-virtual-hooks as a subcommand and prints flag-aware --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec list-virtual-hooks --help` from a shell
    Then the command exits 0
    And stdout contains clap-generated help describing the list-virtual-hooks subcommand
    And stdout contains the positional placeholder "<workUnitId>"
    And stdout does NOT contain the substring '--format'
    And stdout does NOT contain the substring '--workspace'

  Scenario: CLI prints the populated text layout when the work unit has virtual hooks
    Given spec/work-units.json contains AUTH-001 with virtualHooks=[{name:'lint',event:'post-implementing',command:'npm run lint',blocking:true}]
    When I run `./codelet/target/release/fspec list-virtual-hooks AUTH-001`
    Then the command exits 0
    And stdout contains the substring "Virtual Hooks for AUTH-001:"
    And stdout contains the substring "post-implementing:"
    And stdout contains the substring "[blocking]"

  Scenario: CLI prints the empty-hooks sentinel and exits 0 when the work unit has no virtual hooks
    Given spec/work-units.json contains AUTH-001 with no virtualHooks field
    When I run `./codelet/target/release/fspec list-virtual-hooks AUTH-001`
    Then the command exits 0
    And stdout contains the substring "No virtual hooks configured for AUTH-001"

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher (no duplicated business logic)
    Given a project root whose spec/work-units.json contains AUTH-001 with virtualHooks=[{name:'lint',event:'post-implementing',command:'npm run lint',blocking:true}]
    When I dispatch list-virtual-hooks through fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' and format='json'
    Then the dispatcher's DispatchResult.data parses to a JSON object with hooks array of length 1
    And the CLI bridge module codelet/fspec/src/list_virtual_hooks.rs contains NO inline rendering, hook-grouping, or work-unit-lookup logic — its only computation is JSON arg marshalling

  Scenario: list-virtual-hooks --help is byte-for-byte identical to TS formatCommandHelp reference output
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec list-virtual-hooks --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/list-virtual-hooks.txt
    And stdout starts with a blank line followed by 'LIST-VIRTUAL-HOOKS'
    And stdout contains the section header 'COMMON PATTERNS'
