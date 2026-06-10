@done
@feature-management
@work-management
@prefix-epic
@prefixes
@rust
@cli
@RPC-213
Feature: Create prefix CLI subcommand

  """
  CLI subcommand is wired into codelet/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003 §7/§11. The action arm delegates to fspec_core::commands::create_prefix::run(args_json, &cwd) so business logic is not duplicated between the LLM-facing dispatcher and the shell-facing CLI.

  The subcommand exposes NO flags — only the two required positional args `<prefix>` and `<description>`, mirroring the TypeScript Commander.js registration at src/commands/create-prefix.ts:66-86. The global `--workspace` flag, `--format`, `--json`, etc. are all out of scope for RPC-213.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec create-prefix <prefix> <description>` directly from a shell with the same flag-less surface offered by the TypeScript Commander.js CLI
    So that I can register prefixes from a script or terminal without going through the LLM tool-call dispatcher

  Scenario: Clap exposes create-prefix as a subcommand and prints flag-aware --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec create-prefix --help` from a shell
    Then the command exits 0
    Then stdout contains the substring 'CREATE-PREFIX'
    Then stdout contains the substring '<prefix>'
    Then stdout contains the substring '<description>'
    Then stdout does NOT contain the substring '--format'
    Then stdout does NOT contain the substring '--json'
    Then stdout does NOT contain the substring '--workspace'

  Scenario: CLI against empty directory creates the prefixes file and prints the success message
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./codelet/target/release/fspec create-prefix AUTH "Auth features"` from that directory
    Then the command exits 0
    Then stdout contains the substring '✓ Prefix AUTH created successfully'
    Then spec/prefixes.json now exists in the directory
    Then spec/prefixes.json contains a top-level prefixes object with an AUTH key whose description is 'Auth features'

  Scenario: CLI rejects a lowercase prefix with exit 1 and stderr Error prefix
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./codelet/target/release/fspec create-prefix auth "bad case"` from that directory
    Then the command exits with code 1
    Then stderr contains the substring 'Error:'
    Then stderr contains the substring 'Prefix must be 2-6 uppercase letters'
    Then spec/prefixes.json was NOT created in the directory

  Scenario: CLI rejects a duplicate prefix without touching the file
    Given spec/prefixes.json contains AUTH (description 'Auth features')
    When I run `./codelet/target/release/fspec create-prefix AUTH "Different desc"` from that directory
    Then the command exits with code 1
    Then stderr contains the substring 'Error:'
    Then stderr contains the substring 'Prefix AUTH already exists'
    Then spec/prefixes.json is byte-identical to its pre-call content

  Scenario: CLI surfaces a malformed prefixes.json parse error to stderr
    Given spec/prefixes.json exists in the working directory but contains invalid JSON
    When I run `./codelet/target/release/fspec create-prefix AUTH "Auth features"` from that directory
    Then the command exits with code 1
    Then stderr contains the substring 'Error:'
    Then stderr contains the substring 'Failed to parse prefixes.json'

  Scenario: CLI missing positional argument fails with a clap usage error
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./codelet/target/release/fspec create-prefix` from that directory
    Then the command exits with a non-zero code
    Then stderr contains the substring 'prefix' or 'description'
    Then spec/prefixes.json was NOT created in the directory

  Scenario: Default combined TUI mode is preserved when no subcommand is provided
    Given the fspec Rust binary has create-prefix registered as a clap subcommand alongside daemon, client, status, list-work-units, and list-prefixes
    When I run `./codelet/target/release/fspec --help`
    Then the help output lists daemon, client, status, list-work-units, list-prefixes, and create-prefix as available subcommands
    Then the long-about description still documents that running fspec with no subcommand enters combined TUI mode

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher (no duplicated business logic)
    Given a project root with no spec/ subdirectory
    When I dispatch create-prefix through fspec_core::dispatch::dispatch_command with prefix='AUTH' and description='Auth features'
    Then the dispatcher's DispatchResult.success is true and spec/prefixes.json now contains AUTH
    Then the CLI bridge module codelet/fspec/src/create_prefix.rs contains NO inline validation, file IO, or rendering logic — its only computation is JSON arg marshalling and stdout/stderr printing

  Scenario: create-prefix --help is byte-for-byte identical to TS formatCommandHelp reference output
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec create-prefix --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/create-prefix.txt
    And stdout starts with a blank line followed by 'CREATE-PREFIX'
