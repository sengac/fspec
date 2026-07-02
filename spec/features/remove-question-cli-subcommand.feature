@done
@RPC-278
@rust
@cli
@mutation
Feature: fspec remove-question CLI subcommand (Rust port)
  """
  Clap derive subcommand `remove-question` exposes the same surface as the TS Commander.js registration at src/commands/remove-question.ts:88-107 — two positional arguments `<workUnitId>` and `<index>`, no flags. The bridge module at codelet/fspec/src/remove_question.rs marshals the clap args into a JSON object and delegates to codelet_fspec_core::commands::remove_question::run; no validation or rendering logic is duplicated.
  Exit codes: 0 on success, 1 on any FspecCoreError. Errors are written to stderr prefixed with '✗ Failed to remove question:' (parity with the TS error path at src/commands/remove-question.ts:106).
  The `fspec remove-question --help` output is byte-for-byte identical to `node dist/index.js remove-question --help` (TS reference) — captured as codelet/fspec/tests/fixtures/help/remove-question.txt.
  """

  Background: User Story
    As a fspec user running the standalone Rust binary
    I want a `remove-question` subcommand whose CLI shape mirrors the TypeScript reference
    So that scripts and muscle-memory keep working when the binary swap from Node.js to Rust lands

  Scenario: Clap exposes remove-question with two positional args in --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec remove-question --help`
    Then the command exits 0
    And stdout describes the remove-question subcommand
    And stdout mentions the `<workUnitId>` argument
    And stdout mentions the `<index>` argument
    And stdout does NOT advertise a `--workspace` global flag

  Scenario: CLI removes a question and prints the success line
    Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with one question id=0 text 'Q?'
    When I run `./codelet/target/release/fspec remove-question AUTH-001 0`
    Then the command exits 0
    And stdout contains the line '✓ Removed question: "Q?"'

  Scenario: CLI rejects unknown question ID with exit 1 and stderr Failed prefix
    Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with one question id=0
    When I run `./codelet/target/release/fspec remove-question AUTH-001 5`
    Then the command exits with code 1
    And stderr contains the substring '✗ Failed to remove question:'
    And stderr contains the substring 'Question with ID 5 not found'

  Scenario: CLI rejects unknown work unit with exit 1 and stderr Failed prefix
    Given spec/work-units.json contains no work unit 'AUTH-999'
    When I run `./codelet/target/release/fspec remove-question AUTH-999 0`
    Then the command exits with code 1
    And stderr contains the substring '✗ Failed to remove question:'
    And stderr contains the substring 'Work unit'

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with two questions id=0 and id=1
    When I dispatch remove-question via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' index=0
    Then the dispatcher mutates spec/work-units.json
    And running `./codelet/target/release/fspec remove-question AUTH-001 1` afterwards exits 0
    And spec/work-units.json on disk shows both questions with deleted=true
    And the CLI bridge module codelet/fspec/src/remove_question.rs contains NO inline state mutation or file-write logic — its only computation is JSON arg marshalling

  Scenario: remove-question --help is byte-for-byte identical to TS reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec remove-question --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/remove-question.txt
    And stdout starts with a blank line followed by 'REMOVE-QUESTION'
