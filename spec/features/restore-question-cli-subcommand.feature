@done
@RPC-290
Feature: fspec restore-question CLI subcommand (Rust port)
  """
  Clap derive subcommand `restore-question` exposes the same surface as the TS Commander.js registration at src/commands/restore-question.ts:88-110 — two positional arguments `<workUnitId>` and `<index>`, no flags wired into the action (the help-doc advertises `--ids` but the TS CLI does not implement it; we mirror Commander's actual surface — positional-only).
  The bridge module at rust/fspec/src/restore_question.rs marshals the clap args into a JSON object and delegates to codelet_fspec_core::commands::restore_question::run; no validation or rendering logic is duplicated.
  Exit codes: 0 on success, 1 on any FspecCoreError. Errors are written to stderr prefixed with '✗ Failed to restore question:' (parity with the TS error path at src/commands/restore-question.ts:107).
  The `fspec restore-question --help` output is byte-for-byte identical to `node dist/index.js restore-question --help` (TS reference) — captured as rust/fspec/tests/fixtures/help/restore-question.txt.
  """

  Background: User Story
    As a fspec user running the standalone Rust binary
    I want a `restore-question` subcommand whose CLI shape mirrors the TypeScript reference
    So that scripts and muscle-memory keep working when the binary swap from Node.js to Rust lands

  Scenario: Clap exposes restore-question with two positional args in --help
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec restore-question --help`
    Then the command exits 0
    And stdout describes the restore-question subcommand
    And stdout mentions the `<workUnitId>` argument
    And stdout mentions the `<index>` argument
    And stdout does NOT advertise a `--workspace` global flag

  Scenario: CLI restores a question and prints the success line
    Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with one question id=0 text 'Q?' marked deleted
    When I run `./rust/target/release/fspec restore-question AUTH-001 0`
    Then the command exits 0
    And stdout contains the line '✓ Restored question: "Q?"'

  Scenario: CLI prints idempotent success message when already active
    Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with one question id=0 text 'Q?' deleted=false
    When I run `./rust/target/release/fspec restore-question AUTH-001 0`
    Then the command exits 0
    And stdout contains the line '✓ Restored question: "Q?"'
    And stdout contains the substring 'Item ID 0 already active'

  Scenario: CLI rejects unknown question ID with exit 1 and stderr Failed prefix
    Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with one question id=0 marked deleted
    When I run `./rust/target/release/fspec restore-question AUTH-001 5`
    Then the command exits with code 1
    And stderr contains the substring 'Question with ID 5 not found'

  Scenario: CLI rejects unknown work unit with exit 1 and stderr Failed prefix
    Given spec/work-units.json contains no work unit 'AUTH-999'
    When I run `./rust/target/release/fspec restore-question AUTH-999 0`
    Then the command exits with code 1
    And stderr contains the substring 'Work unit'

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with two questions id=0 and id=1 both marked deleted
    When I dispatch restore-question via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' index=0
    Then the dispatcher mutates spec/work-units.json
    And running `./rust/target/release/fspec restore-question AUTH-001 1` afterwards exits 0
    And spec/work-units.json on disk shows both questions with deleted=false
    And the CLI bridge module rust/fspec/src/restore_question.rs contains NO inline state mutation or file-write logic — its only computation is JSON arg marshalling

  Scenario: restore-question --help is byte-for-byte identical to TS reference
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec restore-question --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/restore-question.txt
    And stdout starts with a blank line followed by 'RESTORE-QUESTION'
