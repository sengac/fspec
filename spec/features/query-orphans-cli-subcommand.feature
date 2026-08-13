@done
@cli
@querying
@RPC-262
@wip
Feature: query-orphans clap subcommand on the standalone fspec Rust binary
  """
  CLI surface for the `query-orphans` subcommand on the standalone fspec Rust binary.
  Two-front-doors pattern:
  - Shell argv         → clap → rust/fspec/src/query_orphans.rs → fspec_core::commands::query_orphans::run
  - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::query_orphans::run
  Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
  The CLI surface resolves project_root from CWD (parity with TS `process.cwd()` default).
  The clap subcommand exposes `--output <text|json>` defaulting to `text` and `--exclude-done` (boolean).
  Text format prints a multi-line summary; the empty case prints '✓ No orphaned work units found.' verbatim.
  JSON format prints the pretty-printed JSON payload returned by fspec_core::commands::query_orphans::run.
  Exit-code contract: 0 on success, 1 on any FspecCoreError with stderr prefixed `Error:`.
  --help is byte-for-byte identical to the captured TS fixture at rust/fspec/tests/fixtures/help/query-orphans.txt.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want a query-orphans clap subcommand that delegates to the same fspec_core function the LLM dispatcher uses
    So that orphan-detection logic is never duplicated and byte-parity with the TS CLI is preserved

  Scenario: Clap exposes query-orphans as a subcommand and prints flag-aware --help
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec query-orphans --help` from a shell
    Then the command exits 0
    And stdout contains the substring 'query-orphans'

  Scenario: CLI without options prints success message against an empty workspace
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./rust/target/release/fspec query-orphans` from that directory
    Then the command exits 0
    And stdout contains the exact line '✓ No orphaned work units found.'

  Scenario: CLI with --output=json prints empty orphans array
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./rust/target/release/fspec query-orphans --output json` from that directory
    Then the command exits 0
    And stdout parses as JSON whose root object has orphans=[]

  Scenario: CLI text output lists each orphan with suggested actions
    Given a workspace whose spec/work-units.json contains MISC-001 with no epic and no relationships
    When I run `./rust/target/release/fspec query-orphans` from that workspace
    Then the command exits 0
    And stdout contains the substring 'Found 1 orphaned work unit(s):'
    And stdout contains the substring 'MISC-001'
    And stdout contains the substring 'No epic or dependency relationships'
    And stdout contains the substring 'Assign epic'

  Scenario: CLI --exclude-done flag suppresses done orphans
    Given a workspace whose spec/work-units.json contains DONE-1 with status='done' (orphaned) and OPEN-1 with status='backlog' (orphaned)
    When I run `./rust/target/release/fspec query-orphans --exclude-done --output json` from that workspace
    Then the command exits 0
    And stdout parses as JSON whose orphans array contains OPEN-1 and does NOT contain DONE-1

  Scenario: CLI exits 1 and writes to stderr when work-units.json is malformed
    Given spec/work-units.json exists in the working directory but contains invalid JSON
    When I run `./rust/target/release/fspec query-orphans --output json` from that directory
    Then the command exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Failed to parse work-units.json'

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a workspace whose spec/work-units.json contains MISC-001 with no epic and no relationships
    When I dispatch query-orphans through fspec_core::dispatch::dispatch_command with output='json' against that workspace
    And I run `./rust/target/release/fspec query-orphans --output json` against the same workspace
    Then both invocations produce JSON with orphans[0].id='MISC-001'
    And the CLI bridge module rust/fspec/src/query_orphans.rs contains NO inline orphan-detection or rendering logic — its only computation is JSON arg marshalling and stdout printing

  Scenario: query-orphans --help is byte-for-byte identical to TS reference
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec query-orphans --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/query-orphans.txt
    And stdout starts with a blank line followed by 'QUERY-ORPHANS'
