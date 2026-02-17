@done
@tui
@BRIDGE-013
Feature: TUI persistent chunk handler for bridge input display
  """
  Uses useSessionStreamManager hook at AgentView component level, not inside callbacks
  Handler cleanup managed by useEffect return function - automatic on session change or component unmount
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. AgentView must call useSessionStreamManager(currentSessionId, handleStreamChunk) at component level
  #   2. The persistent handler must process ALL chunk types that handleStreamChunk currently handles
  #   3. When session changes, old handler is automatically unregistered via useEffect cleanup
  #   4. Integration tests must verify chunks flow from Rust NAPI through GlobalSessionStreamManager to React state
  #   5. Handler must NOT be registered when currentSessionId is null/undefined
  #   6. Chunks for non-current sessions must be ignored by the handler
  #   7. processStreamingChunk must be reused - no new inline chunk processing logic
  #
  # EXAMPLES:
  #   1. TUI is idle viewing session, bridge sends input, LLM TextChunks are displayed in real-time in TUI
  #   2. TUI is idle viewing session, bridge sends input, ToolCall chunks appear in TUI conversation
  #   3. TUI is idle viewing session, bridge sends input, Done chunk updates conversation state and re-enables input
  #   4. User switches from session A to session B, then bridge sends input to session A - TUI does NOT show session A chunks
  #   5. User sends input via TUI while persistent handler is active - chunks flow correctly through same handler
  #   6. WatcherInput chunk from bridge shows injected input in TUI conversation before LLM response
  #
  # ========================================
  Background: User Story
    As a TUI user viewing a session
    I want to see all LLM responses in real-time regardless of input source
    So that I have visibility into session activity from bridges or watchers

  @bridge-input
  @text-chunks
  Scenario: Display TextChunks from bridge input while TUI is idle
    Given the TUI is viewing session "test-session"
    And the TUI input is idle with no pending requests
    And a persistent chunk handler is registered for "test-session"
    When the bridge sends input to session "test-session"
    And the LLM responds with TextChunk data
    Then the TextChunk content should appear in the TUI conversation
    And the conversation should update in real-time

  @bridge-input
  @tool-calls
  Scenario: Display ToolCall chunks from bridge input while TUI is idle
    Given the TUI is viewing session "test-session"
    And the TUI input is idle with no pending requests
    And a persistent chunk handler is registered for "test-session"
    When the bridge sends input to session "test-session"
    And the LLM responds with ToolCall chunks
    Then the ToolCall should appear in the TUI conversation
    And the tool execution should be displayed

  @bridge-input
  @done-chunk
  Scenario: Done chunk updates conversation state and re-enables input
    Given the TUI is viewing session "test-session"
    And the TUI input is idle with no pending requests
    And a persistent chunk handler is registered for "test-session"
    When the bridge sends input to session "test-session"
    And the LLM responds with a Done chunk
    Then the conversation state should be updated
    And the TUI input should be re-enabled

  @session-switch
  @handler-isolation
  Scenario: Switching sessions does not show chunks from previous session
    Given the TUI is viewing session "session-A"
    And a persistent chunk handler is registered for "session-A"
    When the user switches to session "session-B"
    And the bridge sends input to session "session-A"
    And the LLM responds with TextChunk data for "session-A"
    Then the TUI should NOT display chunks from "session-A"
    And the persistent handler for "session-A" should be unregistered

  @tui-input
  @handler-reuse
  Scenario: User input via TUI flows through persistent handler
    Given the TUI is viewing session "test-session"
    And a persistent chunk handler is registered for "test-session"
    When the user sends input via the TUI
    And the LLM responds with TextChunk data
    Then the TextChunk content should appear in the TUI conversation
    And the same persistent handler should process the chunks

  @bridge-input
  @watcher-input
  Scenario: WatcherInput chunk shows injected input before LLM response
    Given the TUI is viewing session "test-session"
    And the TUI input is idle with no pending requests
    And a persistent chunk handler is registered for "test-session"
    When the bridge sends input to session "test-session"
    And a WatcherInput chunk is emitted
    Then the injected input should appear in the TUI conversation
    And the injected input should appear before the LLM response

  @edge-case
  @null-session
  Scenario: Handler not registered when session is null
    Given the TUI has no active session
    When the component renders
    Then no chunk handler should be registered
    And the GlobalSessionStreamManager should have no handlers

  @integration
  @end-to-end
  Scenario: Chunks flow from Rust NAPI through GlobalSessionStreamManager to React state
    Given the TUI is viewing session "test-session"
    And a persistent chunk handler is registered for "test-session"
    When Rust NAPI emits a chunk via GLOBAL_CHUNK_CALLBACK
    Then GlobalSessionStreamManager should receive the chunk
    And the chunk should be dispatched to the registered handler
    And the React conversation state should be updated
