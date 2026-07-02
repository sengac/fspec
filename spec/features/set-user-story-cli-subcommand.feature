@done
@RPC-298
@rust
@cli
@mutation
Feature: fspec set-user-story CLI subcommand (Rust port)
  """
  Clap derive subcommand `set-user-story` exposes the same surface as the TS Commander.js registration at src/commands/set-user-story.ts:65-80 — positional `<work-unit-id>` plus required `--role`, `--action`, `--benefit` flags. The bridge at codelet/fspec/src/set_user_story.rs marshals the clap args into JSON and delegates to codelet_fspec_core::commands::set_user_story::run; no rendering or persistence logic is duplicated.
  Exit codes: 0 on success, 1 on any FspecCoreError. Errors are written to stderr prefixed with 'Error:'.
  The `fspec set-user-story --help` output is byte-for-byte identical to `node dist/index.js set-user-story --help` — captured as codelet/fspec/tests/fixtures/help/set-user-story.txt.
  """

  Background: User Story
    As a fspec user running the standalone Rust binary
    I want a `set-user-story` subcommand whose CLI shape mirrors the TypeScript reference
    So that scripts and muscle-memory keep working when the binary swap from Node.js to Rust lands

  Scenario: Clap exposes set-user-story with positional and required flags in --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec set-user-story --help`
    Then the command exits 0
    And stdout describes the set-user-story subcommand
    And stdout mentions the `--role` flag
    And stdout mentions the `--action` flag
    And stdout mentions the `--benefit` flag
    And stdout does NOT advertise a `--workspace` global flag

  Scenario: CLI writes the user story and prints the success block
    Given spec/work-units.json contains work unit 'AUTH-001' with no userStory
    When I run `./codelet/target/release/fspec set-user-story AUTH-001 --role developer --action ship --benefit happiness`
    Then the command exits 0
    And stdout contains the line '✓ User story set for AUTH-001'
    And stdout contains the line '  As a developer'
    And stdout contains the line '  I want to ship'
    And stdout contains the line '  So that happiness'
    And spec/work-units.json work unit 'AUTH-001' has userStory.role='developer'

  Scenario: CLI rejects an unknown work unit with exit 1 and stderr Error prefix
    Given spec/work-units.json contains no work unit 'MISSING-001'
    When I run `./codelet/target/release/fspec set-user-story MISSING-001 --role x --action y --benefit z`
    Then the command exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring "Work unit 'MISSING-001' does not exist"

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given spec/work-units.json contains work unit 'AUTH-001'
    When I dispatch set-user-story via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' role='dev' action='go' benefit='win'
    Then the dispatcher writes spec/work-units.json
    And the CLI bridge module codelet/fspec/src/set_user_story.rs contains NO inline userStory build, file-write, or success-line rendering — its only computation is JSON arg marshalling

  Scenario: set-user-story --help is byte-for-byte identical to TS reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec set-user-story --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/set-user-story.txt
    And stdout starts with a blank line followed by 'SET-USER-STORY'
