@validation
@validator
@rust
@wip
@RPC-322
Feature: Port validate-hooks command to Rust
  """
  File layout: core impl rust/fspec-core/src/commands/validate_hooks.rs (rewrite stub); help config rust/fspec-core/src/help/configs/validate_hooks.rs; CLI bridge rust/fspec/src/validate_hooks.rs; core test rust/fspec-core/tests/validate_hooks.rs; CLI test rust/fspec/tests/cli_validate_hooks.rs; help fixture rust/fspec/tests/fixtures/help/validate-hooks.txt
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

  Scenario: Dispatcher reports all hooks valid when every script exists
    Given spec/fspec-hooks.json configures one hook whose command script exists on disk
    When I dispatch the validate-hooks command against that project root
    Then the dispatcher returns success=true
    Then the result is valid with message '✓ All hooks are valid' and exitCode 0

  Scenario: Dispatcher reports a missing hook script
    Given spec/fspec-hooks.json configures a hook with command 'spec/hooks/lint.sh' that does not exist on disk
    When I dispatch the validate-hooks command against that project root
    Then the result is invalid with exitCode 1
    Then the message contains '✗ Hook validation failed'
    Then the message contains 'Hook command not found: spec/hooks/lint.sh'

  Scenario: Dispatcher reports no hooks configured for an empty hooks object
    Given spec/fspec-hooks.json exists with an empty hooks object
    When I dispatch the validate-hooks command against that project root
    Then the dispatcher returns success=true
    Then the message is 'No hooks configured (nothing to validate)' with exitCode 0

  Scenario: Dispatcher reports a load failure when the config is missing
    Given an empty project root with no spec/fspec-hooks.json
    When I dispatch the validate-hooks command against that project root
    Then the result is invalid with exitCode 1
    Then the message is 'Failed to load hook configuration'

  Scenario: Dispatcher reports a load failure when the config is malformed JSON
    Given spec/fspec-hooks.json exists but contains the malformed bytes '{ not json'
    When I dispatch the validate-hooks command against that project root
    Then the result is invalid with exitCode 1
    Then the message is 'Failed to load hook configuration'

  Scenario: Dispatcher lists every missing script across multiple events
    Given spec/fspec-hooks.json configures two hooks under different events whose command scripts are both missing
    When I dispatch the validate-hooks command against that project root
    Then the result is invalid with exitCode 1
    Then the message contains a 'Hook command not found' line for each missing script
