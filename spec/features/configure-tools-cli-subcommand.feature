@done
@configuration
@cli
@RPC-208
Feature: fspec configure-tools CLI subcommand
  """
  CLI bridge: codelet/fspec/src/configure_tools.rs — clap-derived struct mirroring the TS
  Commander.js registration (src/commands/configure-tools.ts:193-243). Surface:
  `fspec configure-tools [--test-command <command>] [--quality-commands <commands...>] [--reconfigure]`.
  The bridge marshals args into JSON {testCommand?, qualityCommands?, reconfigure?} (omitting None)
  and forwards to fspec_core commands::configure_tools::run — NO domain logic in the bridge.

  Stdout (non-reconfigure success): '✓ Tool configuration saved to spec/fspec-config.json'
  (TS output.log; ANSI tolerated via substring match); exit code 0. The reconfigure path prints the
  returned RECONFIGURE TOOLS reminder message instead of the saved-confirmation line.
  DIVERGENCE: silent agent-template regeneration (installAgentFiles via init) is deferred because
  init is not yet ported. Help fixture captured from `node dist/index.js configure-tools --help`.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want the standalone Rust fspec binary's configure-tools subcommand to parse the same flags as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven tool-configuration script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec configure-tools --help`
    Then the exit code is 0
    And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/configure-tools.txt

  Scenario: CLI saves the test command and prints the confirmation line
    Given a project root tempdir with no spec/fspec-config.json
    When I run `fspec configure-tools --test-command "cargo test"` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Tool configuration saved to spec/fspec-config.json'
    And spec/fspec-config.json shows tools.test.command='cargo test'

  Scenario: CLI forwards multi-value quality commands into the persisted array
    Given a project root tempdir with no spec/fspec-config.json
    When I run `fspec configure-tools --test-command "npm test" --quality-commands "eslint ." "prettier --check ."` in that tempdir
    Then the exit code is 0
    And spec/fspec-config.json shows tools.qualityCheck.commands=['eslint .','prettier --check .']

  Scenario: CLI --reconfigure does not write the config file
    Given a project root tempdir with no spec/fspec-config.json
    When I run `fspec configure-tools --reconfigure` in that tempdir
    Then the exit code is 0
    And stdout is empty (TS Commander action discards the reconfigure return value; the RECONFIGURE TOOLS guidance surfaces only via the LLM dispatcher)
    And spec/fspec-config.json does not exist on disk

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with no spec/fspec-config.json
    When I dispatch configure-tools via fspec_core::dispatch::dispatch_command with testCommand='via-dispatcher'
    Then spec/fspec-config.json shows tools.test.command='via-dispatcher'
    And running `fspec configure-tools --quality-commands "via-cli"` afterwards exits 0
    And spec/fspec-config.json still shows tools.test.command='via-dispatcher' and tools.qualityCheck.commands=['via-cli']
    And the CLI bridge module codelet/fspec/src/configure_tools.rs contains NO inline config-merge or file-write logic — its only computation is JSON arg marshalling
