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
  #
  # EXAMPLES:
  #   1. AI runs 'fspec update-work-unit-status AUTH-001 validating' with no config, sees system-reminder: 'NO TEST COMMAND CONFIGURED'
  #   2. AI runs 'fspec update-work-unit-status AUTH-001 validating' with config present, sees system-reminder: 'Run tests: npm test'
  #   3. AI receives reminder, runs 'fspec configure-tools --test-command npm test', then validation emits actual command
  #   4. Integration test verifies checkTestCommand and checkQualityCommands are called during status transition
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
    As a AI agent validating work units
    I want to receive system-reminders about test and quality check commands
    So that I know exactly what commands to run without guessing

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
