@GIT-038
@gitoxide-integration
@isolated-sessions
Feature: Send merge conflict context to LLM for AI-assisted resolution
  """
  When /merge-worktree detects conflicts, send a user message to the Rust session
  (via the normal handleSubmit → sessionSendInput path) containing the conflict
  details. The LLM receives this as real user input so it can read the conflicting
  files, resolve the git conflict markers, and tell the user to run /merge-worktree
  again. This is ADDITIONAL to the existing TUI status message — the LLM message
  makes the AI aware of the conflict state.

  Architecture:
  - mergeWorktreeHandler.ts gains an injectLlmContext callback in MergeWorktreeContext
  - On conflict, calls injectLlmContext with structured conflict details
  - AgentView.tsx wires injectLlmContext to set inputValue + flag pendingAutoSubmitRef
  - A useEffect fires handleSubmit() on the next render, which calls sessionSendInput
  - The Rust agent loop receives the conflict details as a normal user message
  - CRITICAL: persistenceAppendMessage only writes to disk — sessionSendInput is the
  correct path to reach the live Rust session
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When /merge-worktree detects conflicts, the conflict details MUST be sent as a user message to the Rust session via sessionSendInput so the LLM can act on it.
  #   2. The conflict message must include the worktree path so the LLM knows where to find the conflicting files.
  #   3. The conflict message must mention git conflict markers and instruct the LLM to resolve them.
  #   4. The conflict context is sent as a user-role message via the normal handleSubmit → sessionSendInput path, not via persistenceAppendMessage.
  #   5. The visual TUI status message remains unchanged — the user message to the LLM is an ADDITIONAL action.
  #   6. Only conflict errors trigger the user message — successful merges and non-conflict errors do NOT send any message.
  #
  # EXAMPLES:
  #   1. User runs /merge-worktree → conflicts on README.md → conflict message sent to Rust via sessionSendInput → LLM reads file, resolves markers
  #   2. User runs /merge-worktree → conflicts on multiple files → all listed in the message sent to Rust
  #   3. User runs /merge-worktree → merge succeeds → NO message sent to Rust session
  #   4. User runs /merge-worktree → non-conflict error → NO message sent to Rust session
  #
  # ========================================
  Background: User Story
    As a user in an isolated session
    I want to have the AI resolve merge conflicts when /merge-worktree detects them
    So that I don't have to manually find and fix git conflict markers in files

  Scenario: Single file conflict sends context to LLM
    Given I am in an isolated session with a worktree
    And the session has a valid session ID
    When I run /merge-worktree and a conflict is detected on "README.md"
    Then the TUI status message should contain the conflict summary
    And a conflict context message should be sent to the LLM session
    And the message should list "README.md" as a conflicting file
    And the message should mention git conflict markers
    And the message should instruct to run /merge-worktree again after resolving

  Scenario: Multiple file conflicts send context listing all files
    Given I am in an isolated session with a worktree
    And the session has a valid session ID
    When I run /merge-worktree and conflicts are detected on "src/auth/login.ts" and "src/utils/helpers.ts"
    Then a conflict context message should be sent to the LLM session
    And the message should list "src/auth/login.ts" as a conflicting file
    And the message should list "src/utils/helpers.ts" as a conflicting file

  Scenario: Conflict context triggers auto-submit to Rust session
    Given I am in an isolated session with a worktree
    And the session has a valid session ID
    When I run /merge-worktree and a conflict is detected on "README.md"
    Then the injectLlmContext callback should be called with conflict details
    And the TUI status message should still be present unchanged

  Scenario: Successful merge does not send context to LLM
    Given I am in an isolated session with a worktree
    And the session has a valid session ID
    When I run /merge-worktree and the merge succeeds
    Then no conflict context message should be sent to the LLM session
    And the action prompt "Press Enter to close session" should be shown

  Scenario: Non-conflict error does not send context to LLM
    Given I am in an isolated session with a worktree
    And the session has a valid session ID
    When I run /merge-worktree and a non-conflict error occurs
    Then no conflict context message should be sent to the LLM session
    And the TUI status message should show the generic error

  Scenario: Context message does not contain system-reminder tags
    Given I am in an isolated session with a worktree
    And the session has a valid session ID
    When I run /merge-worktree and a conflict is detected on "README.md"
    Then the conflict context message should be sent to the LLM session
    And the message should not contain system-reminder tags

  Scenario: No context sent when session ID is missing
    Given I am in an isolated session with a worktree
    But the session ID is null
    When I run /merge-worktree and a conflict is detected
    Then no conflict context message should be sent to the LLM session
    And the TUI status message should still show the conflict summary
