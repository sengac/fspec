@done
@RPC-215
@rust
@cli
@mutation
Feature: fspec create-task CLI subcommand (Rust port)
  """
  Clap derive subcommand `create-task` exposes the same surface as the TS Commander.js registration at src/commands/create-task.ts:273-283 — two positional arguments `<prefix>` and `<title>` plus optional `-d, --description`, `-e, --epic`, and `-p, --parent` flags. The bridge module at rust/fspec/src/create_task.rs marshals the clap args into a JSON object and delegates to codelet_fspec_core::commands::create_task::run; no validation or rendering logic is duplicated.
  Exit codes: 0 on success, 1 on any FspecCoreError. Errors are written to stderr prefixed with 'Error:' (parity with the TS error path at src/commands/create-task.ts:264-270). The success block (✓ Created task <id>, Title:, optional Description:/Epic:/Parent:) prints to stdout and the minimal-requirements system-reminder prints to stderr.
  The `fspec create-task --help` output is byte-for-byte identical to the captured fixture at rust/fspec/tests/fixtures/help/create-task.txt.
  """

  Background: User Story
    As a fspec user running the standalone Rust binary
    I want a `create-task` subcommand whose CLI shape mirrors the TypeScript reference
    So that scripts and muscle-memory keep working when the binary swap from Node.js to Rust lands

  Scenario: Clap exposes create-task with positional args and option flags in --help
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec create-task --help`
    Then the command exits 0
    And stdout describes the create-task subcommand
    And stdout mentions the `<prefix>` argument
    And stdout mentions the `<title>` argument
    And stdout advertises the `--description` flag (or its `-d` short form)
    And stdout advertises the `--epic` flag (or its `-e` short form)
    And stdout advertises the `--parent` flag (or its `-p` short form)

  Scenario: CLI creates a minimal task and prints the success block
    Given a working directory with spec/foundation.json present and prefix 'INFRA' registered
    When I run `./rust/target/release/fspec create-task INFRA "Setup CI pipeline"`
    Then the command exits 0
    And stdout contains the line '✓ Created task INFRA-001'
    And stdout contains the line '  Title: Setup CI pipeline'
    And the file spec/work-units.json contains work unit 'INFRA-001' with type='task'
    And stderr contains the substring 'Task INFRA-001 created successfully.'

  Scenario: CLI creates a task with description, epic, and parent printing all detail lines
    Given a working directory with spec/foundation.json present, prefix 'INFRA' registered, an existing task 'INFRA-001', and an existing epic 'ops'
    When I run `./rust/target/release/fspec create-task INFRA "Configure monitoring" -d "Datadog dashboards" -e ops -p INFRA-001`
    Then the command exits 0
    And stdout contains the line '✓ Created task INFRA-002'
    And stdout contains the line '  Description: Datadog dashboards'
    And stdout contains the line '  Epic: ops'
    And stdout contains the line '  Parent: INFRA-001'

  Scenario: CLI fails when foundation is missing with exit 1 and stderr Error prefix
    Given a working directory with no spec/foundation.json
    When I run `./rust/target/release/fspec create-task INFRA "Setup CI pipeline"`
    Then the command exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Project foundation not found'

  Scenario: CLI rejects an unregistered prefix with exit 1
    Given a working directory with spec/foundation.json present and no registered prefixes
    When I run `./rust/target/release/fspec create-task INFRA "Setup CI pipeline"`
    Then the command exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring "Prefix 'INFRA' is not registered"

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a working directory with spec/foundation.json present and prefix 'INFRA' registered
    When I dispatch create-task via fspec_core::dispatch::dispatch_command with prefix='INFRA' title='First'
    Then the dispatcher writes spec/work-units.json with 'INFRA-001'
    And running `./rust/target/release/fspec create-task INFRA "Second"` afterwards exits 0
    And spec/work-units.json now contains both 'INFRA-001' and 'INFRA-002'
    And the CLI bridge module rust/fspec/src/create_task.rs contains NO inline validation, id-generation, or file-write logic — its only computation is JSON arg marshalling

  Scenario: create-task --help is byte-for-byte identical to the fixture
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec create-task --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/create-task.txt
