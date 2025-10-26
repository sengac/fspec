@cli
@initialization
@bug-fix
@feature-management
@BUG-034
Feature: Double prompt and missing success message in interactive init mode
  """
  Fix 1: Pass promptAgentSwitch parameter to executeInit() in interactive mode that returns true (auto-confirm). This skips the second prompt while preserving CLI mode behavior.
  Fix 2: Change success message condition from 'if (options.agent.length > 0)' to 'if (result.success)' so both interactive and CLI modes show feedback.
  Files to modify: src/commands/init.ts (action handler lines 366 and 375). NO changes needed to executeInit(), installAgents(), or AgentSelector component.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Interactive mode MUST NOT show double prompts (AgentSelector selection IS user confirmation)
  #   2. Interactive mode MUST show success message with file list and activation instructions
  #   3. CLI mode (--agent flag) SHOULD still show agent switch confirmation prompt
  #   4. Both interactive and CLI modes MUST show identical success messages
  #
  # EXAMPLES:
  #   1. User runs 'fspec init', selects Cursor from menu, sees NO second confirmation prompt, sees success message with file list
  #   2. User runs 'fspec init --agent=cursor' when Claude installed, sees 'Switch from Claude to Cursor?' prompt, confirms, sees success message
  #   3. User runs 'fspec init', selects same agent (Claude), sees NO prompt, sees success message showing reinstallation
  #   4. Success message shows: checkmark, agent name, indented file list, activation instructions - identical format for both modes
  #
  # ========================================
  Background: User Story
    As a developer using fspec init
    I want to select an agent and see clear confirmation
    So that I know the installation succeeded and what to do next

  Scenario: Interactive mode skips second confirmation prompt when switching agents
    Given I have fspec initialized with Claude agent
    And spec/fspec-config.json contains agent 'claude'
    When I run 'fspec init' in interactive mode
    And I select 'Cursor' from the agent menu
    Then no agent switch confirmation prompt should appear
    And Cursor should be installed successfully
    And I should see success message with checkmark
    And I should see detailed file list showing installed files
    And I should see activation instructions

  Scenario: CLI mode still shows agent switch confirmation prompt
    Given I have fspec initialized with Claude agent
    And spec/fspec-config.json contains agent 'claude'
    When I run 'fspec init --agent=cursor'
    Then I should see prompt 'Switch from Claude to Cursor?'
    When I confirm the switch
    Then Cursor should be installed successfully
    And I should see success message with checkmark
    And I should see detailed file list showing installed files
    And I should see activation instructions

  Scenario: Interactive mode shows success message for same agent reinstall
    Given I have fspec initialized with Claude agent
    And spec/fspec-config.json contains agent 'claude'
    When I run 'fspec init' in interactive mode
    And I select 'Claude' from the agent menu
    Then no agent switch prompt should appear
    And Claude should be reinstalled successfully
    And I should see success message with checkmark
    And I should see detailed file list showing reinstalled files
    And I should see activation instructions

  Scenario: Success message format is identical in both interactive and CLI modes
    Given I have fspec initialized with Claude agent
    When I install Cursor in interactive mode
    And I install Windsurf in CLI mode with --agent flag
    Then both success messages should have identical format
    And both should show checkmark symbol
    And both should show agent name
    And both should show indented file list with bullet points
    And both should show activation instructions
