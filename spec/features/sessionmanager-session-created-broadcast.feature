@done
@rust
@agent-manager
@session
@RPC-385
Feature: SessionManager session-created broadcast
  """
  SessionManager owns a session_created broadcast sender alongside chunks_tx/logs_tx/status_changes_tx. create_session_with_id and create_isolated_session_with_id fire it after inserting the new session, carrying the SessionInfo. This is the backend half of RPC-385 (Approach A); the TUI half consumes it via FspecBackend::session_created_rx.
  """

  Background: User Story
    As an operator running the Rust TUI
    I want SessionManager to broadcast a session-created event whenever any session is created
    So that subscribers (the TUI) can react and register newly created sessions

  Scenario: Creating a session broadcasts a session-created event
    Given a SessionManager with a subscriber on the session-created broadcast
    When a session is created via create_session_with_id
    Then the subscriber receives the new session id
