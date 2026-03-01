@done
@external-bridge
@bridge
@BRIDGE-015
Feature: Platform-Agnostic Relay Bridge Endpoint

  """
  Standalone TypeScript file at bridge/relay-endpoint.ts with two WebSocket connections: (1) relay-client WS that connects TO the relay server, (2) local WS server that accepts codelet BridgeTool connections.

  ONE UNIFIED COMMAND EXECUTION PATH:
  ALL messages (input, control, command, chunk, connected) flow through the bridge WebSocket ↔ codelet bridge_relay.rs using the InboundMessage/OutboundMessage/StreamChunk event system. The relay endpoint is a PURE PROTOCOL TRANSLATOR — it translates relay protocol format (camelCase, {data:{}} envelope) to fspec flat InboundMessage format and vice versa. It does NOT call fspecCallback directly.

  COMMAND FLOW:
  1. Relay sends {type:command, session_id, request_id, data:{command, args}} to relay endpoint
  2. Relay endpoint translates to InboundMessage and forwards to codelet bridge WS
  3. bridge_relay.rs receives 'command' InboundMessage and emits FspecCommandRequest StreamChunk into the session
  4. GlobalSessionStreamManager intercepts FspecCommandRequest, calls fspecCallback (same path as LLM Fspec tool)
  5. FspecCommandResult StreamChunk flows back through session broadcast channel
  6. bridge_relay.rs intercepts FspecCommandResult (does NOT forward as regular chunk), formats as commandResponse OutboundMessage
  7. Relay endpoint translates OutboundMessage to relay protocol format and sends to relay

  KEY TYPES:
  - FspecCommandRequest (StreamChunk type) — emitted by bridge_relay.rs when command arrives, AND by session_manager.rs when LLM invokes Fspec tool
  - FspecCommandResult (StreamChunk type) — result flowing back, intercepted by bridge_relay.rs for commandResponse
  - InboundMessage (bridge_relay.rs) — extended with 'command' type, request_id, command, args_json fields
  - OutboundMessage (bridge.rs) — extended with optional request_id for commandResponse messages
  - fspecCallback (src/utils/fspec-callback.ts) — shared in-process command executor, called ONLY by GlobalSessionStreamManager

  Uses 'ws' npm package for both client and server.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Endpoint is a standalone WebSocket server in bridge/ directory (like telegram-endpoint.ts), started independently
  #   2. Endpoint connects to the relay server as a WebSocket CLIENT (relay is the server), unlike Telegram where fspec connects TO the endpoint
  #   3. Auth handshake: endpoint sends {type:auth, data:{channel_id, api_key}} on connect and waits for authSuccess/authError before processing messages
  #   4. Must handle all 5 message types: input (session→fspec), sessionControl (session→fspec), command (session→fspec), commandResponse (fspec→client), chunk (fspec→client). ALL message types are session-scoped. Commands provide a separate channel within the session for fspec CLI operations, distinct from the agent conversation (input/chunk).
  #   5. command messages are session-scoped and use request/response pattern with request_id correlation. Commands flow through the session's bridge WebSocket to Rust bridge_relay.rs, which channels them through the FspecCommandRequest/FspecCommandResult StreamChunk event system via fspecCallback. The relay endpoint does NOT call fspecCallback directly — it is a pure protocol translator.
  #   6. chunk and connected messages pass through from fspec to relay with no transformation needed
  #   7. sessionControl from relay must be renamed to 'control' and data unwrapped when forwarding to fspec bridge
  #   8. Relay protocol uses camelCase message types (sessionControl, commandResponse) matching mobile app, wraps payloads in {data:{...}} envelope. fspec InboundMessage uses flat fields (message, action) and 'control' instead of 'sessionControl'. Endpoint translates between formats. NOTE: Part 3 architecture notes use snake_case but actual implementations use camelCase.
  #   9. InboundMessage in bridge_relay.rs must be extended to support 'command' type with request_id, command name, and args_json fields. bridge_relay.rs emits FspecCommandRequest StreamChunk, GlobalSessionStreamManager handles it via fspecCallback, FspecCommandResult flows back through broadcast channel, bridge_relay.rs intercepts it and sends commandResponse OutboundMessage with matching request_id.
  #  10. Endpoint supports multiple concurrent codelet sessions. Each BridgeTool identifies itself with session_id via connected message. Endpoint maintains session_id→WebSocket map for routing.
  #  11. Endpoint initiates periodic ping messages to relay server (30s interval, matching mobile app pattern). Relay responds with pong.
  #  12. Configuration via env vars: RELAY_URL (full WebSocket URL including path, e.g. ws://relay.example.com/v1/ws), RELAY_CHANNEL_ID, RELAY_API_KEY, WEBSOCKET_PORT (local port for codelet connections, default 8080)
  #  13. commandResponse uses canonical format: {type:'commandResponse', request_id, session_id, data:{command, success, result, error}} where success and error are derived from fspec's FspecCommandResult. OutboundMessage must be extended to support request_id.
  #  14. StreamChunk data from fspec forwarded to relay without internal field transformation. fspec uses {type:'tool_call'} snake_case, mobile expects {chunkType:'toolCall'} camelCase — MOBILE-010 must handle fspec's native format.
  #  15. Messages from relay targeting unknown/disconnected session_id logged as warnings and silently dropped.
  #  16. Command execution timeout is handled within the FspecCommandRequest/FspecCommandResult pipeline — the same mechanism and timeout as when the LLM invokes the Fspec tool. The relay endpoint does not implement its own timeout.
  #
  # EXAMPLES:
  #   1. Endpoint starts, connects to relay, sends auth with channel_id and api_key, receives authSuccess with instances list
  #   2. Endpoint sends auth with invalid api_key, receives authError with code INVALID_API_KEY, logs error and exits
  #   3. codelet BridgeTool connects to endpoint WS server, sends connected message, endpoint learns session_id for routing
  #   4. Relay sends {type:input, session_id:X, data:{message:'fix bug', images:[...]}} → translates to flat InboundMessage → forwards to codelet WS
  #   5. Relay sends {type:sessionControl, session_id:X, data:{action:interrupt}} → translates to {type:control, session_id:X, action:interrupt} → forwards to codelet WS
  #   6. Relay sends {type:command, session_id:X, request_id:R1, data:{command:'board', args:{}}} → endpoint translates to InboundMessage → forwards to codelet WS → bridge_relay.rs emits FspecCommandRequest → fspecCallback executes → FspecCommandResult flows back → bridge_relay.rs sends commandResponse
  #   7. Relay sends command for unknown fspec command → fspecCallback returns error → bridge_relay.rs sends commandResponse with success:false
  #   8. codelet sends chunk StreamChunk → forwards to relay as {type:chunk, session_id:X, data:StreamChunk} with no transformation
  #   9. Endpoint starts without RELAY_URL env var → exits with clear error message
  #  10. Relay connection drops → logs warning, reconnects with exponential backoff, re-authenticates
  #  11. Relay sends command that takes too long → pipeline timeout (same as LLM path) → commandResponse with timeout error
  #  12. Relay sends input for unknown session → logs warning and drops message
  #  13. Two codelets connect with different session_ids → relay input routed to correct codelet WS
  #
  # ========================================

  Background: User Story
    As a fspec instance
    I want to connect to a platform-agnostic relay server
    So that any client (mobile, desktop, web) can interact with my sessions and run fspec commands remotely

  # ========================================
  # Authentication & Connection
  # ========================================

  @happy-path
  Scenario: Successful authentication with relay server
    Given the relay endpoint is configured with RELAY_URL, RELAY_CHANNEL_ID, and RELAY_API_KEY
    And the relay server is running
    When the endpoint starts and connects to the relay server
    And sends an auth message with channel_id and api_key
    Then the relay should respond with authSuccess
    And the endpoint should be in authenticated state
    And the endpoint should start the local WebSocket server for codelet connections

  @error
  Scenario: Authentication failure with invalid API key
    Given the relay endpoint is configured with an invalid RELAY_API_KEY
    And the relay server is running
    When the endpoint starts and connects to the relay server
    And sends an auth message with channel_id and invalid api_key
    Then the relay should respond with authError and code "INVALID_API_KEY"
    And the endpoint should log the authentication error
    And the endpoint should exit with a non-zero exit code

  @error
  Scenario: Missing required configuration
    Given the RELAY_URL environment variable is not set
    When the endpoint attempts to start
    Then it should exit with a clear error message explaining the required configuration
    And the error message should list RELAY_URL as a required variable

  # ========================================
  # Session Connection (codelet BridgeTool)
  # ========================================

  @happy-path
  Scenario: Codelet BridgeTool establishes session connection
    Given the relay endpoint is authenticated and running
    And the local WebSocket server is listening
    When a codelet BridgeTool connects to the local WebSocket server
    And sends a connected message with a session_id
    Then the endpoint should store the session_id in the session routing map
    And the endpoint should forward the connected message to the relay

  @happy-path
  Scenario: Multiple concurrent codelet sessions
    Given the relay endpoint is authenticated and running
    And codelet session "session-A" is connected
    When a second codelet BridgeTool connects with session_id "session-B"
    Then the endpoint should have both sessions in its routing map
    And messages for session-A should be routed to session-A's WebSocket only
    And messages for session-B should be routed to session-B's WebSocket only

  # ========================================
  # Input Translation (relay → fspec)
  # ========================================

  @happy-path
  Scenario: Translate relay input message to fspec format
    Given the relay endpoint is authenticated and a codelet session is connected
    When the relay sends an input message with type "input", session_id, and data containing message and images
    Then the endpoint should translate it to fspec flat InboundMessage format
    And unwrap the data envelope so message and images are top-level fields
    And forward the translated message to the codelet WebSocket connection

  # ========================================
  # Session Control Translation (relay → fspec)
  # ========================================

  @happy-path
  Scenario: Translate relay sessionControl to fspec control format
    Given the relay endpoint is authenticated and a codelet session is connected
    When the relay sends a message with type "sessionControl" and data containing action "interrupt"
    Then the endpoint should rename the type from "sessionControl" to "control"
    And unwrap the data envelope so the action field is at top level
    And forward the translated control message to the codelet WebSocket connection

  # ========================================
  # Unknown Session Handling
  # ========================================

  @error
  Scenario: Drop messages for unknown session
    Given the relay endpoint is authenticated
    And no codelet session with id "unknown-session" is connected
    When the relay sends an input message targeting session_id "unknown-session"
    Then the endpoint should log a warning about the unknown session
    And the message should be silently dropped
    And no error response should be sent to the relay

  # ========================================
  # Command Execution (relay → bridge WS → Rust → StreamChunk → fspecCallback → relay)
  # ========================================

  @happy-path
  Scenario: Execute fspec command via StreamChunk pipeline and return result
    Given the relay endpoint is authenticated and a codelet session is connected
    When the relay sends a command message with session_id, request_id, command "board", and args
    Then the endpoint should translate it to an InboundMessage with type "command" and forward to the codelet WebSocket
    And bridge_relay.rs should emit a FspecCommandRequest StreamChunk into the session
    And GlobalSessionStreamManager should handle it by calling fspecCallback
    And the FspecCommandResult should flow back through the session broadcast channel
    And bridge_relay.rs should intercept the FspecCommandResult and NOT forward it as a regular chunk
    And bridge_relay.rs should send a commandResponse OutboundMessage with the matching request_id and session_id
    And the relay endpoint should translate and forward the commandResponse to the relay

  @error
  Scenario: Command execution for unknown fspec command via StreamChunk pipeline
    Given the relay endpoint is authenticated and a codelet session is connected
    When the relay sends a command message for an unknown command "nonexistent-command"
    Then the command should flow through bridge_relay.rs into the FspecCommandRequest StreamChunk pipeline
    And fspecCallback should return a failure result
    And the FspecCommandResult with success false should flow back through the broadcast channel
    And bridge_relay.rs should send a commandResponse with success false and error message
    And the original request_id should be preserved in the response

  @error
  Scenario: Command execution timeout via StreamChunk pipeline
    Given the relay endpoint is authenticated and a codelet session is connected
    When the relay sends a command message for a long-running command
    And the command does not complete within the pipeline timeout
    Then the FspecCommandRequest/FspecCommandResult pipeline should handle the timeout
    And a FspecCommandResult with timeout error should flow back through the broadcast channel
    And bridge_relay.rs should send a commandResponse with success false and a timeout error message
    And the original request_id should be preserved in the response

  # ========================================
  # Chunk Passthrough (fspec → relay)
  # ========================================

  @happy-path
  Scenario: Forward StreamChunk from codelet to relay without transformation
    Given the relay endpoint is authenticated and a codelet session is connected
    When the codelet session sends a chunk message with StreamChunk data
    Then the endpoint should forward it to the relay as-is with no transformation
    And the chunk message should retain its type, session_id, and data fields
    And the internal StreamChunk field names should not be modified

  @happy-path
  Scenario: FspecCommandResult chunks are intercepted and not forwarded as regular chunks
    Given the relay endpoint is authenticated and a codelet session is connected
    When the codelet session's broadcast channel emits a FspecCommandResult StreamChunk
    Then bridge_relay.rs should intercept it and format it as a commandResponse OutboundMessage
    And the FspecCommandResult should NOT be forwarded to the relay as a regular chunk message

  # ========================================
  # Reconnection
  # ========================================

  @resilience
  Scenario: Reconnect to relay after connection drops
    Given the relay endpoint is authenticated and operating normally
    When the relay connection drops unexpectedly
    Then the endpoint should log a warning about the disconnection
    And attempt reconnection with exponential backoff
    And re-authenticate with the relay upon successful reconnection
    And resume normal message routing after re-authentication

  # ========================================
  # Heartbeat
  # ========================================

  @happy-path
  Scenario: Send periodic heartbeat to relay
    Given the relay endpoint is authenticated
    When 30 seconds elapse since the last ping
    Then the endpoint should send a ping message to the relay
    And process the pong response to confirm the connection is alive
