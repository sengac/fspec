@done
@SESS-002
Feature: Resume session overwrites manifest destroying message references

  """
  The fix is in rust/sessions/src/handle_impl.rs resume_session(). Instead of calling create_session_with_id() which overwrites the manifest, we need to create the BackgroundSession directly using the loaded manifest data. The manifest already has message references - we just need to pass them to the BackgroundSession.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When resuming a session that is not in memory, the session manifest must NOT be overwritten with a blank manifest
  #   2. When resuming a session not in memory, the BackgroundSession must be created with the manifest's existing message references preserved
  #   3. The resume_session function must use the already-loaded manifest data to create the BackgroundSession instead of calling create_session_with_id
  #   4. After resume, the session must have all its previous messages restored and visible in the TUI
  #
  # EXAMPLES:
  #   1. User has a session with 102 messages, closes TUI, reopens and resumes - all 102 messages should be visible
  #   2. User resumes a session that was just created (0 messages) - session should work normally with no messages
  #   3. User resumes a session that is already in memory - no change to existing behavior, messages remain intact
  #
  # ========================================

  Background: User Story
    As a user
    I want to resume a previous session
    So that see all my previous conversation messages

  Scenario: Resume session with many messages after TUI restart
    Given I have a session with 102 messages persisted on disk
    When I close the TUI and reopen it
    And I resume the session
    Then all 102 messages should be visible in the session history
    And the session manifest should still reference all 102 messages

  Scenario: Resume empty session after TUI restart
    Given I have a session with zero messages persisted on disk
    When I close the TUI and reopen it
    And I resume the session
    Then the session should be empty with no messages
    And the session should be functional for new messages

  Scenario: Resume session that is already in memory
    Given I have a session with messages that is currently active in memory
    When I resume the same session
    Then the session messages remain unchanged
    And no manifest overwrite occurs
