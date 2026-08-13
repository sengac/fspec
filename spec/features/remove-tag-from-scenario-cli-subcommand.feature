@done
@RPC-282
Feature: fspec remove-tag-from-scenario CLI subcommand
  """
  CLI bridge: rust/fspec/src/remove_tag_from_scenario.rs — clap-derived struct mirroring the
  TS Commander.js registration (src/commands/remove-tag-from-scenario.ts:213-226). Surface:
  `fspec remove-tag-from-scenario <file> <scenario> <tags...>`.
  Stdout (success): '✓ <message>' via output.log.
  Stderr (failure): 'Error: <message>'; exit code 1.
  Two-front-doors invariant: bridge marshals positional args into JSON {file, scenario, tags}
  and forwards to commands::remove_tag_from_scenario::run — NO domain logic in the bridge.
  Help fixture captured from `node dist/index.js remove-tag-from-scenario --help`.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want the standalone Rust fspec binary's remove-tag-from-scenario subcommand to parse the same positional arguments as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec remove-tag-from-scenario --help`
    Then the exit code is 0
    And the stdout matches the canonical help fixture at rust/fspec/tests/fixtures/help/remove-tag-from-scenario.txt
    And stdout starts with a blank line followed by 'REMOVE-TAG-FROM-SCENARIO'

  Scenario: CLI successfully removes a tag and prints the success line
    Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' tagged @smoke @critical
    When I run `fspec remove-tag-from-scenario spec/features/login.feature "Login" @critical` in that tempdir
    Then the exit code is 0
    And stdout contains the substring "✓ Removed @critical from scenario 'Login'"
    And spec/features/login.feature on disk shows the Login scenario tagged @smoke

  Scenario: CLI variadic positional collects multiple tags
    Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' tagged @smoke @critical @wip
    When I run `fspec remove-tag-from-scenario spec/features/login.feature "Login" @critical @wip` in that tempdir
    Then the exit code is 0
    And stdout contains the substring "✓ Removed @critical, @wip from scenario 'Login'"
    And spec/features/login.feature on disk shows the Login scenario tagged @smoke

  Scenario: CLI idempotent path for non-matching tags
    Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' tagged @smoke
    When I run `fspec remove-tag-from-scenario spec/features/login.feature "Login" @critical` in that tempdir
    Then the exit code is 0
    And stdout contains the substring "No changes made - none of the specified tags found on scenario 'Login'"
    And spec/features/login.feature on disk is byte-equal to its pre-call contents

  Scenario: CLI reports missing feature file with exit 1
    Given an empty project root directory with no spec/features/missing.feature
    When I run `fspec remove-tag-from-scenario spec/features/missing.feature "Login" @smoke` in that tempdir
    Then the exit code is 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'File not found: spec/features/missing.feature'

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' tagged @smoke @critical
    When I dispatch remove-tag-from-scenario via fspec_core::dispatch::dispatch_command with file='spec/features/login.feature' scenario='Login' tags=['@smoke']
    Then the dispatcher returns success=true
    And running `fspec remove-tag-from-scenario spec/features/login.feature "Login" @critical` afterwards exits 0
    And spec/features/login.feature on disk shows the Login scenario with no tag lines immediately above it
    And the CLI bridge module rust/fspec/src/remove_tag_from_scenario.rs contains NO inline scenario lookup, line-walk filter, or file-write logic — its only computation is JSON arg marshalling
