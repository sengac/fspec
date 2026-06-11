@done
@RPC-188
@rust
@cli
@mutation
Feature: fspec add-question CLI subcommand (Rust port)

  """
  Clap derive subcommand `add-question` exposes the same surface as the TS Commander.js registration at src/commands/add-question.ts:86-100 — two positional arguments `<workUnitId>` and `<question>`, no flags. The bridge module at codelet/fspec/src/add_question.rs marshals the clap args into a JSON object and delegates to codelet_fspec_core::commands::add_question::run; no validation or rendering logic is duplicated.
  Exit codes: 0 on success, 1 on any FspecCoreError. Errors are written to stderr prefixed with '✗ Failed to add question:' (parity with the TS error path at src/commands/add-question.ts:97).
  The `fspec add-question --help` output is byte-for-byte identical to `node dist/index.js add-question --help` (TS reference) — captured as codelet/fspec/tests/fixtures/help/add-question.txt.
  """

  Background: User Story
    As a fspec user running the standalone Rust binary
    I want an `add-question` subcommand whose CLI shape mirrors the TypeScript reference
    So that scripts and muscle-memory keep working when the binary swap from Node.js to Rust lands


  Scenario: Clap exposes add-question with two positional args in --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec add-question --help`
    Then the command exits 0
    And stdout describes the add-question subcommand
    And stdout mentions the `<workUnitId>` argument
    And stdout mentions the `<question>` argument
    And stdout does NOT advertise a `--workspace` global flag


  Scenario: CLI adds a question and prints the success line
    Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status
    When I run `./codelet/target/release/fspec add-question AUTH-001 "Should we add OAuth?"`
    Then the command exits 0
    And stdout contains the line '✓ Question added successfully'


  Scenario: CLI rejects unknown work unit with exit 1 and stderr Failed prefix
    Given spec/work-units.json contains no work unit 'AUTH-999'
    When I run `./codelet/target/release/fspec add-question AUTH-999 "Q?"`
    Then the command exits with code 1
    And stderr contains the substring '✗ Failed to add question:'
    And stderr contains the substring 'Work unit'


  Scenario: CLI rejects wrong status with exit 1 and stderr Failed prefix
    Given spec/work-units.json contains work unit 'AUTH-001' in 'backlog' status
    When I run `./codelet/target/release/fspec add-question AUTH-001 "Q?"`
    Then the command exits with code 1
    And stderr contains the substring '✗ Failed to add question:'
    And stderr contains the substring 'discovery/specification phase'


  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status
    When I dispatch add-question via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' question='dispatched'
    Then the dispatcher mutates spec/work-units.json
    And running `./codelet/target/release/fspec add-question AUTH-001 "from-cli"` afterwards exits 0
    And spec/work-units.json now contains both 'dispatched' and 'from-cli' question texts on AUTH-001
    And the CLI bridge module codelet/fspec/src/add_question.rs contains NO inline state mutation or file-write logic — its only computation is JSON arg marshalling


  Scenario: add-question --help is byte-for-byte identical to TS reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec add-question --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/add-question.txt
    And stdout starts with a blank line followed by 'ADD-QUESTION'
