@testing
@integration
@BLOCK-001
Feature: Blocklist System Integration Tests
  """
  Integration tests should be placed in rust/tools/tests/ for Rust E2E tests and src/__tests__/integration/ for TypeScript/React E2E tests
  Use mock file system and mock session context to test complete flows without requiring actual TUI rendering
  Test NAPI bindings integration: blocklist_load, blocklist_check, blocklist_allow_session must be tested across Rust→Node boundary
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Tests must verify complete user journey from config loading → rule evaluation → TUI feedback
  #   2. Integration tests must exercise all child feature components together: BLOCK-002 (core), BLOCK-003 (stage), BLOCK-004 (TUI), BLOCK-005 (prompts), BLOCK-006 (notifications)
  #   3. Test realistic TUI interaction flows including session state, navigation, and visual feedback
  #   4. Verify config hierarchy works across system → project → session levels in integrated scenarios
  #   5. Test Rust ↔ TypeScript ↔ React data flow for all blocklist operations
  #
  # EXAMPLES:
  #   1. E2E flow: System rule blocks git checkout → Project override allows git checkout stash → TUI shows blocklist with merged rules → User disables rule for session → AI runs git checkout stash → Allowed → TUI restart → Rule active again
  #   2. E2E flow: Work unit in testing stage → AI writes impl file → Stage permission blocks → Notification shown to user → AI writes test file → Allowed → AI moves to implementing → AI writes impl file → Allowed
  #   3. E2E flow: AI reads ~/.ssh/config → Prompt dialog appears → User selects Allow Session → File read → AI reads ~/.ssh/known_hosts → No prompt (session allowed) → TUI restart → AI reads ~/.ssh/id_rsa → Prompt shown again
  #   4. TUI integration: User opens /blocklist → Sees merged system+project rules → Navigates with keyboard → Disables rule → Returns to agent view → AI action allowed → Re-opens /blocklist → Rule shows disabled state
  #
  # QUESTIONS (ANSWERED):
  #   Q: What happens when no work unit is active? Block all file writes? Allow all? Configurable default behavior?
  #   A: Allow all - no ACDD stage enforcement when no work unit is active
  #
  #   Q: Can user override stage blocks in-session? (e.g., 'I know I'm in specifying but let me write this impl file once') Or are stage blocks hard-enforced?
  #   A: No override - stage blocks are hard. Must move to correct ACDD stage to write those file types.
  #
  #   Q: One unified config file or separate concerns? (blocklist.json for commands/tools, stage-permissions.json for ACDD rules)
  #   A: Two separate files: blocklist.json (commands, tools, sensitive paths) and stage-permissions.json (ACDD file categories and stage mappings)
  #
  #   Q: Should there be prompt action (user confirms) in addition to block action? Or only block for this feature?
  #   A: Yes, both block and prompt actions. Block for clear substitutions (git checkout→switch, cat→Read) and ACDD violations. Prompt for context-dependent security (sensitive files where user decides).
  #
  #   Q: What TUI command for rule management? /rules? /blocklist? Both?
  #   A: /blocklist as the TUI command name
  #
  #   Q: Should blocked actions notify the user in TUI (e.g., 'AI tried to write impl file but was blocked') or just silently error to the AI?
  #   A: Notify user in TUI when AI is blocked (e.g., 'AI was blocked from writing src/auth.ts - testing stage')
  #
  #   Q: What are the default stage permissions for validating, done, and backlog stages? Example [18] only covers specifying, testing, implementing.
  #   A: backlog = nothing (work hasn't started), validating = nothing (if you need changes, go back to implementing), done = nothing (if you need changes, reopen it)
  #
  # ========================================
  Background: User Story
    As a developer
    I want to run integration tests for the blocklist system
    So that I can verify all components work together correctly

  Scenario: Config hierarchy with session override
    Given a system blocklist rule blocks "git checkout" commands
    When the user opens /blocklist
    Then the merged rules should show the project override
    And a project blocklist rule allows "git checkout stash"
    When the user disables the rule for the session
    And the AI runs "git checkout stash"
    Then the command should be allowed
    When the TUI is restarted
    Then the rule should be active again

  Scenario: Stage permissions block and allow based on work unit state
    Given a work unit is in "testing" stage
    When the AI tries to write to "src/auth.ts"
    Then the write should be blocked by stage permissions
    And a notification should be shown to the user
    When the AI tries to write to "src/__tests__/auth.test.ts"
    Then the write should be allowed
    When the work unit is moved to "implementing" stage
    And the AI tries to write to "src/auth.ts"
    Then the write should now be allowed

  Scenario: Sensitive path prompts with session memory
    Given a blocklist rule exists prompting for "~/.ssh" access
    When the AI tries to read "~/.ssh/config"
    Then a prompt dialog should appear
    When the user selects "Allow Session"
    Then the file should be read successfully
    When the AI tries to read "~/.ssh/known_hosts"
    Then no prompt should appear due to session allowance
    When the TUI is restarted
    And the AI tries to read "~/.ssh/id_rsa"
    Then a prompt should appear again

  Scenario: TUI blocklist view with session toggle
    Given system and project blocklist configs are loaded
    When the user opens /blocklist
    Then the merged rules from both configs should be displayed
    When the user navigates with keyboard and disables a rule
    And returns to agent view
    Then the AI action should be allowed
    When the user re-opens /blocklist
    Then the rule should show disabled state

  Scenario: Blocklist system initializes automatically at TUI startup
    Given a blocklist config exists at system level with a blocking rule
    When the TUI starts up
    Then blocklist rules should be loaded and active
    Then blocked commands should be rejected when executed
