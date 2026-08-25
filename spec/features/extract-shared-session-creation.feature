@done
@RPC-425
Feature: Extract shared session creation logic into common helper
  """
  create_session_with_id and create_session_from_manifest share ~200 lines of nearly identical code:
  - Provider manager creation
  - Credentials resolution
  - Lifecycle hooks loading
  - BackgroundSession construction
  - Pre-tool hook registration
  - MCP session initialization
  - Agent loop spawning
  - Session insertion into in-memory map
  - Session-created broadcast
  - Isolation state change broadcast
  - Footer poller spawning
  - Metadata broadcast

  Extract this into a shared helper function so both methods call it.
  """

  Background: User Story
    As a Rust developer
    I want to have shared session creation logic extracted into a common helper
    So that I reduce code duplication and make future changes easier to maintain

  Scenario: create_session_with_id uses shared helper for session construction
    Given a SessionManager with create_session_with_id
    When a session is created via create_session_with_id
    Then the shared helper creates the BackgroundSession
    And the manifest is saved to disk before the helper is called

  Scenario: create_session_from_manifest uses shared helper for session construction
    Given a SessionManager with create_session_from_manifest
    When a session is resumed via create_session_from_manifest
    Then the shared helper creates the BackgroundSession
    And the manifest is NOT saved to disk

  Scenario: Shared helper preserves all existing session setup behavior
    Given the shared session creation helper
    When it creates a session
    Then credentials are resolved for the provider
    And lifecycle hooks are loaded from the project path
    And pre-tool hooks are registered if lifecycle hooks exist
    And MCP session is initialized
    And agent loop is spawned via hooks
    And session is inserted into in-memory map
    And session-created broadcast is sent
    And isolation state change is broadcast
    And footer poller is spawned
    And metadata update is broadcast

  Scenario: Model limits and thinking level are set by shared helper
    Given the shared session creation helper
    When it creates a session
    Then the persisted default thinking level is applied
    And model limits (context window, max output tokens, compaction threshold) are set
