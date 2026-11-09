@done
@RPC-002
@REFAC-061
@session
@session-management
Feature: Session resume failure and isolation
  """
  RPC-002: resume_session fails for a session that was never persisted, and two sessions persisted through the agent_loop helpers are restored independently after a simulated process restart. Tests: rust/agent-loop/tests/rpc002_session_resume_isolation.rs (REFAC-061).
  """

  Background: User Story
    As a Rust TUI user
    I want session content written by the agent loop to survive a process restart
    So that resuming a session restores its full message history and token state

  Scenario: Resume fails for a non-existent session
    Given a SessionManager with no sessions
    When I try to resume a session that doesn't exist
    Then it returns an error

  Scenario: Multiple sessions persist independently
    Given a SessionManager with two sessions
    When I reset the stores and resume both sessions
    Then both sessions are restored independently
