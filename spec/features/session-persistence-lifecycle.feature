@done
@session
@session-management
@RPC-422
@REFAC-059
Feature: Session Persistence Lifecycle
  """
  Split from session-persistence-integration.feature (REFAC-059): destroy, delete, list and resume of persisted sessions in SessionManager. Tests live in rust/sessions/tests/rpc422_session_lifecycle_persistence.rs and share the offline data-dir helpers in rust/sessions/tests/rpc422_session_persistence_support/mod.rs.
  """

  Background: User Story
    As a Rust TUI user
    I want persisted sessions to survive destruction, listing and resume
    So that sessions survive process restart and can be resumed with full message history

  Scenario: Session destruction removes from memory but preserves manifest
    Given a SessionManager with a persisted session manifest on disk
    When I call destroy_session with that session's UUID
    Then the session should be removed from the in-memory session map
    And the manifest file at {data_dir}/sessions/{uuid}.json should still exist on disk
    And the session should still appear in list_sessions via persisted merge

  Scenario: Persistence delete removes manifest from disk
    Given a SessionManager with a persisted session manifest on disk
    When I call persistence_delete_session with that session's UUID
    Then the manifest file at {data_dir}/sessions/{uuid}.json should no longer exist

  Scenario: Session listing includes both in-memory and persisted sessions
    Given a SessionManager with one in-memory session
    And a second session manifest persisted on disk but not in memory
    When I call list_sessions
    Then the result should contain both sessions

  Scenario: Resume session loads messages from persistence layer
    Given a persisted session manifest with two stored messages
    When I call resume_session with that session's UUID
    Then the BackgroundSession should be created in memory
    And the session's inner messages should contain the restored messages
    And the token state should be restored from the manifest
