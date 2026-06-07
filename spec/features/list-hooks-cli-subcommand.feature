@done
@RPC-247
@rust
@cli
Feature: List hooks CLI subcommand

  """
  CLI subcommand is wired into codelet/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003 §7/§11. The action arm delegates to fspec_core::commands::list_hooks::run(args_json, &cwd) so business logic is not duplicated between the LLM-facing dispatcher and the shell-facing CLI.

  The subcommand exposes NO flags — mirroring the TypeScript Commander.js registration at src/commands/list-hooks.ts:47-53 which only declares `.command('list-hooks').description('List all configured lifecycle hooks')` with no `.option(...)` calls. --status / --event / --format / --workspace are out of scope for RPC-247.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec list-hooks` directly from a shell with the same flag-less surface offered by the TypeScript Commander.js CLI
    So that I can list configured lifecycle hooks from a script or terminal without going through the LLM tool-call dispatcher

  Scenario: list-hooks --help is byte-for-byte identical to TS reference output
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    And the TS reference binary `node dist/index.js list-hooks --help` produces a documented 51-line block starting with a blank line, then "LIST-HOOKS", "List all configured lifecycle hooks", then WHEN TO USE / USAGE / OPTIONS / TYPICAL WORKFLOW / EXAMPLES / COMMON ERRORS / RELATED COMMANDS / NOTES sections
    When I run `./codelet/target/release/fspec list-hooks --help` piped to non-TTY (no color codes)
    Then the command exits 0
    Then stdout is byte-for-byte identical to the TS reference output
    Then stdout starts with a blank line followed by "LIST-HOOKS"
    Then stdout contains the section header "WHEN TO USE"
    Then stdout contains the section header "USAGE" followed by "  fspec list-hooks"
    Then stdout contains the section header "OPTIONS" followed by "  No options available"
    Then stdout contains the section header "TYPICAL WORKFLOW"
    Then stdout contains the section header "EXAMPLES" with both documented examples
    Then stdout contains the section header "COMMON ERRORS"
    Then stdout contains the section header "RELATED COMMANDS" listing validate-hooks, add-hook, remove-hook
    Then stdout contains the section header "NOTES" listing the four documented notes
    Then stdout does NOT contain the substring '--format'
    Then stdout does NOT contain the substring '--event'
    Then stdout does NOT contain the substring '--workspace'

  Scenario: CLI against empty directory writes zero bytes to stdout and does not auto-create files
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./codelet/target/release/fspec list-hooks` from that directory
    Then the command exits 0
    Then spec/fspec-hooks.json was NOT created in the directory
    Then stdout is exactly zero bytes (byte-parity with TS Commander.js action that discards listHooks result)

  Scenario: CLI writes zero bytes to stdout regardless of populated hooks (TS Commander.js action discards result)
    Given spec/fspec-hooks.json contains event 'pre-implementing' with hooks ['lint'] and event 'post-implementing' with hooks ['test', 'notify'] in that order
    When I run `./codelet/target/release/fspec list-hooks`
    Then the command exits 0
    Then stdout is exactly zero bytes (byte-parity with TS Commander.js action that discards listHooks result)

  Scenario: CLI exits 0 with zero stdout bytes when spec/fspec-hooks.json contains invalid JSON
    Given spec/fspec-hooks.json exists in the working directory but contains invalid JSON syntax
    When I run `./codelet/target/release/fspec list-hooks`
    Then the command exits 0
    Then stderr does NOT contain the substring 'Error:'
    Then stdout is exactly zero bytes (byte-parity with TS Commander.js action that discards listHooks result on the swallowed-error path)

  Scenario: Default combined TUI mode is preserved when no subcommand is provided
    Given the fspec Rust binary has list-hooks registered as a clap subcommand alongside daemon, client, status, list-work-units, and list-prefixes
    When I run `./codelet/target/release/fspec --help`
    Then the help output lists daemon, client, status, list-work-units, list-prefixes, and list-hooks as available subcommands
    Then the long-about description still documents that running fspec with no subcommand enters combined TUI mode

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher (no duplicated business logic)
    Given a project root whose spec/fspec-hooks.json contains event 'post-implementing' with hooks ['lint','test']
    When I dispatch list-hooks through fspec_core::dispatch::dispatch_command with format='json'
    Then the dispatcher's DispatchResult.data parses to an events array containing one entry with event='post-implementing' and hooks=['lint','test']
    Then the CLI bridge module codelet/fspec/src/list_hooks.rs contains NO inline event-aggregation or rendering logic — its only computation is JSON arg marshalling
