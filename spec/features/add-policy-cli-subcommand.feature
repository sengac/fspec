@done
@mutation
@cli
@rust
@RPC-187
Feature: fspec add-policy CLI subcommand (Rust port)

  """
  CLI bridge: codelet/fspec/src/add_policy.rs — clap-derived struct mirroring the TS Commander.js
  registration (src/commands/add-policy.ts:73-116). Surface:
  `fspec add-policy <workUnitId> <text> [--when <event>] [--then <command>] [--timestamp <ms>] [--bounded-context <name>]`.
  Stdout (success): '✓ Policy added to <workUnitId> (id: <policyId>)' (TS uses chalk.green; ANSI tolerated via
  substring match — matches src/commands/add-policy.ts:106-110).
  Stderr (failure): '✗ Failed to add policy: <message>'; exit code 1. Mirrors TS
  output.error('✗ Failed to add policy:', ...) at src/commands/add-policy.ts:101-103 and 111-113.
  --timestamp is parsed as an integer (TS uses parseInt at src/commands/add-policy.ts:81).
  Two-front-doors invariant: the bridge marshals args into JSON {workUnitId, text, when?, then?, timestamp?,
  boundedContext?} and forwards to commands::add_policy::run — NO domain logic in the bridge.
  Help fixture captured from `node dist/index.js add-policy --help`, stored at
  codelet/fspec/tests/fixtures/help/add-policy.txt.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want the standalone Rust fspec binary's add-policy subcommand to parse the same arguments as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven Event Storming script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec add-policy --help`
    Then the exit code is 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/add-policy.txt
    And stdout starts with a blank line followed by 'ADD-POLICY'

  Scenario: Clap exposes add-policy with positional args and the four optional flags
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec add-policy --help`
    Then the exit code is 0
    And stdout mentions the `<workUnitId>` argument
    And stdout mentions the `<text>` argument
    And stdout advertises the `--when` flag
    And stdout advertises the `--then` flag
    And stdout advertises the `--timestamp` flag
    And stdout advertises the `--bounded-context` flag

  Scenario: CLI successfully appends a policy and prints the success line
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    When I run `./codelet/target/release/fspec add-policy AUTH-001 "Send welcome email"` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Policy added to AUTH-001 (id: 0)'
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].type='policy'
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].text='Send welcome email'

  Scenario: CLI forwards the when/then/bounded-context flags into the persisted item
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    When I run `./codelet/target/release/fspec add-policy AUTH-001 "Send welcome email" --when UserRegistered --then SendWelcomeEmail --bounded-context Identity` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Policy added to AUTH-001 (id: 0)'
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].when='UserRegistered'
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].then='SendWelcomeEmail'
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].boundedContext='Identity'

  Scenario: CLI rejects a done-state work unit with exit 1 and the TS-parity error prefix
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=done
    When I run `./codelet/target/release/fspec add-policy AUTH-001 "Anything"` in that tempdir
    Then the exit code is 1
    And stderr contains the substring '✗ Failed to add policy:'
    And stderr contains the substring 'Cannot add Event Storm items to work unit in done state'
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    When I dispatch add-policy via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' text='P1'
    Then the dispatcher returns success=true
    And running `./codelet/target/release/fspec add-policy AUTH-001 "P2"` afterwards exits 0
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items has length 2
    And the CLI bridge module codelet/fspec/src/add_policy.rs contains NO inline policy construction, status guard, or file-write logic — its only computation is JSON arg marshalling
