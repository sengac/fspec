@validation
@cli
@rust
@wip
@RPC-322
Feature: Validate-hooks CLI subcommand

  """
  File layout: core impl codelet/fspec-core/src/commands/validate_hooks.rs (rewrite stub); help config codelet/fspec-core/src/help/configs/validate_hooks.rs; CLI bridge codelet/fspec/src/validate_hooks.rs; core test codelet/fspec-core/tests/validate_hooks.rs; CLI test codelet/fspec/tests/cli_validate_hooks.rs; help fixture codelet/fspec/tests/fixtures/help/validate-hooks.txt
  FRAMING A: the TS shell validate-hooks action awaits validateHooks but DISCARDS the result (prints nothing, never calls process.exit) — the broken-CLI pattern. Rust implements the help-doc canon (print status + meaningful exit code), mirroring the RPC-247 list-hooks precedent. DESIGN: core run reads spec/fspec-hooks.json as raw serde_json::Value (only hook.command strings needed; no new shared type). Map empty hooks -> 'No hooks configured (nothing to validate)' exit 0; missing/invalid file -> 'Failed to load hook configuration' exit 1; missing scripts -> '✗ Hook validation failed' block exit 1; all good -> '✓ All hooks are valid' exit 0. PROPOSAL for supervisor: run returns JSON {valid, exitCode, message} so CLI bridge prints message + uses exitCode (RPC-247 precedent); confirm. Supervisor wires canonical.rs, dispatch.rs, help/configs/mod.rs, main.rs Mode+intercept+forward. No new io/ensure helper required.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Read spec/fspec-hooks.json (no auto-create). For each event's hook list, resolve hook.command relative to project root and check it exists; each missing script yields 'Hook command not found: <hook.command>'
  #   2. If the config file is missing or contains invalid JSON, validation fails with the message 'Failed to load hook configuration' and a non-zero exit code (catch-all branch, parity with validateHooks)
  #   3. Framing A: the TS shell action discards the validateHooks result (prints nothing, always exits 0). The Rust port implements the HELP-DOC CANON instead: all scripts found -> '✓ All hooks are valid' exit 0; missing scripts -> '✗ Hook validation failed' followed by each 'Hook command not found' line and 'Fix these issues before using hooks.' exit 1; config with no configured hooks -> 'No hooks configured (nothing to validate)' exit 0
  #   4. Two front doors: clap subcommand exposes NO flags; both dispatcher and CLI call fspec_core::commands::validate_hooks::run. --help is byte-for-byte identical to TS formatCommandHelp (custom validate-hooks-help.ts -> dedicated help config module)
  #
  # EXAMPLES:
  #   1. A config whose hooks all point to existing scripts validates as '✓ All hooks are valid' and exits 0
  #   2. A config referencing spec/hooks/lint.sh that does not exist reports '✗ Hook validation failed' and 'Hook command not found: spec/hooks/lint.sh' and exits 1
  #   3. A config with an empty hooks object reports 'No hooks configured (nothing to validate)' and exits 0
  #   4. Running validate-hooks when spec/fspec-hooks.json does not exist reports 'Failed to load hook configuration' and exits 1
  #
  # ========================================

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to run `fspec validate-hooks` to confirm that every hook script referenced in spec/fspec-hooks.json exists on disk
    So that I can trust my hook configuration before relying on hooks for workflow automation

  Scenario: validate-hooks --help is byte-for-byte identical to the TS reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec validate-hooks --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/validate-hooks.txt
    And stdout starts with a blank line followed by 'VALIDATE-HOOKS'

  Scenario: CLI prints success and exits 0 when all hook scripts exist
    Given spec/fspec-hooks.json configures one hook whose command script exists on disk
    When I run `./codelet/target/release/fspec validate-hooks`
    Then the command exits 0
    Then stdout contains the substring '✓ All hooks are valid'

  Scenario: CLI reports missing scripts and exits 1
    Given spec/fspec-hooks.json configures a hook with command 'spec/hooks/lint.sh' that does not exist on disk
    When I run `./codelet/target/release/fspec validate-hooks`
    Then the command exits with code 1
    Then stdout contains the substring '✗ Hook validation failed'
    Then stdout contains the substring 'Hook command not found: spec/hooks/lint.sh'

  Scenario: CLI reports a load failure and exits 1 when the config is missing
    Given an empty directory with no spec/fspec-hooks.json is set as the working directory
    When I run `./codelet/target/release/fspec validate-hooks`
    Then the command exits with code 1
    Then stdout contains the substring 'Failed to load hook configuration'

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root whose spec/fspec-hooks.json references a missing hook script
    When I dispatch validate-hooks through fspec_core::dispatch::dispatch_command and also run `./codelet/target/release/fspec validate-hooks` against the same on-disk state
    Then both paths agree the configuration is invalid
    Then the CLI bridge module codelet/fspec/src/validate_hooks.rs contains NO inline validation logic — its only computation is JSON arg marshalling
