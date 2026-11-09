@done
@RPC-002
@REFAC-061
@session
@session-management
Feature: Session persistence round trip through the agent loop
  """
  RPC-002 round trip: messages and token state written through the agent_loop persist helpers (persist_user_message, persist_assistant_message_internal, persist_token_state) land in the session manifest, survive reset_stores_for_tests (a simulated process restart), and are kept by SessionManagerHandle::resume_session. Tests: rust/agent-loop/tests/rpc002_session_persistence.rs, helpers in rust/agent-loop/tests/rpc002_session_persistence_support/mod.rs (REFAC-061).
  """

  Background: User Story
    As a Rust TUI user
    I want session content written by the agent loop to survive a process restart
    So that resuming a session restores its full message history and token state

  Scenario: Full round trip persists messages and keeps them on resume
    Given a SessionManager with a session that has persisted messages
    When I reset the stores (simulating process restart)
    And I resume the session
    Then the session is in memory with restored messages
    And the manifest still has all 4 message references

  Scenario: Resume restores token state from manifest
    Given a SessionManager with a session that has token state
    When I reset the stores and resume
    Then the token state is restored

  Scenario: Resume restores messages with correct ordering
    Given a session with multiple messages in specific order
    When I reset the stores and resume
    Then the manifest still has all 5 message references in order
