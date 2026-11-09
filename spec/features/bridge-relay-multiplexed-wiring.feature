@done
@BUG-513
@tools
@bridge
@rust
Feature: Bridge relay multiplexed wiring
  """
  rust/tools/src/bridge_relay.rs speaks only the multiplexed envelope protocol
  of rust/tools/src/bridge_multiplexed.rs (service + type + optional
  instance_id / session_id / request_id + data); the flat protocol is gone.
  The bridge authenticates with an auth envelope built from its instance
  metadata, wraps outbound stream chunks as relay envelopes and fspec command
  results as fspec commandResponse envelopes (matched to the pending command
  by tool call id), dispatches inbound session input, session control and
  fspec commands to the injected InputInjector, ControlHandler and
  CommandEmitter, and answers a system ping with a pong.
  The unit tests were added with the bridge multiplexing overhaul (5811c8ca5,
  no card id; the code cites ARCH-004, which is not on this board) and named
  this feature before it existed; BUG-513 wrote it. Each
  scenario states what its unit test checks: the tests build and route
  envelopes directly and never open a WebSocket connection.
  Tests: rust/tools/src/bridge_multiplexed_wiring_tests.rs.
  """

  Background: User Story
    As a maintainer of the bridge relay
    I want to have the relay wiring speak only multiplexed envelopes
    So that every message to and from the relay server has one shape and the flat protocol cannot come back

  Scenario: Bridge builds its auth envelope from the instance metadata
    Given the bridge's instance metadata for the current working directory
    When the bridge builds its auth envelope
    Then the envelope has service "auth" and type "authenticate"
    And the auth data contains role "agent" and the instance metadata
    And the instance name is the name of the current working directory

  Scenario: An authSuccess envelope is routed as a successful auth response
    Given an authSuccess envelope from the relay server
    When the bridge routes it
    Then it is an auth response that reports success

  Scenario: An authError envelope is routed as a failed auth response
    Given an authError envelope from the relay server with code "AUTH_FAILED"
    When the bridge routes it
    Then it is an auth response that reports failure

  Scenario: Outbound chunks are sent as relay envelopes
    Given the bridge is authenticated with instance_id "my-project"
    And a session is producing stream chunks
    When a text chunk from the session is processed for sending
    Then the bridge sends an Envelope with service "relay" and type "chunk"
    And the envelope includes instance_id "my-project" and the session's id
    And the data field contains the original StreamChunk

  Scenario: Outbound command results are sent as fspec commandResponse envelopes
    Given the bridge is authenticated with instance_id "my-project"
    And a pending command with request_id "r1" for command "board" is recorded under tool call id "tool-123"
    When an fspecCommandResult chunk for tool call "tool-123" is processed for sending
    Then the bridge sends an Envelope with service "fspec" and type "commandResponse"
    And the envelope includes instance_id "my-project" and request_id "r1"
    And the data contains command "board" and the result's success

  Scenario: Inbound session input is dispatched to the InputInjector
    Given the bridge has an InputInjector for a session
    When the server sends an Envelope with service "session" and type "input" for that session
    And the data contains message "hello" and an image
    Then the InputInjector receives message "hello" and the image

  Scenario: Inbound session control is dispatched to the ControlHandler
    Given the bridge has a ControlHandler for a session
    When the server sends an Envelope with service "session" and type "control" for that session
    And the data contains action "interrupt"
    Then the ControlHandler receives action "interrupt"

  Scenario: Inbound fspec commands are dispatched to the CommandEmitter
    Given the bridge has a CommandEmitter and an empty pending-command map
    When the server sends an Envelope with service "fspec" and type "command" and request_id "r1"
    And the data contains command "board" and args
    Then the CommandEmitter receives command "board"
    And one pending command is recorded

  Scenario: A system ping is answered with a pong
    Given the bridge is handling inbound envelopes for a session
    When the server sends an Envelope with service "system" and type "ping"
    Then the bridge answers with an Envelope with service "system" and type "pong"

  Scenario: The flat protocol code is deleted
    Given the bridge_relay.rs source code
    When it is checked for the flat protocol
    Then there is no send_connected_message function
    And there is no flat InboundMessage struct
    And there is no is_multiplexed_endpoint branching
    And there is no flat "connected" message
