@RPC-422
Feature: Session Persistence Integration
  """
  Mirrors TypeScript sessionService.ts two-step pattern: persistenceCreateSessionWithProvider() creates manifest, then sessionManagerCreateWithId() creates BackgroundSession. Rust code must follow the same order in SessionManager::create_session_with_id()
  """

  Background: User Story
    As a Rust TUI user
    I want to create and persist sessions to disk
    So that sessions survive process restart and can be resumed with full message history

  Scenario: Session creation persists manifest to disk before creating BackgroundSession
    Given a SessionManager instance with no existing sessions
    When I call create_session_with_id with a valid UUID, model, project, and name
    Then a session manifest file should exist at {data_dir}/sessions/{uuid}.json
    And the manifest should contain the session name, project path, and provider
    And the manifest should have an empty messages list
    And the in-memory session map should contain the BackgroundSession with the same UUID

  Scenario: Session creation persists manifest with provider information
    Given a SessionManager instance with no existing sessions
    When I call create_session_with_id with model "anthropic/claude-sonnet-4"
    Then the persisted manifest should have provider field set to "anthropic/claude-sonnet-4"

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

  Scenario: Session creation fails gracefully when persistence fails
    Given a SessionManager instance with a corrupted data directory
    When I call create_session_with_id
    Then the error should propagate and the BackgroundSession should not be created
