@BUG-628
@testing
@integration
@BLOCK-001
Feature: Blocklist System Integration Tests
  """
  The end-to-end tests are in rust/tools/tests/blocklist_system_integration.rs: they drive the real Write and Read tools for a session, with HOME and the user directory isolated, a registered work unit stage callback and block notification callback, and a pause handler standing in for the user's triple choice.
  A TUI restart is a new process: the in-memory session allowances are gone (clear_session_allowances) and the blocklist is initialized again (init_blocklist).
  Startup initialization is project-blocklist-initialization's (build_service); turning a rule off for a session is BUG-601's.
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
  #   5. Session allowances are kept in memory only, so after a restart the same rule prompts again
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

  Scenario: Stage permissions block and allow based on work unit state
    Given a work unit is in "testing" stage
    When the AI tries to write to "src/auth.ts"
    Then the write should be blocked by stage permissions
    And a block notification is emitted for the session
    When the AI tries to write to "src/__tests__/auth.test.ts"
    Then the write should be allowed
    When the work unit is moved to "implementing" stage
    And the AI tries to write to "src/auth.ts"
    Then the write should now be allowed

  Scenario: Sensitive path prompts with session memory
    Given a blocklist rule exists prompting for "~/.ssh" access
    When the AI tries to read "~/.ssh/config"
    Then the read pauses for an Allow Once, Allow Session or Deny choice
    When the user selects "Allow Session"
    Then the file should be read successfully
    When the AI tries to read "~/.ssh/known_hosts"
    Then no prompt should appear due to session allowance
    When the TUI is restarted
    And the AI tries to read "~/.ssh/id_rsa"
    Then a prompt should appear again
