@BRIDGE-018
Feature: Relay endpoint command flow through bridge WebSocket
  """
  Pure refactoring of relay-endpoint.ts command handler (lines 224-245): replace executeCommand() call with WebSocket send of translated InboundMessage. Delete relay-command-executor.ts. Add command fields to FspecInboundMessage in relay-types.ts. Remove unused CommandResult/CommandResponseMessage types. Update tests to verify: (1) command translated to InboundMessage and sent to codelet WS, (2) commandResponse from codelet forwarded to relay, (3) command for unknown session dropped. Tests for timeout/error are now Rust-side (BRIDGE-017).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The relay endpoint's command handler must translate relay command messages to flat InboundMessage format {type:'command', session_id, message:'', request_id, command, args_json} and forward through the rust's bridge WebSocket — NOT call fspecCallback directly
  #   2. relay-command-executor.ts must be deleted entirely — it directly imports fspecCallback which violates the pure protocol translator architecture
  #   3. All imports and re-exports of executeCommand must be removed from relay-endpoint.ts
  #   4. FspecInboundMessage in relay-types.ts must include optional command fields: request_id, command, args_json — to match the InboundMessage format expected by bridge_relay.rs
  #   5. Command messages for unknown/disconnected sessions must be logged as warnings and silently dropped — same behavior as input and sessionControl messages
  #   6. commandResponse messages from the codelet WebSocket are already forwarded to the relay by the existing passthrough handler — no change needed for that direction
  #   7. CommandResult interface and CommandResponseMessage interface in relay-types.ts should be removed since they are no longer used (command execution now handled by Rust pipeline)
  #   8. No bridge/ TypeScript file should import from src/utils/fspec-callback — the bridge/ directory is a standalone process boundary
  #
  # EXAMPLES:
  #   1. Relay sends {type:command, session_id:X, request_id:R1, data:{command:'board', args:{}}} → endpoint translates to {type:'command', session_id:X, message:'', request_id:R1, command:'board', args_json:'{}'} → sends to codelet WS for session X
  #   2. Relay sends command for unknown session_id Y → endpoint logs warning 'No codelet connection for session Y (command)' and drops the message
  #   3. Codelet sends commandResponse OutboundMessage {type:'commandResponse', session_id:X, request_id:R1, data:{command:'board', success:true, result:{...}}} → existing passthrough handler forwards it to relay unchanged
  #   4. After refactor: no TypeScript file in bridge/ directory imports from '../src/utils/fspec-callback' — grep returns zero results
  #   5. relay-command-executor.ts file no longer exists on disk after implementation
  #
  # ========================================
  Background: User Story
    As a relay endpoint
    I want to forward command messages through the codelet bridge WebSocket to bridge_relay.rs
    So that commands follow the same FspecCommandRequest/FspecCommandResult pipeline as LLM-invoked Fspec tool calls, making the relay endpoint a pure protocol translator

  # ========================================
  # Command Translation (relay → codelet WS)
  # ========================================
  @happy-path
  Scenario: Translate relay command message to InboundMessage and forward to codelet WebSocket
    Given the relay endpoint is authenticated and running
    And a codelet session "session-X" is connected via the local WebSocket server
    When the relay sends a command message with type "command", session_id "session-X", request_id "R1", and data containing command "board" and args {}
    Then the endpoint should translate it to a flat InboundMessage with type "command", session_id "session-X", message "", request_id "R1", command "board", and args_json "{}"
    And forward the translated InboundMessage to the codelet WebSocket for session "session-X"
    And the endpoint should NOT call fspecCallback directly

  @error
  Scenario: Drop command message for unknown session
  # ========================================
  # Unknown Session Handling for Commands
  # ========================================
    Given the relay endpoint is authenticated and running
    And no codelet session with id "unknown-session" is connected
    When the relay sends a command message targeting session_id "unknown-session"
    Then the endpoint should log a warning containing "No codelet connection for session unknown-session"
    And the command message should be silently dropped
    And no message should be sent to the relay

  @happy-path
  Scenario: Forward commandResponse from codelet to relay unchanged
  # ========================================
  # commandResponse Passthrough (codelet → relay)
  # ========================================
    Given the relay endpoint is authenticated and running
    And a codelet session "session-X" is connected via the local WebSocket server
    When the codelet sends a commandResponse message with type "commandResponse", session_id "session-X", request_id "R1", and data containing command "board", success true, and result
    Then the endpoint should forward the commandResponse to the relay without modification
    And the forwarded message should preserve the request_id "R1"
    And the forwarded message should preserve the data fields

  @architecture
  Scenario: No bridge TypeScript files import fspecCallback
  # ========================================
  # Architecture Constraints
  # ========================================
    Given the relay endpoint refactoring is complete
    When searching for imports of "fspec-callback" in the bridge directory
    Then no TypeScript file in the bridge directory should import from "src/utils/fspec-callback"

  @architecture
  Scenario: relay-command-executor.ts is deleted
    Given the relay endpoint refactoring is complete
    When checking for the file "bridge/relay-command-executor.ts"
    Then the file should not exist on disk
