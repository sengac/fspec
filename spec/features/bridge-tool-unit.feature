@done
@codelet
@bridge
@BRIDGE-001
Feature: Bridge Tool Unit Tests

  """
  Unit tests for Bridge tool data structures, state management, and tool definition.
  Tests BridgeConnection, BridgeManager, OutboundMessage, InboundMessage structures.
  These tests simulate state transitions without actual WebSocket connections.
  """

  Background: User Story
    As a AI agent
    I want to connect my session to external WebSocket endpoints using the Bridge tool
    So that I can relay my responses to platforms like Telegram and receive remote input

  # -------------------------------------------
  # Connect Action (Unit)
  # -------------------------------------------

  @connect @unit
  Scenario: Connect to a valid WebSocket endpoint
    Given an agent session is running
    And a WebSocket server is listening at "ws://localhost:8080"
    When the agent calls Bridge with action "connect" and url "ws://localhost:8080"
    Then the tool should return "Connected to ws://localhost:8080"
    And the bridge should be subscribed to the session's broadcast channel

  @connect @error @unit
  Scenario: Fail to connect to invalid endpoint
    Given an agent session is running
    When the agent calls Bridge with action "connect" and url "ws://invalid:9999"
    Then the tool should return an error containing "Connection refused"

  # -------------------------------------------
  # Disconnect Action (Unit)
  # -------------------------------------------

  @disconnect @unit
  Scenario: Disconnect from a connected endpoint
    Given an agent session is running
    And the agent has connected a bridge to "ws://localhost:8080"
    When the agent calls Bridge with action "disconnect" and url "ws://localhost:8080"
    Then the tool should return "Disconnected from ws://localhost:8080"
    And the WebSocket connection should be closed

  # -------------------------------------------
  # List Action (Unit)
  # -------------------------------------------

  @list @unit
  Scenario: List active bridge connections
    Given an agent session is running
    And the agent has connected a bridge to "ws://localhost:8080"
    When the agent calls Bridge with action "list"
    Then the tool should return a list containing:
      | url                   | state     | buffered |
      | ws://localhost:8080   | connected | 0        |

  @list @unit
  Scenario: List connections during reconnect
    Given an agent session is running
    And the agent has connected a bridge to "ws://localhost:8080"
    And the WebSocket connection has dropped
    And the bridge is attempting to reconnect
    When the agent calls Bridge with action "list"
    Then the tool should return a list containing:
      | url                   | state        |
      | ws://localhost:8080   | reconnecting |

  # -------------------------------------------
  # Multiple Bridges (Unit)
  # -------------------------------------------

  @multiple @unit
  Scenario: Connect to multiple endpoints simultaneously
    Given an agent session is running
    And a WebSocket server is listening at "ws://localhost:8080"
    And a WebSocket server is listening at "ws://localhost:9090"
    When the agent calls Bridge with action "connect" and url "ws://localhost:8080"
    And the agent calls Bridge with action "connect" and url "ws://localhost:9090"
    Then both bridges should be connected
    When the agent produces a text response "Hello"
    Then "ws://localhost:8080" should receive a JSON chunk with the text "Hello"
    And "ws://localhost:9090" should receive a JSON chunk with the text "Hello"

  # -------------------------------------------
  # Outbound Messages (Unit)
  # -------------------------------------------

  @outbound @unit
  Scenario: Relay StreamChunks to connected endpoint as JSON
    Given an agent session is running
    And the agent has connected a bridge to "ws://localhost:8080"
    When the agent produces a text response "I can help with that"
    Then "ws://localhost:8080" should receive a JSON message with:
      | field      | value                        |
      | type       | chunk                        |
      | session_id | <current_session_id>         |
      | data.type  | text                         |
      | data.text  | I can help with that         |

  # -------------------------------------------
  # Inbound Messages (Unit)
  # -------------------------------------------

  @inbound @unit
  Scenario: Receive input from endpoint and inject into session
    Given an agent session is running
    And the agent has connected a bridge to "ws://localhost:8080"
    When the endpoint sends a JSON message:
      """
      {"type": "input", "session_id": "<session_id>", "message": "build the app"}
      """
    Then the agent should receive "build the app" as user input

  # -------------------------------------------
  # Reconnection & Buffering (Unit)
  # -------------------------------------------

  @reconnect @unit
  Scenario: Auto-reconnect and deliver buffered messages
    Given an agent session is running
    And the agent has connected a bridge to "ws://localhost:8080"
    When the WebSocket connection drops unexpectedly
    And the agent produces text responses "Message 1" and "Message 2"
    Then the bridge should buffer the messages
    When the WebSocket server becomes available again
    And the bridge reconnects
    Then "ws://localhost:8080" should receive the buffered messages in order

  @buffer-overflow @unit
  Scenario: Drop connection when buffer exceeds 1GB
    Given an agent session is running
    And the agent has connected a bridge to "ws://localhost:8080"
    And the WebSocket connection is down
    When the message buffer exceeds 1GB
    Then the bridge connection should be dropped
    And the tool should report an error for that connection

  # -------------------------------------------
  # Tool Definition (Unit)
  # -------------------------------------------

  @tool-definition @unit
  Scenario: Bridge tool definition
    Given a BridgeTool instance
    When definition is called
    Then the name should be "Bridge"
    And the description should contain "WebSocket"

  @tool-definition @unit
  Scenario: Bridge tool requires session context
    Given a BridgeTool instance
    When call is invoked directly
    Then an error should mention "session context"

  @session-context @unit
  Scenario: Bridge wrapper uses current session ID from handler
    Given the bridge handler is configured
    And the current bridge session is set to a valid session ID
    When the BridgeToolFacadeWrapper executes a list action
    Then the request should contain the correct session ID
