@done
@RPC-210
@rust
@cli
@mutation
Feature: fspec create-bug CLI subcommand (Rust port)
  """
  Clap derive subcommand `create-bug` exposes the same surface as the TS Commander.js registration at src/commands/create-bug.ts:278-288 — two positional arguments `<prefix>` and `<title>` plus optional `-d, --description`, `-e, --epic`, and `-p, --parent` flags. The bridge module at codelet/fspec/src/create_bug.rs marshals the clap args into a JSON object and delegates to codelet_fspec_core::commands::create_bug::run; no validation or rendering logic is duplicated.
  Exit codes: 0 on success, 1 on any FspecCoreError. Errors are written to stderr prefixed with 'Error:' (parity with the TS error path at src/commands/create-bug.ts:269-275). The success block (✓ Created bug <id>, Title:, optional Description:/Epic:/Parent:) prints to stdout and the research-guidance system-reminder prints to stderr.
  The `fspec create-bug --help` output is byte-for-byte identical to the captured fixture at codelet/fspec/tests/fixtures/help/create-bug.txt.
  """

  Background: User Story
    As a fspec user running the standalone Rust binary
    I want a `create-bug` subcommand whose CLI shape mirrors the TypeScript reference
    So that scripts and muscle-memory keep working when the binary swap from Node.js to Rust lands

  Scenario: Clap exposes create-bug with positional args and option flags in --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec create-bug --help`
    Then the command exits 0
    And stdout describes the create-bug subcommand
    And stdout mentions the `<prefix>` argument
    And stdout mentions the `<title>` argument
    And stdout advertises the `--description` flag (or its `-d` short form)
    And stdout advertises the `--epic` flag (or its `-e` short form)
    And stdout advertises the `--parent` flag (or its `-p` short form)

  Scenario: CLI creates a minimal bug and prints the success block
    Given a working directory with spec/foundation.json present and prefix 'BUG' registered
    When I run `./codelet/target/release/fspec create-bug BUG "Login crash"`
    Then the command exits 0
    And stdout contains the line '✓ Created bug BUG-001'
    And stdout contains the line '  Title: Login crash'
    And the file spec/work-units.json contains work unit 'BUG-001' with type='bug'
    And stderr contains the substring 'Bug BUG-001 created successfully.'

  Scenario: CLI creates a bug with description, epic, and parent printing all detail lines
    Given a working directory with spec/foundation.json present, prefix 'BUG' registered, an existing bug 'BUG-001', and an existing epic 'auth'
    When I run `./codelet/target/release/fspec create-bug BUG "Login crash" -d "Crashes on submit" -e auth -p BUG-001`
    Then the command exits 0
    And stdout contains the line '✓ Created bug BUG-002'
    And stdout contains the line '  Description: Crashes on submit'
    And stdout contains the line '  Epic: auth'
    And stdout contains the line '  Parent: BUG-001'

  Scenario: CLI fails when foundation is missing with exit 1 and stderr Error prefix
    Given a working directory with no spec/foundation.json
    When I run `./codelet/target/release/fspec create-bug BUG "Login crash"`
    Then the command exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Project foundation not found'

  Scenario: CLI rejects an unregistered prefix with exit 1
    Given a working directory with spec/foundation.json present and no registered prefixes
    When I run `./codelet/target/release/fspec create-bug BUG "Login crash"`
    Then the command exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring "Prefix 'BUG' is not registered"

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a working directory with spec/foundation.json present and prefix 'BUG' registered
    When I dispatch create-bug via fspec_core::dispatch::dispatch_command with prefix='BUG' title='First'
    Then the dispatcher writes spec/work-units.json with 'BUG-001'
    And running `./codelet/target/release/fspec create-bug BUG "Second"` afterwards exits 0
    And spec/work-units.json now contains both 'BUG-001' and 'BUG-002'
    And the CLI bridge module codelet/fspec/src/create_bug.rs contains NO inline validation, id-generation, or file-write logic — its only computation is JSON arg marshalling

  Scenario: create-bug --help is byte-for-byte identical to the fixture
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec create-bug --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/create-bug.txt
