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
  Integration point: src/commands/update-work-unit-status.ts must import and call checkTestCommand and checkQualityCommands when status changes to 'validating'. Functions exist in src/commands/configure-tools.ts (lines 32-127). Must use formatAgentOutput for system-reminder wrapping. No modifications to check functions themselves - pure integration.
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
  #   8. configure-tools MUST regenerate agent templates (spec/CLAUDE.md, .claude/commands/fspec.md, etc.) after updating config to ensure templates reflect latest version
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
  #   8. AI runs 'fspec configure-tools --test-command npm test', command saves config AND regenerates spec/CLAUDE.md and .claude/commands/fspec.md with latest templates, ensuring AI has up-to-date documentation
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
    Given spec/fspec-config.json does not have tools.test.command configured
    When AI runs 'fspec update-work-unit-status AUTH-001 validating'
    Then update-work-unit-status command should call checkTestCommand function
    And system-reminder should be emitted to console output
    And system-reminder should contain text: 'NO TEST COMMAND CONFIGURED'
    And system-reminder should tell AI to run: fspec configure-tools --test-command <cmd>

  Scenario: Emit system-reminder when moving to validating with config present
    Given spec/fspec-config.json has tools.test.command = 'npm test'
    When AI runs 'fspec update-work-unit-status AUTH-001 validating'
    Then update-work-unit-status command should call checkTestCommand function
    And system-reminder should be emitted to console output
    And system-reminder should contain text: 'RUN TESTS'
    And system-reminder should contain text: 'Run tests: npm test'

  Scenario: Emit quality check reminder when moving to validating
    Given spec/fspec-config.json has tools.qualityCheck.commands = ['eslint .', 'prettier --check .']
    When AI runs 'fspec update-work-unit-status AUTH-001 validating'
    Then update-work-unit-status command should call checkQualityCommands function
    And system-reminder should be emitted to console output
    And system-reminder should contain text: 'RUN QUALITY CHECKS'
    And system-reminder should contain chained command: 'eslint . && prettier --check .'

  Scenario: Integration test verifies check functions are called
    Given integration test for update-work-unit-status command
    When test transitions work unit to 'validating' status
    Then test should spy on checkTestCommand function
    And test should spy on checkQualityCommands function
    And test should verify both functions were called with correct cwd argument
    And test should verify console.log was called with system-reminder output

  Scenario: Fail sync-version when tool configuration is missing
    Given versions match (embedded version equals package.json version)
    And spec/fspec-config.json does not have tools.test.command configured
    When AI runs 'fspec --sync-version <current-version>'
    Then sync-version should call checkTestCommand function
    And system-reminder should be emitted: 'NO TEST COMMAND CONFIGURED'
    And command should exit with code 1 (failure)
    And AI agent workflow should stop (cannot continue without tool setup)

  Scenario: Fail sync-version when version is incorrect
    Given spec/fspec-config.json has tools.test.command configured
    And embedded version is 0.6.0 but AI provides 0.5.0
    When AI runs 'fspec --sync-version 0.5.0'
    Then command should display version mismatch error
    And error should show expected version: 0.6.0
    And error should show provided version: 0.5.0
    And command should exit with code 1 (failure)
    And AI agent workflow should stop (cannot continue with wrong version)

  Scenario: Emit system-reminder when config file completely missing during sync-version
    Given versions match (embedded version equals package.json version)
    And spec/fspec-config.json file does not exist at all
    When AI runs 'fspec --sync-version <current-version>'
    Then sync-version should call checkTestCommand function
    And system-reminder should be emitted: 'NO TEST COMMAND CONFIGURED'
    And system-reminder should guide AI to run: 'fspec configure-tools --test-command <cmd>'
    And command should exit with code 1 (failure)

  Scenario: configure-tools regenerates agent templates after updating config
    Given spec/fspec-config.json exists with agent = 'claude'
    And spec/CLAUDE.md exists (old template)
    And .claude/commands/fspec.md exists (old template)
    When AI runs 'fspec configure-tools --test-command "npm test"'
    Then config should be updated with tools.test.command = "npm test"
    And spec/CLAUDE.md should be regenerated with latest template
    And .claude/commands/fspec.md should be regenerated with latest template
    And templates should reflect current fspec version

  Scenario: configure-tools regenerates templates silently without extra output
    Given spec/fspec-config.json exists
    When AI runs 'fspec configure-tools --test-command "npm test"'
    Then console output should contain: '✓ Tool configuration saved to spec/fspec-config.json'
    And console output should NOT contain: 'Regenerating templates'
    And console output should NOT contain: '✓ Templates updated'
    And console output should NOT mention spec/CLAUDE.md
    And console output should NOT mention .claude/commands/fspec.md
    But templates should be regenerated in the background
