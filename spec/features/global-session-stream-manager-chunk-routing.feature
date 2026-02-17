@session-management
@tui
@BRIDGE-012
Feature: GlobalSessionStreamManager chunk routing by session_id
  """
  TypeScript GlobalSessionStreamManager registers the global callback ONCE at startup.
  It receives ALL chunks from ALL sessions and routes them to session-specific handlers.
  Session isolation is achieved via Map lookup in TypeScript, not Rust gating.
  """

  Background: User Story
    As a developer
    I want GlobalSessionStreamManager to route chunks by session_id
    So that each UI component only receives chunks for its session

  Scenario: Register global callback once at initialization
    Given GlobalSessionStreamManager is not initialized
    When initGlobalSessionStreamManager is called
    Then sessionSetGlobalChunkCallback should be called exactly once
    And the callback should be stored for routing

  Scenario: Route chunk to correct session handler
    Given GlobalSessionStreamManager is initialized
    And a handler is registered for session "session-a"
    And a handler is registered for session "session-b"
    When a chunk arrives for session "session-a"
    Then only the handler for session "session-a" should be invoked
    And the handler for session "session-b" should not be invoked

  Scenario: Ignore chunks for sessions without handlers
    Given GlobalSessionStreamManager is initialized
    And a handler is registered for session "session-a"
    When a chunk arrives for session "session-unknown"
    Then no handler should be invoked
    And no error should be thrown

  Scenario: Multiple handlers for same session all receive chunk
    Given GlobalSessionStreamManager is initialized
    And handler A is registered for session "session-x"
    And handler B is registered for session "session-x"
    When a chunk arrives for session "session-x"
    Then both handler A and handler B should be invoked

  Scenario: Global handlers receive all chunks with session_id
    Given GlobalSessionStreamManager is initialized
    And a global handler is registered
    When a chunk arrives for session "session-a"
    And a chunk arrives for session "session-b"
    Then the global handler should receive both chunks
    And each chunk should include its session_id

  Scenario: No sessionAttach or sessionDetach calls
    Given GlobalSessionStreamManager source code
    When I search for sessionAttach usage
    Then no usages should be found
    When I search for sessionDetach usage
    Then no usages should be found
