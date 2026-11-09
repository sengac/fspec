@done
@session-creation
@session
@persistence
@rust
@BUG-414
Feature: Session creation failure hygiene
  """
  SessionManager::create_session_with_id persists the session manifest only
  after the in-memory session is fully constructed (provider resolution and
  create_background_session_inner succeed), so a failed creation leaves no
  orphaned manifest, and the failure reaches the caller as an Err (BUG-399).
  Tests: rust/sessions/tests/session_creation_failure_hygiene.rs, fully
  offline (seeded models.json, dummy credentials). BUG-414 gave these tests
  their feature; the test file named it but it did not exist.
  """

  Background: User Story
    As a developer whose session creation can fail
    I want a failed creation to leave nothing behind and to tell me why
    So that a broken session never reappears from disk and errors are never silent

  Scenario: A failed session creation leaves no orphaned manifest on disk
    Given a SessionManager with a seeded offline model cache
    And the default model is set to a model NOT present in the registry
    When I call create_session with no role
    Then session creation fails with an error
    And no orphaned session manifest is persisted to disk
    And no session exists in the manager's in-memory map

  Scenario: A successful session creation persists exactly one manifest
    Given a SessionManager with a seeded offline model cache
    And the default model is set to a model present in the registry
    When I call create_session with no role
    Then the session is created
    And exactly one session manifest is persisted to disk

  Scenario: A malformed model string leaves no orphaned manifest on disk
    Given a SessionManager with a seeded offline model cache
    And the default model is a malformed string (no '/')
    When I call create_session with no role
    Then session creation fails with an error
    And no orphaned session manifest is persisted to disk
