@wip
@RPC-293
Feature: fspec retag CLI subcommand
  """
  CLI bridge: rust/fspec/src/retag.rs — clap-derived struct mirroring the TS Commander.js
  registration (src/commands/retag.ts:214-221). Surface: `fspec retag --from <tag> --to <tag> [--dry-run]`.
  Stdout (success, real): '✓ <message>' then 'Modified files:' + '  - <file>' list, exit 0.
  Stdout (dry-run): 'Dry run mode - no files modified' + cyan summary + '  - <file>' list, exit 0.
  Stderr (failure): 'Error: <message>', exit 1.
  Two-front-doors invariant: the bridge marshals args into JSON {from,to,dryRun} and forwards to
  fspec_core commands::retag::run — NO domain logic in the bridge.
  Help fixture captured from `node dist/index.js retag --help`.
  SURFACE NOTE: TS registers --from/--to/--dry-run FLAGS (not positionals); Rust clap mirrors flags.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want the standalone Rust fspec binary's retag subcommand to parse the same flags as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven retag script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec retag --help`
    Then the exit code is 0
    And the stdout matches the canonical help fixture at rust/fspec/tests/fixtures/help/retag.txt

  Scenario: CLI renames a tag and prints the success summary
    Given a project root tempdir with two spec/features feature files that each tag a scenario with @wip
    When I run `fspec retag --from @wip --to @in-progress` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓'
    And stdout contains the substring 'Modified files:'
    And neither feature file on disk contains the token '@wip' anymore

  Scenario: CLI dry run prints the preview and modifies nothing
    Given a project root tempdir with two spec/features feature files that each tag a scenario with @wip
    When I run `fspec retag --from @wip --to @in-progress --dry-run` in that tempdir
    Then the exit code is 0
    And stdout contains the substring 'Dry run mode - no files modified'
    And both feature files on disk are byte-equal to their pre-call contents

  Scenario: CLI reports a not-found tag with exit 1 and the TS-parity error prefix
    Given a project root tempdir with one spec/features feature file tagged @wip
    When I run `fspec retag --from @missing --to @found` in that tempdir
    Then the exit code is 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Tag @missing not found in any feature files'
    And the feature file on disk is byte-equal to its pre-call contents

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with one spec/features feature file tagged @wip
    When I dispatch retag via fspec_core::dispatch::dispatch_command with from='@wip' to='@done'
    Then the dispatcher returns success=true
    And running `fspec retag --from @done --to @wip` afterwards exits 0
    And the CLI bridge module rust/fspec/src/retag.rs contains NO inline glob, regex replace, Gherkin re-parse, or file-write logic — its only computation is JSON arg marshalling
