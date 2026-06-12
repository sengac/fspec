@done
@RPC-223
@rust
@cli
@mutation
Feature: fspec delete-work-unit CLI subcommand (Rust port)

  """
  Clap derive subcommand `delete-work-unit` exposes the same surface as the TS Commander.js registration at src/commands/delete-work-unit.ts:142-180 — a required positional `<workUnitId>` plus optional `--force`, `--skip-confirmation`, and `--cascade-dependencies` flags. The bridge module at codelet/fspec/src/delete_work_unit.rs marshals workUnitId and cascadeDependencies into a JSON object and delegates to codelet_fspec_core::commands::delete_work_unit::run; --force and --skip-confirmation are parsed for parity but NOT forwarded because the TS implementation never reads them.
  Exit codes: 0 on success, 1 on any FspecCoreError. Errors are written to stderr prefixed with '✗ Failed to delete work unit:' (parity with the chalk-red TS error path at src/commands/delete-work-unit.ts:172-178). The success line is '✓ Work unit <id> deleted successfully', followed by '⚠ <warning>' lines.
  The `fspec delete-work-unit --help` output is byte-for-byte identical to `node dist/index.js delete-work-unit --help` (TS reference) — captured as codelet/fspec/tests/fixtures/help/delete-work-unit.txt.
  """

  Background: User Story
    As a fspec user running the standalone Rust binary
    I want a `delete-work-unit` subcommand whose CLI shape mirrors the TypeScript reference
    So that scripts and muscle-memory keep working when the binary swap from Node.js to Rust lands

  Scenario: Clap exposes delete-work-unit with a positional arg and flags in --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec delete-work-unit --help`
    Then the command exits 0
    And stdout describes the delete-work-unit subcommand
    And stdout mentions the `<workUnitId>` argument
    And stdout advertises the `--cascade-dependencies` flag
    And stdout does NOT advertise a `--workspace` global flag

  Scenario: CLI deletes an existing leaf work unit and prints the success line
    Given spec/work-units.json contains work unit AUTH-001 with status='backlog' and no dependencies
    When I run `./codelet/target/release/fspec delete-work-unit AUTH-001`
    Then the command exits 0
    And stdout contains the line '✓ Work unit AUTH-001 deleted successfully'
    And the on-disk spec/work-units.json no longer contains the AUTH-001 work unit

  Scenario: CLI exits 1 when the work unit does not exist
    Given spec/work-units.json contains work unit AUTH-001 with status='backlog' and no dependencies
    When I run `./codelet/target/release/fspec delete-work-unit MISSING-999`
    Then the command exits with code 1
    And stderr contains the substring '✗ Failed to delete work unit:'
    And stderr contains the substring "Work unit 'MISSING-999' does not exist"

  Scenario: CLI cascades dependencies and prints a blocks warning
    Given spec/work-units.json contains work unit AUTH-001 with blocks API-001 and work unit API-001 with blockedBy AUTH-001
    When I run `./codelet/target/release/fspec delete-work-unit AUTH-001 --cascade-dependencies`
    Then the command exits 0
    And stdout contains the line '✓ Work unit AUTH-001 deleted successfully'
    And stdout contains the substring '⚠ This work unit blocks 1 work unit(s): API-001'

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given spec/work-units.json contains work unit AUTH-001 and work unit DASH-001 each with status='backlog' and no dependencies
    When I dispatch delete-work-unit via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001'
    Then the dispatcher returns success=true
    And running `./codelet/target/release/fspec delete-work-unit DASH-001` afterwards exits 0
    And spec/work-units.json contains neither AUTH-001 nor DASH-001 work units
    And the CLI bridge module codelet/fspec/src/delete_work_unit.rs contains NO inline file-read, mutation, or rendering logic — its only computation is JSON arg marshalling

  Scenario: delete-work-unit --help is byte-for-byte identical to TS reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec delete-work-unit --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/delete-work-unit.txt
    And stdout starts with a blank line followed by 'DELETE-WORK-UNIT'
