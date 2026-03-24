@done
@BRIDGE-019
Feature: Relay Server — Local WebSocket hub for routing messages between relay-endpoint and mobile app
  """
  Standalone TypeScript file at bridge/relay-server.ts using 'ws' npm package (already a dependency). Single file, under 300 lines. Uses dotenv for config. NPM scripts: bridge:server (foreground), bridge:server:bg (background), bridge:server:stop. Server maintains a Map<channelId, Set<WebSocket>> for channel routing. Each WebSocket also stores its channelId and authenticated state. The full local stack: (1) npm run bridge:server on port 8765, (2) npm run bridge:relay connects relay-endpoint to ws://localhost:8765, (3) mobile app connects to http://localhost:8765. relay-endpoint.ts RELAY_URL must point at the relay server, NOT at itself.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Relay server is a standalone TypeScript WebSocket server in bridge/relay-server.ts, started via npm run bridge:server
  #   2. Server listens on a configurable port (default 8765) and accepts WebSocket connections at any path (including /v1/ws for mobile app compatibility)
  #   3. Auth handshake: client sends {type:'auth', data:{channel_id, api_key}} → server validates and responds with {type:'authSuccess', data:{instances:[...]}} or {type:'authError', data:{code, message}}
  #   4. Channel-based routing: clients on the same channel_id form a channel group. Messages from one client are forwarded to all OTHER clients in the same channel (not echoed back to sender)
  #   5. Server is a pure message router — it does NOT inspect, transform, or interpret message payloads. It forwards the raw JSON to other clients in the channel.
  #   6. All 5 session-scoped message types pass through: input, sessionControl, command, commandResponse, chunk — plus connected, ping/pong
  #   7. Server responds to ping with pong directly (heartbeat is server-side, not forwarded to other clients)
  #   8. When relay-endpoint sends a 'connected' message (indicating a new codelet session), server broadcasts it to all other clients in the channel so mobile app discovers available sessions
  #   9. On authSuccess, server sends the current list of known instances/sessions for that channel so newly connecting clients get immediate state
  #   10. Configuration via env vars: RELAY_SERVER_PORT (default 8765), RELAY_SERVER_API_KEY (optional — if set, clients must provide matching api_key; if unset, any api_key accepted)
  #   11. Unauthenticated clients that send non-auth messages receive an error and are disconnected
  #   12. Server logs client connections, disconnections, auth events, and message routing to console with [relay-server] prefix
  #   13. When a client disconnects, it is removed from its channel. If it was the relay-endpoint (the one that sent 'connected' messages), the server does NOT notify remaining clients — the mobile app handles stale sessions via its own heartbeat/reconnect logic
  #
  # EXAMPLES:
  #   1. Start server with npm run bridge:server → server listens on ws://localhost:8765, logs '[relay-server] Listening on port 8765'
  #   2. Mobile app connects with {type:'auth', data:{channel_id:'my-project', api_key:'secret'}} → server validates api_key matches RELAY_SERVER_API_KEY env var → responds {type:'authSuccess', data:{instances:[]}} (no fspec instances yet)
  #   3. relay-endpoint connects to server with same channel_id, authenticates → server adds it to channel → relay-endpoint sends {type:'connected', session_id:'sess-1'} → server broadcasts 'connected' to mobile app → mobile app now knows session 'sess-1' exists
  #   4. Mobile app sends {type:'input', session_id:'sess-1', data:{message:'fix the bug'}} → server forwards raw JSON to relay-endpoint (other client in same channel) → relay-endpoint receives it and translates to fspec InboundMessage
  #   5. relay-endpoint sends {type:'chunk', session_id:'sess-1', data:{type:'text', text:'I found...'}} → server forwards raw JSON to mobile app → mobile app renders the AI output
  #   6. Mobile app sends {type:'command', session_id:'sess-1', request_id:'r1', data:{command:'board', args:{}}} → server forwards to relay-endpoint → fspec processes → relay-endpoint sends {type:'commandResponse', ...} → server forwards to mobile app
  #   7. Client sends auth with wrong api_key when RELAY_SERVER_API_KEY is set → server responds {type:'authError', data:{code:'INVALID_API_KEY', message:'Invalid API key'}} and closes connection
  #   8. RELAY_SERVER_API_KEY not set → server accepts any api_key value (open mode for local dev)
  #   9. Client sends {type:'ping'} → server responds {type:'pong'} directly, does NOT forward ping to other clients
  #   10. Unauthenticated client sends {type:'input', ...} before auth → server responds {type:'authError', data:{code:'NOT_AUTHENTICATED', message:'Must authenticate first'}} and closes connection
  #   11. relay-endpoint disconnects → server removes it from channel, mobile app eventually notices via heartbeat timeout on its side
  #   12. Two different channel_ids (project-A and project-B) — messages from project-A clients are NEVER forwarded to project-B clients
  #
  # ========================================
  Background: User Story
    As a developer running fspec locally
    I want to start a relay server on this computer
    So that the relay-endpoint and mobile app can both connect to it and exchange real messages without needing an external hosted service

  @server-startup
  Scenario: Server starts and listens on configured port
    Given the RELAY_SERVER_PORT environment variable is set to "8765"
    When I start the relay server
    Then the server should listen for WebSocket connections on port 8765
    And the server should log "[relay-server] Listening on port 8765"

  @auth
  @happy-path
  Scenario: Successful authentication with valid API key
    Given the relay server is running with RELAY_SERVER_API_KEY set to "secret"
    When a client connects and sends auth with channel_id "my-project" and api_key "secret"
    Then the server should respond with authSuccess
    And the authSuccess data should include an instances array

  @auth
  @happy-path
  Scenario: Open mode authentication when no API key is configured
    Given the relay server is running without RELAY_SERVER_API_KEY set
    When a client connects and sends auth with channel_id "my-project" and api_key "anything"
    Then the server should respond with authSuccess

  @auth
  @error
  Scenario: Authentication failure with invalid API key
    Given the relay server is running with RELAY_SERVER_API_KEY set to "secret"
    When a client connects and sends auth with channel_id "my-project" and api_key "wrong"
    Then the server should respond with authError code "INVALID_API_KEY"
    And the server should close the connection

  @auth
  @error
  Scenario: Unauthenticated client sends non-auth message
    Given the relay server is running
    And a client is connected but has not authenticated
    When the client sends an input message
    Then the server should respond with authError code "NOT_AUTHENTICATED"
    And the server should close the connection

  @routing
  @session
  Scenario: Connected message broadcast to channel peers
    Given the relay server is running
    And client A is authenticated on channel "my-project"
    And client B is authenticated on channel "my-project"
    When client A sends a connected message with session_id "sess-1"
    Then client B should receive the connected message with session_id "sess-1"
    And client A should NOT receive the connected message back

  @routing
  @session
  Scenario: Route input message to channel peers
    Given the relay server is running
    And client A is authenticated on channel "my-project"
    And client B is authenticated on channel "my-project"
    When client A sends an input message for session_id "sess-1"
    Then client B should receive the exact same JSON message
    And client A should NOT receive the message back

  @routing
  @session
  Scenario: Route chunk message to channel peers
    Given the relay server is running
    And client A is authenticated on channel "my-project"
    And client B is authenticated on channel "my-project"
    When client A sends a chunk message with session_id "sess-1" and text data
    Then client B should receive the exact same JSON message

  @routing
  @session
  Scenario: Route command and commandResponse between channel peers
    Given the relay server is running
    And client A is authenticated on channel "my-project"
    And client B is authenticated on channel "my-project"
    When client A sends a command message with request_id "r1"
    Then client B should receive the command message with request_id "r1"
    When client B sends a commandResponse with request_id "r1"
    Then client A should receive the commandResponse with request_id "r1"

  @routing
  @isolation
  Scenario: Channel isolation prevents cross-channel message leaking
    Given the relay server is running
    And client A is authenticated on channel "project-A"
    And client B is authenticated on channel "project-B"
    When client A sends a chunk message
    Then client B should NOT receive any message

  @heartbeat
  Scenario: Server responds to ping with pong directly
    Given the relay server is running
    And a client is authenticated on channel "my-project"
    When the client sends a ping message
    Then the server should respond with a pong message
    And other clients in the channel should NOT receive the ping

  @disconnect
  Scenario: Client removed from channel on disconnect
    Given the relay server is running
    And client A is authenticated on channel "my-project"
    And client B is authenticated on channel "my-project"
    When client A disconnects
    And client B sends a chunk message
    Then client A should NOT receive the message
    And the server should log the disconnection

  @auth
  @state
  Scenario: Newly connecting client receives current instance list
    Given the relay server is running
    And client A is authenticated on channel "my-project"
    And client A has sent a connected message with session_id "sess-1"
    When client B authenticates on channel "my-project"
    Then client B should receive authSuccess with instances containing session_id "sess-1"

  @routing
  @session
  Scenario: Route sessionControl message to channel peers
    Given the relay server is running
    And client A is authenticated on channel "my-project"
    And client B is authenticated on channel "my-project"
    When client A sends a sessionControl message for session_id "sess-1"
    Then client B should receive the exact same JSON message
    And client A should NOT receive the message back

