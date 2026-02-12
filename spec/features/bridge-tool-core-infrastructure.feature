@bridge
@codelet
@BRIDGE-001
Feature: Bridge Tool Core Infrastructure

  """
  Uses watcher_broadcast channel via subscribe_to_stream(). WebSocket client connects to endpoint URL. BridgeConnection manages single connection, BridgeManager manages multiple. NAPI bindings: bridge_connect(url), bridge_disconnect(url), bridge_list(). JSON format: outbound {type: chunk, session_id, data: StreamChunk}, inbound {type: input, session_id, message}. tokio-tungstenite for WebSocket. Buffer up to 1GB during disconnect.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Multiple bridges can connect to a single session simultaneously
  #   2. Inbound messages from external platforms are injected into session's input channel
  #   3. Buffer messages when the external endpoint is unreachable, then deliver when connection is restored.
  #   4. No authentication required - bridge connects to local/trusted WebSocket endpoints
  #   5. WebSocket messages use JSON format with envelope containing type, session_id, and payload
  #   6. Buffer max 1GB - if exceeded, drop connection, log via logger.error, and show error dialog to user
  #   7. Auto-reconnect on unexpected disconnect with exponential backoff
  #   8. Bridge endpoints configured via config file (defaults) and/or TUI dialog (runtime add/override)
  #   9. Bridge connections remain active for the duration of the session until manually disconnected or endpoint becomes unreachable
  #   10. Bridge acts as WebSocket client - connects outbound to endpoint URL for bidirectional streaming (chunks out, input in)
  #   11. Bridge subscribes to session's watcher_broadcast channel and relays all StreamChunks (text, thinking, tools) - endpoint decides what to display
  #
  # EXAMPLES:
  #   1. User connects bridge to ws://localhost:8080 → AI responds 'Hello' → JSON chunk sent to endpoint
  #   2. Endpoint sends JSON input message via WebSocket → Bridge injects into session → Agent processes as user input
  #   3. User connects bridges to two endpoints (ws://localhost:8080 and ws://localhost:9090) → AI response sent to both endpoints
  #   4. WebSocket connection drops → Bridge buffers messages and attempts reconnect with backoff → Connection restored, buffered messages delivered
  #   5. User runs /bridge disconnect ws://localhost:8080 → WebSocket closed → Session continues without that bridge
  #   6. Buffer exceeds 1GB while endpoint is down → Connection dropped, error dialog shown, logged via logger.error
  #   7. Bridge endpoints in config file auto-connect on session start → User can add more via TUI dialog at runtime
  #   8. User tries to connect to ws://invalid:9999 → Connection refused → Error shown in TUI, logged via logger.error
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should tool outputs (Read, Bash, etc.) be relayed to external platforms, or only AI text responses?
  #   A: Send all StreamChunks to the endpoint via SSE streaming. The endpoint decides what to relay to platforms like Telegram.
  #
  #   Q: Should thinking/reasoning be relayed, or only final assistant responses?
  #   A: Relay everything including thinking/reasoning. The bridge is a dumb pipe - the endpoint decides what to display.
  #
  #   Q: What happens when bridge is connected but external platform is unreachable? Buffer messages or drop them?
  #   A: Buffer messages when the external endpoint is unreachable, then deliver when connection is restored.
  #
  # ========================================

  Background: User Story
    As a developer
    I want to connect external messaging platforms to agent sessions
    So that monitor and control agents remotely from my phone via Telegram or other platforms

  # -------------------------------------------
  # Outbound: Session → Bridge → Endpoint
  # -------------------------------------------

  @outbound
  Scenario: Relay AI response to connected WebSocket endpoint
    Given a session is running
    And a bridge is connected to "ws://localhost:8080"
    When the AI responds with "Hello"
    Then a JSON chunk should be sent to the endpoint
    And the chunk should contain type "chunk" and the session_id
    And the chunk data should contain the AI response

  @inbound
  Scenario: Receive input from WebSocket endpoint and inject into session
    Given a session is running
    And a bridge is connected to "ws://localhost:8080"
    When the endpoint sends a JSON message with type "input" and message "run tests"
    Then the message should be injected into the session's input channel
    And the agent should process "run tests" as user input

  @multiple-bridges
  Scenario: Relay AI response to multiple connected endpoints
    Given a session is running
    And a bridge is connected to "ws://localhost:8080"
    And a bridge is connected to "ws://localhost:9090"
    When the AI responds with "Hello"
    Then a JSON chunk should be sent to "ws://localhost:8080"
    And a JSON chunk should be sent to "ws://localhost:9090"

  # -------------------------------------------
  # Connection Management
  # -------------------------------------------

  @reconnect
  Scenario: Buffer messages and reconnect on connection drop
    Given a session is running
    And a bridge is connected to "ws://localhost:8080"
    When the WebSocket connection drops unexpectedly
    Then the bridge should buffer outgoing messages
    And the bridge should attempt to reconnect with exponential backoff
    When the connection is restored
    Then the buffered messages should be delivered to the endpoint

  @disconnect
  Scenario: Manually disconnect a bridge
    Given a session is running
    And a bridge is connected to "ws://localhost:8080"
    When the user runs "/bridge disconnect ws://localhost:8080"
    Then the WebSocket connection should be closed
    And the session should continue running without that bridge

  @buffer-overflow
  Scenario: Drop connection when buffer exceeds 1GB
    Given a session is running
    And a bridge is connected to "ws://localhost:8080"
    And the WebSocket connection is down
    When the message buffer exceeds 1GB
    Then the bridge connection should be dropped
    And an error dialog should be shown to the user
    And the error should be logged via logger.error

  # -------------------------------------------
  # Configuration
  # -------------------------------------------

  @config
  Scenario: Auto-connect bridges from config file on session start
    Given bridge endpoints are configured in the config file:
      | url                    |
      | ws://localhost:8080    |
      | ws://localhost:9090    |
    When a new session starts
    Then bridges should auto-connect to all configured endpoints
    And the user can add more bridges via TUI dialog at runtime

  @connection-error
  Scenario: Handle connection failure gracefully
    Given a session is running
    When the user tries to connect a bridge to "ws://invalid:9999"
    And the connection is refused
    Then an error should be shown in the TUI
    And the error should be logged via logger.error
    And the session should continue running normally
