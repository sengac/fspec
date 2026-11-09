@done
@session
@session-management
@BUG-186
@RPC-422
Feature: Session Persistence Integration
  """
  Mirrors TypeScript sessionService.ts two-step pattern: persistenceCreateSessionWithProvider() creates manifest, then sessionManagerCreateWithId() creates BackgroundSession. Rust code must follow the same order in SessionManager::create_session_with_id(). REFAC-059 moved the destroy, delete, list and resume scenarios to session-persistence-lifecycle.feature.

  BUG-186: the rpc002 (agent-loop) and rpc422 (sessions) persistence test suites seed the fresh temp data dir with an offline models.dev cache fixture (rust/sessions/tests/fixtures/rpc422_models.json for rpc422, rust/agent-loop/tests/fixtures/prov101_models.json for rpc002) plus dummy provider credentials, so resolve_provider_manager → ModelRegistry::new → ModelCache::get() stays fully offline and create_session no longer declines with an empty SessionId. rpc422 keeps its own fixture (claude-sonnet-4 + claude-opus-4-5 + o3 + gemini-2.5-pro) because its acceptance criteria pin model "anthropic/claude-sonnet-4", which the shared prov101 fixture does not contain and which would flip PROV-130's first-model ordering assertion in shared consumers.
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

  Scenario: Session creation fails gracefully when persistence fails
    Given a SessionManager instance with a corrupted data directory
    When I call create_session_with_id
    Then the error should propagate and the BackgroundSession should not be created

  Scenario: Session persistence tests survive a clean offline environment
    Given a fresh temp data directory without provider credentials or a models.dev cache
    When I run the rpc002 and rpc422 session-persistence test suites in that environment
    Then session creation resolves the default model from the seeded offline cache and succeeds
