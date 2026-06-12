@done
@mutation
@cli
@rust
@RPC-214
Feature: fspec create-story CLI subcommand (Rust port)

  """
  Clap derive subcommand `create-story` exposes the same surface as the TS Commander.js registration at
  src/commands/create-story.ts:277-287 — two positional arguments `<prefix>` and `<title>` plus three optional
  flags `-d, --description <description>`, `-e, --epic <epic>`, `-p, --parent <parent>`. The bridge module at
  codelet/fspec/src/create_story.rs marshals the clap args into a JSON object {prefix, title, description?,
  epic?, parent?} and delegates to codelet_fspec_core::commands::create_story::run; no validation or rendering
  logic is duplicated.
  Stdout (success): the rendered success block — '✓ Created story <id>', '  Title: <title>', and optional
  Description/Epic/Parent lines (parity with src/commands/create-story.ts:238-249).
  Exit codes: 0 on success, 1 on any FspecCoreError; errors are written to stderr prefixed with 'Error:'
  (parity with the chalk-red TS error path at src/commands/create-story.ts:268-273).
  The `fspec create-story --help` output is byte-for-byte identical to `node dist/index.js create-story --help`
  (TS reference) — captured as codelet/fspec/tests/fixtures/help/create-story.txt.
  """

  Background: User Story
    As a fspec user running the standalone Rust binary
    I want a `create-story` subcommand whose CLI shape mirrors the TypeScript reference
    So that scripts and muscle-memory keep working when the binary swap from Node.js to Rust lands

  Scenario: create-story --help is byte-for-byte identical to TS reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec create-story --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/create-story.txt
    And stdout starts with a blank line followed by 'CREATE-STORY'

  Scenario: Clap exposes create-story with positional args and the three optional flags
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec create-story --help`
    Then the command exits 0
    And stdout mentions the `<prefix>` argument
    And stdout mentions the `<title>` argument
    And stdout advertises the `--description` flag (or its `-d` short form)
    And stdout advertises the `--epic` flag (or its `-e` short form)
    And stdout advertises the `--parent` flag (or its `-p` short form)

  Scenario: CLI creates a minimal story and prints the success block
    Given a project root tempdir with spec/foundation.json present and spec/prefixes.json registering prefix AUTH
    When I run `./codelet/target/release/fspec create-story AUTH "User login"` in that tempdir
    Then the command exits 0
    And stdout contains the line '✓ Created story AUTH-001'
    And stdout contains the line '  Title: User login'
    And spec/work-units.json on disk contains a work unit AUTH-001 with type='story'

  Scenario: CLI creates a story with -e epic and includes the Epic line
    Given a project root tempdir with spec/foundation.json, spec/prefixes.json registering prefix AUTH, and spec/epics.json containing epic 'auth'
    When I run `./codelet/target/release/fspec create-story AUTH "User login" -e auth` in that tempdir
    Then the command exits 0
    And stdout contains the line '✓ Created story AUTH-001'
    And stdout contains the line '  Epic: auth'
    And spec/epics.json on disk shows epic 'auth' workUnits contains 'AUTH-001'

  Scenario: CLI rejects an unregistered prefix with exit 1 and stderr Error prefix
    Given a project root tempdir with spec/foundation.json present and an empty spec/prefixes.json
    When I run `./codelet/target/release/fspec create-story NOPE "User login"` in that tempdir
    Then the command exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring "Prefix 'NOPE' is not registered"

  Scenario: CLI rejects a missing foundation with exit 1 and the foundation-missing message
    Given an empty working directory with no spec/ subdirectory
    When I run `./codelet/target/release/fspec create-story AUTH "User login"` in that tempdir
    Then the command exits with code 1
    And stderr contains the substring 'Project foundation not found'

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/foundation.json present and spec/prefixes.json registering prefix AUTH
    When I dispatch create-story via fspec_core::dispatch::dispatch_command with prefix='AUTH' title='First'
    Then the dispatcher returns success=true
    And running `./codelet/target/release/fspec create-story AUTH "Second"` afterwards exits 0
    And spec/work-units.json on disk contains both 'AUTH-001' and 'AUTH-002'
    And the CLI bridge module codelet/fspec/src/create_story.rs contains NO inline foundation check, prefix validation, id generation, or file-write logic — its only computation is JSON arg marshalling
