@feature-management
@cli
@initialization
@bug-fix
@wip
@BUG-033
Feature: Incomplete implementation of INIT-009 and INIT-010 features
  """
  Implementation requires: 1) Create src/components/ConfirmPrompt.tsx (reusable Yes/No prompt), 2) Update remove-init-files action handler to use ConfirmPrompt, 3) Update init action handler to detect existing agent and show switch prompt, 4) Remove duplicate writeAgentConfig call in init.ts, 5) Update success messages to show detailed file lists
  Testing strategy: Unit test action handlers (not internal functions). Mock ConfirmPrompt component responses using vitest. Ensure tests call the actual action handler path that real users trigger.
  Files to modify: src/commands/remove-init-files.ts (action handler), src/commands/init.ts (action handler + remove duplicate), src/components/ConfirmPrompt.tsx (create new), src/commands/__tests__/remove-init-files.test.ts (fix tests), src/commands/__tests__/init-agent-switching.test.ts (fix tests)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. remove-init-files MUST prompt user with interactive Yes/No for keeping spec/fspec-config.json
  #   2. init command MUST detect existing agent before installation and compare with requested agent
  #   3. init command MUST prompt for agent switch confirmation when existing agent differs from requested agent
  #   4. Success messages MUST show detailed list of files removed/installed to match help documentation
  #   5. Agent config MUST be written only once (remove duplicate writeAgentConfig calls)
  #   6. Create a generic ConfirmPrompt.tsx component that can be reused for both cases (and future confirmations). Interface: ConfirmPrompt({ message, confirmLabel?, cancelLabel?, onSubmit }) returns boolean. This follows DRY principle and keeps components focused.
  #   7. Unit test the action handlers (not just internal functions). Mock ConfirmPrompt component responses. This is sufficient and provides faster feedback than E2E tests.
  #   8. Add --keep-config and --no-keep-config flags for explicit control in non-interactive environments. When flags are provided, skip the interactive prompt. When no flag provided, show interactive prompt (normal behavior). This is clear and unambiguous.
  #
  # EXAMPLES:
  #   1. User runs 'fspec remove-init-files', sees interactive prompt 'Keep spec/fspec-config.json?', selects 'Yes', config remains, agent files removed
  #   2. User runs 'fspec remove-init-files', sees interactive prompt, selects 'No', all files including config removed, detailed output shown
  #   3. User runs 'fspec init --agent=cursor' when Claude is installed, sees 'Switch from Claude to Cursor?' prompt, selects 'Switch', Claude files removed, Cursor installed
  #   4. User runs 'fspec init --agent=cursor' when Claude is installed, sees switch prompt, selects 'Cancel', Claude files remain, process exits cleanly
  #   5. User runs 'fspec init --agent=claude' when Claude is already installed, no prompt shown, files reinstalled idempotently
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should we create new React/Ink components (KeepConfigPrompt.tsx, AgentSwitchPrompt.tsx) or integrate into existing AgentSelector component?
  #   A: true
  #
  #   Q: Should we add E2E tests that spawn actual CLI processes, or is unit testing the action handlers sufficient?
  #   A: true
  #
  #   Q: What should happen if user runs remove-init-files in non-interactive environment (CI/CD)? Should we have a --yes flag to skip prompts?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer using fspec
    I want to have complete implementations of remove-init-files and init agent switching
    So that the CLI actually works as documented and passes real user acceptance tests

  Scenario: Remove agent files but keep config via interactive prompt
    Given I have fspec initialized with Claude agent
    And spec/fspec-config.json exists
    When I run 'fspec remove-init-files' without flags
    And the interactive prompt asks 'Keep spec/fspec-config.json?'
    And I select 'Yes'
    Then spec/CLAUDE.md should be removed
    And .claude/commands/fspec.md should be removed
    And spec/fspec-config.json should still exist
    And the output should show detailed list of removed files

  Scenario: Remove all files including config via interactive prompt
    Given I have fspec initialized with Claude agent
    And spec/fspec-config.json exists
    When I run 'fspec remove-init-files' without flags
    And the interactive prompt asks 'Keep spec/fspec-config.json?'
    And I select 'No'
    Then spec/CLAUDE.md should be removed
    And .claude/commands/fspec.md should be removed
    And spec/fspec-config.json should be removed
    And the output should show detailed list of removed files

  Scenario: Switch agents when different agent requested via CLI
    Given I have fspec initialized with Claude agent
    And spec/fspec-config.json contains agent 'claude'
    When I run 'fspec init --agent=cursor'
    And the prompt asks 'Switch from Claude to Cursor?'
    And I select 'Switch to Cursor'
    Then spec/CLAUDE.md should be removed
    And .claude/commands/fspec.md should be removed
    And spec/CURSOR.md should be created
    And .cursor/commands/fspec.md should be created
    And spec/fspec-config.json should contain agent 'cursor'
    And the output should show detailed list of installed files

  Scenario: Cancel agent switch and keep existing setup
    Given I have fspec initialized with Claude agent
    And spec/fspec-config.json contains agent 'claude'
    When I run 'fspec init --agent=cursor'
    And the prompt asks 'Switch from Claude to Cursor?'
    And I select 'Cancel'
    Then spec/CLAUDE.md should remain unchanged
    And .claude/commands/fspec.md should remain unchanged
    And spec/CURSOR.md should not be created
    And spec/fspec-config.json should still contain agent 'claude'
    And the command should exit with code 0

  Scenario: Reinstall same agent without prompt (idempotent)
    Given I have fspec initialized with Claude agent
    And spec/fspec-config.json contains agent 'claude'
    When I run 'fspec init --agent=claude'
    Then no switch prompt should appear
    And spec/CLAUDE.md should be reinstalled
    And .claude/commands/fspec.md should be reinstalled
    And spec/fspec-config.json should still contain agent 'claude'
    And the command should exit successfully

  Scenario: Remove all files using --no-keep-config flag (non-interactive)
    Given I have fspec initialized with Claude agent
    And spec/fspec-config.json exists
    When I run 'fspec remove-init-files --no-keep-config'
    Then no interactive prompt should appear
    And spec/CLAUDE.md should be removed
    And .claude/commands/fspec.md should be removed
    And spec/fspec-config.json should be removed
    And the output should show detailed list of removed files

  Scenario: Keep config using --keep-config flag (non-interactive)
    Given I have fspec initialized with Claude agent
    And spec/fspec-config.json exists
    When I run 'fspec remove-init-files --keep-config'
    Then no interactive prompt should appear
    And spec/CLAUDE.md should be removed
    And .claude/commands/fspec.md should be removed
    And spec/fspec-config.json should still exist
    And the output should show detailed list of removed files
