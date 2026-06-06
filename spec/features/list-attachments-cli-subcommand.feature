@done
@querying
@cli
@rust
@RPC-241
Feature: List attachments CLI subcommand

  """
  CLI subcommand is wired into codelet/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003 §7/§11. The action arm delegates to fspec_core::commands::list_attachments::run(args_json, &cwd) so business logic is not duplicated between the LLM-facing dispatcher and the shell-facing CLI.

  The subcommand exposes a single required positional argument `<work_unit_id>` and NO flags — mirroring the TypeScript Commander.js registration at src/commands/list-attachments.ts:62-66 which declares `.command('list-attachments').argument('<workUnitId>', 'Work unit ID')` with no `.option(...)` calls. This is intentional: --status / --prefix / --epic / --format / --workspace are all out of scope for RPC-241.

  The Modified-timestamp line is informational and is NOT bit-stable with Node's `Date.toLocaleString()` (which is host-locale/TZ-dependent). CLI tests assert only the "    Modified: " line prefix — never the literal time content.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec list-attachments <workUnitId>` directly from a shell with the same positional-argument surface offered by the TypeScript Commander.js CLI
    So that I can audit attachments registered for a work unit from a script or terminal without going through the LLM tool-call dispatcher

  Scenario: Clap exposes list-attachments as a subcommand and prints flag-aware --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec list-attachments --help` from a shell
    Then the command exits 0
    Then stdout contains clap-generated help describing the list-attachments subcommand
    Then stdout contains the positional placeholder "<WORK_UNIT_ID>"
    Then stdout does NOT contain the substring '--status'
    Then stdout does NOT contain the substring '--prefix'
    Then stdout does NOT contain the substring '--epic'
    Then stdout does NOT contain the substring '--format'
    Then stdout does NOT contain the substring '--workspace'

  Scenario: CLI exits 2 when the required positional argument is missing
    Given an empty directory is set as the current working directory
    When I run `./codelet/target/release/fspec list-attachments` (no positional) from that directory
    Then the command exits with code 2
    Then stderr names the missing required argument

  Scenario: CLI prints the empty-attachments sentinel and exits 0 when the work unit has no attachments
    Given spec/work-units.json contains AUTH-001 with no attachments field
    When I run `./codelet/target/release/fspec list-attachments AUTH-001`
    Then the command exits 0
    Then stdout contains the substring "No attachments found for work unit AUTH-001"

  Scenario: CLI text output renders present and missing attachments with size and ✗ markers
    Given spec/work-units.json contains AUTH-001 with attachments=["spec/attachments/AUTH-001/a.png","spec/attachments/AUTH-001/b.png"]
    Given the file spec/attachments/AUTH-001/a.png exists on disk with exactly 1234 bytes
    Given no file exists at spec/attachments/AUTH-001/b.png
    When I run `./codelet/target/release/fspec list-attachments AUTH-001`
    Then the command exits 0
    Then stdout contains the substring "Attachments for AUTH-001 (2):"
    Then stdout contains the exact line "  ✓ spec/attachments/AUTH-001/a.png"
    Then stdout contains the exact line "    Size: 1.21 KB"
    Then stdout contains a line starting with "    Modified: "
    Then stdout contains the exact line "  ✗ spec/attachments/AUTH-001/b.png"
    Then stdout contains the exact line "    File not found on filesystem"

  Scenario: CLI exits 1 and writes to stderr when the requested work unit does not exist
    Given spec/work-units.json contains AUTH-001 only (no NONEXISTENT-001 entry)
    When I run `./codelet/target/release/fspec list-attachments NONEXISTENT-001`
    Then the command exits with code 1
    Then stderr contains the substring 'Error:'
    Then stderr contains the substring "Work unit 'NONEXISTENT-001' does not exist"

  Scenario: CLI exits 1 when work-units.json is malformed
    Given spec/work-units.json exists in the working directory but contains invalid JSON
    When I run `./codelet/target/release/fspec list-attachments AUTH-001`
    Then the command exits with code 1
    Then stderr contains the substring 'Error:'
    Then stderr contains the substring 'Failed to parse work-units.json'

  Scenario: Default combined TUI mode is preserved when no subcommand is provided
    Given the fspec Rust binary has list-attachments registered as a clap subcommand alongside daemon, client, status, list-work-units, and list-prefixes
    When I run `./codelet/target/release/fspec --help`
    Then the help output lists daemon, client, status, list-work-units, list-prefixes, and list-attachments as available subcommands
    Then the long-about description still documents that running fspec with no subcommand enters combined TUI mode

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher (no duplicated business logic)
    Given a project root whose spec/work-units.json contains AUTH-001 with attachments=["spec/attachments/AUTH-001/x.png"]
    Given the file spec/attachments/AUTH-001/x.png exists on disk with exactly 1024 bytes
    When I dispatch list-attachments through fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' and format='json'
    Then the dispatcher's DispatchResult.data parses to a JSON object with attachments array of length 1
    Then the CLI bridge module codelet/fspec/src/list_attachments.rs contains NO inline rendering, file-stat, or work-unit-lookup logic — its only computation is JSON arg marshalling
