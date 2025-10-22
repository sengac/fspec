@done
@feature-management
@initialization
@cli
@medium
@INIT-010
Feature: Agent switching prompt in fspec init
  """
  Reuses agent detection logic from removeInitFiles (detectAgents + readAgentConfig). Uses existing AgentSelector React component for interactive prompts with pre-selection support. Integrates into init.ts installAgents() flow. Calls removeOtherAgentFiles() for cleanup before installation. Updates spec/fspec-config.json via writeAgentConfig(). No breaking changes to existing fspec init behavior.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Detect current agent using BOTH spec/fspec-config.json AND file detection (like remove-init-files does), then prompt only if the new agent being installed is different from the detected agent
  #   2. Two options: 'Switch to [NewAgent]' (removes old agent files, installs new agent) or 'Cancel' (keeps existing setup, aborts installation)
  #   3. Update spec/fspec-config.json to reflect the new agent ID (e.g., from {"agent": "claude"} to {"agent": "cursor"}) so runtime detection works correctly
  #   4. Pre-select the detected agent in interactive selector. If user chooses the same agent, reinstall/refresh files (no prompt). If user chooses a different agent, show switch prompt ('Switch from Claude to Cursor?')
  #
  # EXAMPLES:
  #   1. User runs 'fspec init --agent=cursor', Claude files detected, prompt shows 'Switch from Claude to Cursor?', user selects 'Switch', Claude files removed, Cursor files installed, config updated to cursor
  #   2. User runs 'fspec init --agent=cursor', Claude files detected, prompt shows 'Switch from Claude to Cursor?', user selects 'Cancel', Claude files remain, Cursor not installed, exit code 0
  #   3. User runs 'fspec init' (interactive), Claude files detected and pre-selected, user selects Cursor instead, prompt shows 'Switch from Claude to Cursor?', user confirms, switch happens
  #   4. User runs 'fspec init' (interactive), Claude files detected and pre-selected, user presses Enter (keeps Claude), files reinstalled/refreshed without prompt
  #   5. User runs 'fspec init --agent=claude', Claude files already exist, no prompt shown (same agent), files reinstalled/refreshed (idempotent behavior)
  #
  # QUESTIONS (ANSWERED):
  #   Q: When should the agent switching prompt appear - always, or only when installing a different agent than currently installed?
  #   A: true
  #
  #   Q: What options should the agent switching prompt offer to the user?
  #   A: true
  #
  #   Q: Should spec/fspec-config.json be deleted, updated, or left unchanged when switching agents?
  #   A: true
  #
  #   Q: What happens in interactive mode when an existing agent is detected?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer switching between AI coding agents
    I want to be prompted to switch agents when running fspec init with existing agent files
    So that I can easily switch from one agent to another without manual cleanup

  Scenario: Switch agents when different agent requested via CLI
    Given I have fspec initialized with Claude agent
    Given spec/fspec-config.json contains agent 'claude'
    When I run 'fspec init --agent=cursor'
    Then the prompt should ask 'Switch from Claude to Cursor?'
    When I select 'Switch to Cursor'
    Then Claude files should be removed
    Then Cursor files should be installed
    Then spec/fspec-config.json should contain agent 'cursor'

  Scenario: Cancel agent switch and keep existing setup
    Given I have fspec initialized with Claude agent
    When I run 'fspec init --agent=cursor'
    When the prompt asks 'Switch from Claude to Cursor?'
    When I select 'Cancel'
    Then Claude files should remain unchanged
    Then Cursor files should not be installed
    Then the command should exit with code 0

  Scenario: Switch agents in interactive mode
    Given I have fspec initialized with Claude agent
    When I run 'fspec init' without --agent flag
    Then the interactive selector should pre-select Claude
    When I select Cursor from the list
    Then the prompt should ask 'Switch from Claude to Cursor?'
    When I confirm the switch
    Then Claude files should be removed and Cursor files installed

  Scenario: Keep same agent in interactive mode without prompt
    Given I have fspec initialized with Claude agent
    When I run 'fspec init' without --agent flag
    Then the interactive selector should pre-select Claude
    When I press Enter to confirm Claude
    Then no switch prompt should appear
    Then Claude files should be reinstalled/refreshed

  Scenario: Reinstall same agent without prompt
    Given I have fspec initialized with Claude agent
    When I run 'fspec init --agent=claude'
    Then no switch prompt should appear
    Then Claude files should be reinstalled/refreshed (idempotent behavior)
    Then the command should exit successfully
