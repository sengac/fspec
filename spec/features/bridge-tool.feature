@done
@codelet
@bridge
@BRIDGE-001
Feature: Bridge Tool Integration Tests
  """
  Integration tests for Bridge tool using real WebSocket connections.
  Tests actual WebSocket behavior via TestWebSocketServer fixtures.
  Uses tokio-tungstenite for WebSocket client connections.
  """

  Background: User Story
    As a AI agent
    I want to connect my session to external WebSocket endpoints using the Bridge tool
    So that I can relay my responses to platforms like Telegram and receive remote input

  @connect
  @integration
  Scenario: Connect to a valid WebSocket endpoint
  # -------------------------------------------
  # Connect Action (Integration)
  # -------------------------------------------
    Given an agent session is running
    And a WebSocket server is listening at "ws://localhost:8080"
    When the agent calls Bridge with action "connect" and url "ws://localhost:8080"
    Then the tool should return "Connected to ws://localhost:8080"
    And the bridge should be subscribed to the session's broadcast channel

  @connect
  @error
  @integration
  Scenario: Fail to connect to invalid endpoint
    Given an agent session is running
    When the agent calls Bridge with action "connect" and url "ws://invalid:9999"
    Then the tool should return an error containing "Connection refused"

  @disconnect
  @integration
  Scenario: Disconnect from a connected endpoint
  # -------------------------------------------
  # Disconnect Action (Integration)
  # -------------------------------------------
    Given an agent session is running
    And the agent has connected a bridge to "ws://localhost:8080"
    When the agent calls Bridge with action "disconnect" and url "ws://localhost:8080"
    Then the tool should return "Disconnected from ws://localhost:8080"
    And the WebSocket connection should be closed

  @list
  @integration
  Scenario: List active bridge connections
  # -------------------------------------------
  # List Action (Integration)
  # -------------------------------------------
    Given an agent session is running
    And the agent has connected a bridge to "ws://localhost:8080"
    When the agent calls Bridge with action "list"
    Then the tool should return a list containing:
      | url                 | state     | buffered |
      | ws://localhost:8080 | connected | 0        |

  @list
  @integration
  Scenario: List connections during reconnect
    Given an agent session is running
    And the agent has connected a bridge to "ws://localhost:8080"
    And the WebSocket connection has dropped
    And the bridge is attempting to reconnect
    When the agent calls Bridge with action "list"
    Then the tool should return a list containing:
      | url                 | state        |
      | ws://localhost:8080 | reconnecting |

  @multiple
  @integration
  Scenario: Connect to multiple endpoints simultaneously
  # -------------------------------------------
  # Multiple Bridges (Integration)
  # -------------------------------------------
    Given an agent session is running
    And a WebSocket server is listening at "ws://localhost:8080"
    And a WebSocket server is listening at "ws://localhost:9090"
    When the agent calls Bridge with action "connect" and url "ws://localhost:8080"
    And the agent calls Bridge with action "connect" and url "ws://localhost:9090"
    Then both bridges should be connected
    When the agent produces a text response "Hello"
    Then "ws://localhost:8080" should receive a JSON chunk with the text "Hello"
    And "ws://localhost:9090" should receive a JSON chunk with the text "Hello"

  @outbound
  @integration
  Scenario: Relay StreamChunks to connected endpoint as JSON
  # -------------------------------------------
  # Outbound Messages (Integration)
  # -------------------------------------------
    Given an agent session is running
    And the agent has connected a bridge to "ws://localhost:8080"
    When the agent produces a text response "I can help with that"
    Then "ws://localhost:8080" should receive a JSON message with:
      | field      | value                |
      | type       | chunk                |
      | session_id | <current_session_id> |
      | data.type  | text                 |
      | data.text  | I can help with that |

  @inbound
  @integration
  Scenario: Receive input from endpoint and inject into session
  # -------------------------------------------
  # Inbound Messages (Integration)
  # -------------------------------------------
    Given an agent session is running
    And the agent has connected a bridge to "ws://localhost:8080"
    When the endpoint sends a JSON message:
      """
      {"type": "input", "session_id": "<session_id>", "message": "build the app"}
      """
    Then the agent should receive "build the app" as user input

  @reconnect
  @integration
  Scenario: Auto-reconnect and deliver buffered messages
  # -------------------------------------------
  # Reconnection & Buffering (Integration)
  # -------------------------------------------
    Given an agent session is running
    And the agent has connected a bridge to "ws://localhost:8080"
    When the WebSocket connection drops unexpectedly
    And the agent produces text responses "Message 1" and "Message 2"
    Then the bridge should buffer the messages
    When the WebSocket server becomes available again
    And the bridge reconnects
    Then "ws://localhost:8080" should receive the buffered messages in order

  @buffer-overflow
  @integration
  Scenario: Drop connection when buffer exceeds 1GB
    Given an agent session is running
    And the agent has connected a bridge to "ws://localhost:8080"
    And the WebSocket connection is down
    When the message buffer exceeds 1GB
    Then the bridge connection should be dropped
    And the tool should report an error for that connection
