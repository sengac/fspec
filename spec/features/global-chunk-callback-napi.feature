@session-management
@codelet
@BRIDGE-012
Feature: Global chunk callback NAPI for session-agnostic chunk emission
  """
  Rust exposes a single global callback via NAPI that TypeScript registers once at app startup.
  ALL chunks from ALL sessions go through this ONE callback with signature (session_id, chunk).
  Rust has ZERO knowledge of which session is active/attached - it's a pure emitter.
  This replaces the per-session attach()/detach() pattern completely.
  """

  Background: User Story
    As a developer
    I want Rust to emit all chunks via a single global callback with session_id
    So that TypeScript owns all routing logic and Rust remains stateless

  Scenario: Register global chunk callback at startup
    Given no global chunk callback is registered
    When TypeScript calls sessionSetGlobalChunkCallback with a callback function
    Then Rust should store the callback in a global static
    And subsequent chunk emissions should use this callback

  Scenario: Emit chunk with session_id through global callback
    Given a global chunk callback is registered
    And a session exists with id "session-abc"
    When the session emits a TextDelta chunk via handle_output
    Then the global callback should be invoked with session_id "session-abc"
    And the global callback should receive the TextDelta chunk

  Scenario: Multiple sessions emit through same global callback
    Given a global chunk callback is registered
    And session "session-a" exists
    And session "session-b" exists
    When session "session-a" emits a chunk
    And session "session-b" emits a chunk
    Then both chunks should go through the same global callback
    And each chunk should have its respective session_id

  Scenario: No attachment state in Rust
    Given a session exists
    When I inspect the BackgroundSession struct
    Then there should be no is_attached field
    And there should be no attached_callback field
    And there should be no attach method
    And there should be no detach method

  Scenario: No per-session NAPI attachment functions
    When I inspect the NAPI module exports
    Then there should be no session_attach function
    And there should be no session_detach function
    And there should be a sessionSetGlobalChunkCallback function
