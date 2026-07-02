@done
@RPC-194
Feature: fspec add-tag-to-scenario CLI subcommand
  """
  CLI bridge: codelet/fspec/src/add_tag_to_scenario.rs — clap-derived struct mirroring the TS
  Commander.js registration (src/commands/add-tag-to-scenario.ts:261-282). Surface:
  `fspec add-tag-to-scenario <file> <scenario> <tags...> [--validate-registry]`.
  Stdout (success): '✓ Added <tags> to scenario \'<name>\'' (TS uses output.log; ANSI tolerated
  via substring match).
  Stderr (failure): 'Error: <message>'; exit code 1.
  Two-front-doors invariant: bridge marshals positional + flag args into JSON {file, scenario,
  tags, validateRegistry} and forwards to commands::add_tag_to_scenario::run — NO domain logic
  in the bridge.
  Help fixture captured from `node dist/index.js add-tag-to-scenario --help`.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want the standalone Rust fspec binary's add-tag-to-scenario subcommand to parse the same positional arguments as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec add-tag-to-scenario --help`
    Then the exit code is 0
    And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/add-tag-to-scenario.txt
    And stdout starts with a blank line followed by 'ADD-TAG-TO-SCENARIO'

  Scenario: CLI successfully adds a tag and prints the success line
    Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' with no tags
    When I run `fspec add-tag-to-scenario spec/features/login.feature "Login" @smoke` in that tempdir
    Then the exit code is 0
    And stdout contains the substring "✓ Added @smoke to scenario 'Login'"
    And spec/features/login.feature on disk shows a single '  @smoke' line immediately above the Scenario line

  Scenario: CLI rejects duplicate tag with exit 1 and TS-parity error prefix
    Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' tagged @smoke
    When I run `fspec add-tag-to-scenario spec/features/login.feature "Login" @smoke` in that tempdir
    Then the exit code is 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Tag @smoke already exists on this scenario'
    And spec/features/login.feature on disk is byte-equal to its pre-call contents

  Scenario: CLI variadic positional collects multiple tags
    Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' with no tags
    When I run `fspec add-tag-to-scenario spec/features/login.feature "Login" @critical @regression` in that tempdir
    Then the exit code is 0
    And stdout contains the substring "✓ Added @critical, @regression to scenario 'Login'"
    And spec/features/login.feature on disk shows '  @critical' then '  @regression' immediately above the Scenario line

  Scenario: CLI --validate-registry rejects unregistered tag
    Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' with no tags and spec/tags.json that does NOT register @unregistered
    When I run `fspec add-tag-to-scenario spec/features/login.feature "Login" @unregistered --validate-registry` in that tempdir
    Then the exit code is 1
    And stderr contains the substring '@unregistered is not registered in spec/tags.json'
    And spec/features/login.feature on disk is byte-equal to its pre-call contents

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' with no tags
    When I dispatch add-tag-to-scenario via fspec_core::dispatch::dispatch_command with file='spec/features/login.feature' scenario='Login' tags=['@smoke']
    Then the dispatcher returns success=true
    And running `fspec add-tag-to-scenario spec/features/login.feature "Login" @critical` afterwards exits 0
    And spec/features/login.feature on disk shows '  @smoke' then '  @critical' immediately above the Scenario line
    And the CLI bridge module codelet/fspec/src/add_tag_to_scenario.rs contains NO inline tag-format validation, scenario lookup, or file-write logic — its only computation is JSON arg marshalling
