@done
@configuration
@cli
@rust
@RPC-208
Feature: Port configure-tools command to Rust

  """
  Core impl at codelet/fspec-core/src/commands/configure_tools.rs. Reference port: add_command_to_foundation.rs (read_or_init_json + serde_json::Value mutate + write_json_atomic). Config path = spec/fspec-config.json. read-modify-write: load existing or init {agent:'claude'}; ensure .tools object; set tools.test={command} and/or tools.qualityCheck={commands} only for provided flags; write_json_atomic (2-space, no trailing newline).
  CLI bridge codelet/fspec/src/configure_tools.rs: clap struct with --test-command <command>, --quality-commands <commands...> (multi-value), --reconfigure flag. Marshals JSON {testCommand?, qualityCommands?, reconfigure?} omitting None. Non-reconfigure success prints '✓ Tool configuration saved to spec/fspec-config.json'. Reconfigure prints the returned system-reminder message. No domain logic in bridge.
  Reconfigure branch returns CheckResult {type:'system-reminder', message}. TS QUIRK to reproduce bug-for-bug: the reconfigure branch calls formatAgentOutput(cwd, ...) passing the cwd STRING where an AgentConfig is expected, so message is NOT wrapped in system-reminder tags and falls through to the plain prefixed text branch. The dispatcher returns this message; the non-reconfigure path returns void and the CLI prints the saved-confirmation line.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. configure-tools writes test/quality-check commands into spec/fspec-config.json under tools.test.command and tools.qualityCheck.commands
  #   2. The spec/ directory is created if missing before writing the config file
  #   3. Config is read-modify-write: an existing fspec-config.json is loaded and merged (preserving agent and unknown fields); a missing file starts from {agent:'claude'}
  #   4. The test-command option sets tools.test={command}; the quality-commands option sets tools.qualityCheck={commands:[...]}; each is applied only when provided
  #   5. The reconfigure flag short-circuits: it returns a RECONFIGURE TOOLS system-reminder WITHOUT writing the config and WITHOUT regenerating templates
  #   6. The config file is written with 2-space indent (JSON.stringify(config,null,2)) and no trailing newline, preserving the field order agent then tools
  #   7. On non-reconfigure success the CLI prints '✓ Tool configuration saved to spec/fspec-config.json'; exit code 0
  #   8. DIVERGENCE: TS regenerates agent templates silently (installAgentFiles via init) after a successful write; the Rust port defers this template-regeneration side effect because init/installAgentFiles is not yet ported. Config write parity is preserved.
  #   9. Two-front-doors: CLI bridge marshals JSON {testCommand?, qualityCommands?, reconfigure?} (omitting None) only; both dispatcher and standalone binary converge on commands::configure_tools::run
  #
  # EXAMPLES:
  #   1. Running configure-tools --test-command "cargo test" in a fresh project writes spec/fspec-config.json with tools.test.command='cargo test' and prints the saved confirmation
  #   2. Running configure-tools --test-command "npm test" --quality-commands "eslint ." "prettier --check ." writes tools.test.command and tools.qualityCheck.commands=[eslint .,prettier --check .]
  #   3. Running configure-tools --reconfigure returns the RECONFIGURE TOOLS system-reminder and does not create or modify spec/fspec-config.json
  #   4. Running configure-tools --quality-commands twice: a second run with --test-command preserves the previously stored qualityCheck.commands via read-modify-write
  #
  # ========================================

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want to port the configure-tools command to Rust as a parity port
    So that the standalone Rust binary and the dispatcher can both persist test and quality-check commands to spec/fspec-config.json without falling back to TypeScript

  Scenario: Setting only the test command writes it under tools.test.command in a fresh project
    Given a project root tempdir with no spec/fspec-config.json
    When I dispatch configure-tools with testCommand='cargo test'
    Then spec/fspec-config.json exists on disk
    And spec/fspec-config.json shows tools.test.command='cargo test'
    And spec/fspec-config.json shows agent='claude'

  Scenario: Setting both test command and quality commands persists both arrays
    Given a project root tempdir with no spec/fspec-config.json
    When I dispatch configure-tools with testCommand='npm test' and qualityCommands=['eslint .','prettier --check .']
    Then spec/fspec-config.json shows tools.test.command='npm test'
    And spec/fspec-config.json shows tools.qualityCheck.commands=['eslint .','prettier --check .']

  Scenario: The reconfigure flag short-circuits without writing the config
    Given a project root tempdir with no spec/fspec-config.json
    When I dispatch configure-tools with reconfigure=true
    Then the dispatcher result contains the substring 'RECONFIGURE TOOLS'
    And spec/fspec-config.json does not exist on disk

  Scenario: A second run preserves previously stored quality commands via read-modify-write
    Given a project root tempdir whose spec/fspec-config.json already has tools.qualityCheck.commands=['eslint .']
    When I dispatch configure-tools with testCommand='npm test'
    Then spec/fspec-config.json shows tools.test.command='npm test'
    And spec/fspec-config.json still shows tools.qualityCheck.commands=['eslint .']
