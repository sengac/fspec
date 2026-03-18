@BRIDGE-017
Feature: Command handling pipeline in bridge_relay.rs
  """
  Changes span 4 files across 2 crates: (1) bridge_relay.rs: add CommandEmitter type, extend spawn_relay_task/relay_loop/connect_and_relay/handle_inbound_message with command_emitter + pending_commands params, add command handling in MSG_TYPE_COMMAND match arm, modify outbound loop to intercept fspecCommandResult/fspecCommandRequest chunks. (2) bridge_handler.rs: extend BridgeSessionContext + set_bridge_session_context with command_emitter. (3) lib.rs: export CommandEmitter. (4) session_manager.rs: create command_emitter closure using session.handle_output(StreamChunk::fspec_command_request(...)), pass to set_bridge_session_context. Uses Arc<Mutex<HashMap<String, (String, String)>>> for pending_commands (key=tool_call_id, value=(request_id, command_name)), created per-connection.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When bridge_relay.rs receives an InboundMessage with type 'command', it must generate a UUID tool_call_id, store a mapping of tool_call_id → (request_id, command_name) in a pending_commands HashMap, and call the command_emitter callback with (command, args_json, project_root, tool_call_id)
  #   2. CommandEmitter is a new callback type: Arc<dyn Fn(String, String, String, String) + Send + Sync> taking (command, args_json, project_root, tool_call_id). It is optional on spawn_relay_task, relay_loop, connect_and_relay, and handle_inbound_message
  #   3. In the outbound broadcast loop, FspecCommandResult chunks (type == 'fspecCommandResult') must be intercepted and NOT forwarded as regular chunks. Instead, bridge_relay.rs looks up the tool_call_id in the pending_commands map, retrieves the original request_id and command name, and sends a commandResponse OutboundMessage
  #   4. FspecCommandRequest chunks on the broadcast channel must be silently ignored (not forwarded as chunks). They are meant for TypeScript's GlobalSessionStreamManager, not for bridge endpoints
  #   5. commandResponse OutboundMessage format: {type:'commandResponse', session_id, request_id, data:{command, success, result, error}} where result is parsed JSON if possible, otherwise a JSON string
  #   6. BridgeSessionContext must be extended with command_emitter: Option<CommandEmitter>. set_bridge_session_context must accept the new parameter. handle_bridge_action must pass it through to spawn_relay_task
  #   7. session_manager.rs must create a command_emitter closure that calls session.handle_output(StreamChunk::fspec_command_request(...)) — the same path used when the LLM invokes the Fspec tool. The emitter is fire-and-forget (does not block)
  #   8. If no command_emitter is configured when a command message arrives, bridge_relay.rs logs a warning and returns Ok(()) — it does not crash
  #   9. project_root for the FspecCommandRequest is derived from std::env::current_dir() since the fspec process CWD is always the project root
  #   10. CommandEmitter type must be exported from lib.rs alongside existing bridge_relay exports
  #
  # EXAMPLES:
  #   1. Bridge endpoint sends {type:command, session_id:s1, request_id:r1, command:board, args_json:'{}'} → bridge_relay.rs generates tool_call_id uuid, stores r1→uuid mapping, calls command_emitter('board','{}','/project',uuid) → emitter fires FspecCommandRequest → GlobalSessionStreamManager calls fspecCallback → FspecCommandResult flows back on broadcast → bridge_relay.rs intercepts, looks up r1, sends commandResponse with request_id:r1, success:true, result:{...}
  #   2. Command message arrives but no command_emitter is configured (e.g., old session setup) → bridge_relay.rs logs warning 'Received command but no command emitter configured' and returns Ok(())
  #   3. FspecCommandResult chunk with toolCallId abc-123 arrives on broadcast channel and abc-123 is in pending_commands map with request_id r1 → bridge_relay.rs removes abc-123 from map, sends commandResponse with request_id:r1, does NOT forward the chunk as a regular chunk message
  #   4. FspecCommandRequest chunk arrives on broadcast channel → bridge_relay.rs recognizes type 'fspecCommandRequest', silently skips it (does not forward as chunk, does not log error), continues processing next chunk
  #   5. Regular text chunk (type 'text') arrives on broadcast channel → forwarded as normal OutboundMessage {type:'chunk'} with request_id:None — existing behavior unchanged
  #   6. FspecCommandResult with toolCallId xyz arrives but xyz is NOT in pending_commands (e.g., it was for a different session's LLM fspec call) → bridge_relay.rs silently skips it without crashing
  #
  # ========================================
  Background: User Story
    As a bridge endpoint
    I want to send fspec commands via the bridge WebSocket and receive results back
    So that remote clients can execute fspec CLI operations through the established session pipeline

  @inbound
  @happy-path
  Scenario: Handle command InboundMessage by emitting FspecCommandRequest
    Given a bridge relay is connected with a command_emitter configured
    And a pending_commands map is initialized for the connection
    When the relay receives an InboundMessage with type "command", request_id "r1", command "board", and args_json "{}"
    Then a UUID tool_call_id should be generated
    And the pending_commands map should contain a mapping from the tool_call_id to request_id "r1" and command "board"
    And the command_emitter should be called with command "board", args_json "{}", a project_root from current_dir, and the generated tool_call_id

  @inbound
  @graceful-degradation
  Scenario: Handle command InboundMessage without command_emitter configured
    Given a bridge relay is connected without a command_emitter
    When the relay receives an InboundMessage with type "command", request_id "r2", command "board", and args_json "{}"
    Then no emitter callback should be invoked
    And the handler should return Ok without crashing
    And a warning should be logged about no command emitter being configured

  @outbound
  @interception
  Scenario: Intercept FspecCommandResult chunk and send commandResponse
    Given a bridge relay has a pending command with tool_call_id "abc-123" mapped to request_id "r1" and command "board"
    When a FspecCommandResult chunk with toolCallId "abc-123", success true, and data containing results arrives on the broadcast channel
    Then the pending_commands map entry for "abc-123" should be removed
    And a commandResponse OutboundMessage should be sent with request_id "r1"
    And the commandResponse data should contain command "board", success true, and the result value
    And the FspecCommandResult chunk should NOT be forwarded as a regular "chunk" message

  @outbound
  @filtering
  Scenario: Skip FspecCommandRequest chunks on broadcast channel
    Given a bridge relay is connected and processing outbound chunks
    When a FspecCommandRequest chunk with type "fspecCommandRequest" arrives on the broadcast channel
    Then the chunk should be silently skipped
    And no OutboundMessage should be sent to the WebSocket for this chunk

  @outbound
  @backward-compatibility
  Scenario: Forward regular text chunks unchanged
    Given a bridge relay is connected and processing outbound chunks
    When a regular text chunk with type "text" arrives on the broadcast channel
    Then an OutboundMessage with type "chunk" and request_id None should be sent to the WebSocket
    And the chunk data should be forwarded without modification

  @outbound
  @edge-case
  Scenario: Skip FspecCommandResult with unknown tool_call_id
    Given a bridge relay has an empty pending_commands map
    When a FspecCommandResult chunk with toolCallId "xyz-unknown" arrives on the broadcast channel
    Then no commandResponse should be sent
    And the chunk should be silently skipped without crashing
    And the FspecCommandResult should NOT be forwarded as a regular chunk
