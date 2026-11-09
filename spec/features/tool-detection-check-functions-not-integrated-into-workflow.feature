@done
@validation
@setup
@critical
@integration
@configuration
@cli
@bug
@CONFIG-003
Feature: Tool detection check functions not integrated into workflow
  """
  Integration point: rust/fspec-core/src/commands/update_work_unit_status/output.rs prints the configure-tools test command check and quality commands check (update_work_unit_status/reminders/configure_tools.rs) before the confirmation line whenever a work unit moves to validating. The checks read spec/fspec-config.json (tools.test.command, tools.qualityCheck.commands chained with &&) and are wrapped for the agent from the config (system-reminder for agents that support them). sync-version runs the same checks; its scenarios live in automatic-version-check-and-update-for-slash-command-files. configure-tools saves the config only: agent templates are not regenerated (product owner, 2026-11-06, H-CONFIGURE-TOOLS A); sync-version reinstalls the agent files on a version change.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Check functions MUST be called during status transitions to 'validating' state
  #   2. System-reminders MUST be emitted when config is missing to guide AI to run fspec configure-tools
  #   3. System-reminders MUST be emitted when config exists to tell AI what commands to run
  #   4. Integration MUST use existing checkTestCommand and checkQualityCommands functions without modification
  #   5. sync-version MUST fail (exit 1) when tool configuration is missing to prevent AI from continuing workflow without proper setup
  #   6. sync-version MUST fail (exit 1) when version is incorrect to prevent AI from continuing with wrong version
  #   7. sync-version MUST emit system-reminder about configure-tools when tool configuration is completely missing (file does not exist)
  #   8. configure-tools MUST regenerate agent templates (spec/AGENTS.md, .claude/commands/fspec.md, etc.) after updating config to ensure templates reflect latest version
  #   9. configure-tools MUST regenerate templates silently (no output to user about template regeneration) to avoid cluttering the output with implementation details
  #
  # EXAMPLES:
  #   1. AI runs 'fspec update-work-unit-status AUTH-001 validating' with no config, sees system-reminder: 'NO TEST COMMAND CONFIGURED'
  #   2. AI runs 'fspec update-work-unit-status AUTH-001 validating' with config present, sees system-reminder: 'Run tests: npm test'
  #   3. AI receives reminder, runs 'fspec configure-tools --test-command npm test', then validation emits actual command
  #   4. Integration test verifies checkTestCommand and checkQualityCommands are called during status transition
  #   5. AI runs 'fspec --sync-version 0.6.0' with no tool config, sees system-reminder, command exits with code 1, AI cannot continue until tools are configured
  #   6. AI runs 'fspec --sync-version 0.5.0' when actual version is 0.6.0, sees version mismatch error, command exits with code 1, AI cannot continue until correct version used
  #   7. AI runs 'fspec --sync-version 0.6.0' when spec/fspec-config.json does not exist, sees system-reminder to run configure-tools, command exits with code 1, AI runs configure-tools then re-runs sync-version
  #   8. AI runs 'fspec configure-tools --test-command npm test', command saves config AND regenerates spec/AGENTS.md and .claude/commands/fspec.md with latest templates, ensuring AI has up-to-date documentation
  #   9. AI runs 'fspec configure-tools --test-command npm test', sees only '✓ Tool configuration saved to spec/fspec-config.json' output, templates regenerated silently in background without additional messages
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should --sync-version also emit tool configuration checks to help onboard new AI agents?
  #   A: true
  #
  # ASSUMPTIONS:
  #   1. Yes - sync-version should emit tool config checks. This helps onboard new AI agents by guiding them to configure tools immediately after version update, ensuring they have complete workflow setup.
  #
  # ========================================
  Background: User Story
    As a AI agent using fspec for ACDD workflow
    I want to receive automatic guidance about tool configuration and version sync
    So that I can complete proper workflow setup without manual intervention and avoid continuing with incorrect configuration

  Scenario: Emit system-reminder when moving to validating with no config
    Given spec/fspec-config.json sets agent 'claude' but has no tools.test.command configured
    And work unit AUTH-001 is in implementing
    When work unit AUTH-001 moves to validating through update-work-unit-status
    Then the status change succeeds
    And system-reminder should be emitted to console output
    And system-reminder should contain text: 'NO TEST COMMAND CONFIGURED'
    And system-reminder should tell AI to run: fspec configure-tools --test-command <cmd>

  Scenario: Emit system-reminder when moving to validating with config present
    Given spec/fspec-config.json sets agent 'claude' and has tools.test.command = 'npm test'
    And work unit AUTH-001 is in implementing
    When work unit AUTH-001 moves to validating through update-work-unit-status
    Then the status change succeeds
    And system-reminder should be emitted to console output
    And system-reminder should contain text: 'RUN TESTS'
    And system-reminder should contain text: 'Run tests: npm test'
    And the output should not prompt the AI to configure tools

  Scenario: Emit quality check reminder when moving to validating
    Given spec/fspec-config.json sets agent 'claude' and has tools.qualityCheck.commands = ['eslint .', 'prettier --check .']
    And work unit AUTH-001 is in implementing
    When work unit AUTH-001 moves to validating through update-work-unit-status
    Then the status change succeeds
    And system-reminder should be emitted to console output
    And system-reminder should contain text: 'RUN QUALITY CHECKS'
    And system-reminder should contain chained command: 'eslint . && prettier --check .'

  Scenario: Chain multiple test frameworks when AI provides chained command
    Given AI ran configure-tools with the test command 'cargo test && npm test'
    And work unit AUTH-001 is in implementing
    When work unit AUTH-001 moves to validating through update-work-unit-status
    Then spec/fspec-config.json should store tools.test.command = 'cargo test && npm test'
    And the output should contain: 'Run tests: cargo test && npm test'

  Scenario: Include date-aware search queries when no tools detected
    Given spec/fspec-config.json does not exist
    And work unit AUTH-001 is in implementing
    When work unit AUTH-001 moves to validating through update-work-unit-status
    Then the reminder should include the search query 'best <platform> testing tools' followed by a four-digit year
    And the reminder should tell the AI to replace <platform> with the detected project type
