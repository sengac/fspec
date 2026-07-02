@event-storming
@event-storm
@cli
@RPC-185
Feature: add-hotspot CLI subcommand
  """
  CLI bridge: codelet/fspec/src/add_hotspot.rs — clap-derived struct mirroring TS Commander.js registration (src/commands/add-hotspot.ts). Surface: `fspec add-hotspot <workUnitId> <text> [--concern <desc>] [--timestamp <ms>] [--bounded-context <name>]`.
  Stdout (success): '✓ Hotspot added to <workUnitId> (id: <hotspotId>)' (chalk.green; ANSI tolerated via substring match).
  Stderr (failure): '✗ Failed to add hotspot: <message>'; exit code 1.
  Two-front-doors invariant: bridge marshals positional/option args into JSON and forwards to commands::add_hotspot::run — NO domain logic in the bridge.
  Help fixture captured from `node dist/index.js add-hotspot --help`.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want the standalone Rust fspec binary's add-hotspot subcommand to parse the same positional arguments and options as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven Event Storm script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    given the fspec Rust binary is built and on PATH
    when I run `fspec add-hotspot --help`
    then the exit code is 0
    And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/add-hotspot.txt

  Scenario: CLI appends a hotspot and prints the success line
    given a project root tempdir with spec/work-units.json containing RPC-185 status=specifying
    when I run `fspec add-hotspot RPC-185 "Unclear retry policy"` in that tempdir
    then the exit code is 0
    And stdout contains the substring '✓ Hotspot added to RPC-185 (id: 0)'
    And spec/work-units.json on disk shows RPC-185 eventStorm items has length 1

  Scenario: CLI rejects a missing work unit with exit 1 and TS-parity error prefix
    given a project root tempdir with spec/work-units.json that does not contain "NOPE-1"
    when I run `fspec add-hotspot NOPE-1 "X"` in that tempdir
    then the exit code is 1
    And stderr contains the substring '✗ Failed to add hotspot:'
    And stderr contains the substring "Work unit NOPE-1 not found"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    given a project root tempdir with spec/work-units.json containing RPC-185 status=specifying
    when I dispatch add-hotspot via fspec_core::dispatch::dispatch_command with workUnitId='RPC-185' text='H1'
    then the dispatcher returns success=true
    And running `fspec add-hotspot RPC-185 "H2"` afterwards exits 0
    And spec/work-units.json on disk shows RPC-185 eventStorm items has length 2
    And the CLI bridge module codelet/fspec/src/add_hotspot.rs contains NO inline item construction, status guard, or file-write logic — its only computation is JSON arg marshalling
