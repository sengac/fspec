@wip
@RPC-236
Feature: fspec generate-tags-md CLI subcommand
  """
  CLI bridge: rust/fspec/src/generate_tags_md.rs — clap-derived struct mirroring the TS
  Commander.js registration (src/commands/generate-tags-md.ts:96-102). Surface:
  `fspec generate-tags-md [--output <path>]`.
  Stdout (success): '✓ Generated <outputRelative> from spec/tags.json' (TS uses output.log('✓', message)).
  Stderr (failure): 'Error: <message>'; exit code 1 (TS output.error('Error:', error)).
  Two-front-doors invariant: the bridge marshals args into JSON {output?} and forwards to
  fspec_core commands::generate_tags_md::run — NO domain logic in the bridge.
  Help fixture captured from `node dist/index.js generate-tags-md --help`.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want the standalone Rust fspec binary's generate-tags-md subcommand to parse the same flags as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven documentation script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec generate-tags-md --help`
    Then the exit code is 0
    And the stdout matches the canonical help fixture at rust/fspec/tests/fixtures/help/generate-tags-md.txt

  Scenario: CLI generates TAGS.md and prints the success line
    Given a project root tempdir with a schema-valid spec/tags.json
    When I run `fspec generate-tags-md` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Generated spec/TAGS.md from spec/tags.json'
    And the file spec/TAGS.md is created in that tempdir

  Scenario: CLI forwards the --output flag to a custom path
    Given a project root tempdir with a schema-valid spec/tags.json
    When I run `fspec generate-tags-md --output docs/TAGS.md` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Generated docs/TAGS.md from spec/tags.json'
    And the file docs/TAGS.md is created in that tempdir

  Scenario: CLI reports a missing tags.json with exit 1 and the TS-parity error prefix
    Given an empty project root tempdir with no spec/tags.json
    When I run `fspec generate-tags-md` in that tempdir
    Then the exit code is 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'tags.json not found: spec/tags.json'
    And the file spec/TAGS.md is not created

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with a schema-valid spec/tags.json
    When I dispatch generate-tags-md via fspec_core::dispatch::dispatch_command with no args
    Then the dispatcher returns success=true
    And running `fspec generate-tags-md` afterwards exits 0
    And the CLI bridge module rust/fspec/src/generate_tags_md.rs contains NO inline markdown rendering, schema validation, or file-write logic — its only computation is JSON arg marshalling
