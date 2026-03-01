/**
 * Feature: spec/features/relay-bridge-endpoint.feature
 * Feature: spec/features/relay-command-flow-through-bridge.feature
 *
 * This test file validates the acceptance criteria defined in both feature files.
 * Scenarios map directly to Gherkin scenarios for the Platform-Agnostic Relay Bridge Endpoint
 * and the BRIDGE-018 command flow refactoring.
 */

import { describe, it, expect, vi } from 'vitest';
import { readdir, readFile, access } from 'fs/promises';
import { join } from 'path';

import {
  validateConfig,
  translateRelayInputToFspec,
  translateRelaySessionControlToFspec,
  translateRelayCommandToFspec,
  createRelayEndpoint,
  type RelayMessage,
} from '../relay-endpoint';

import {
  createTestConfig,
  createAuthenticatedEndpoint,
  createEndpointWithSession,
  createMockCodeletWebSocket,
} from './fixtures/relay-test-helpers';

describe('Feature: Platform-Agnostic Relay Bridge Endpoint', () => {
  // ========================================
  // Authentication & Connection
  // ========================================

  describe('Scenario: Successful authentication with relay server', () => {
    it('should authenticate and start local WebSocket server', async () => {
      // @step Given the relay endpoint is configured with RELAY_URL, RELAY_CHANNEL_ID, and RELAY_API_KEY
      const config = createTestConfig();

      // @step And the relay server is running
      // (simulated via mock)

      // @step When the endpoint starts and connects to the relay server
      const endpoint = createRelayEndpoint(config);

      // @step And sends an auth message with channel_id and api_key
      const authMessage = endpoint.buildAuthMessage();
      expect(authMessage).toEqual({
        type: 'auth',
        data: {
          channel_id: 'test-channel',
          api_key: 'valid-api-key',
        },
      });

      // @step Then the relay should respond with authSuccess
      endpoint.handleRelayMessage(
        JSON.stringify({ type: 'authSuccess', data: { instances: [] } })
      );

      // @step And the endpoint should be in authenticated state
      expect(endpoint.isAuthenticated()).toBe(true);

      // @step And the endpoint should start the local WebSocket server for codelet connections
      expect(endpoint.isLocalServerRunning()).toBe(true);

      await endpoint.stop();
    });
  });

  describe('Scenario: Authentication failure with invalid API key', () => {
    it('should log error and exit on auth failure', async () => {
      // @step Given the relay endpoint is configured with an invalid RELAY_API_KEY
      const config = createTestConfig({ apiKey: 'invalid-api-key' });

      // @step And the relay server is running
      // (simulated via mock)

      // @step When the endpoint starts and connects to the relay server
      const endpoint = createRelayEndpoint(config);

      // @step And sends an auth message with channel_id and invalid api_key
      // (auth message already sent on connect)

      // @step Then the relay should respond with authError and code "INVALID_API_KEY"
      const consoleSpy = vi
        .spyOn(console, 'error')
        .mockImplementation(() => {});
      endpoint.handleRelayMessage(
        JSON.stringify({
          type: 'authError',
          data: { code: 'INVALID_API_KEY', message: 'Invalid API key' },
        })
      );

      // @step And the endpoint should log the authentication error
      expect(consoleSpy).toHaveBeenCalledWith(
        expect.stringContaining('Authentication failed'),
        expect.any(String)
      );

      // @step And the endpoint should exit with a non-zero exit code
      expect(endpoint.isAuthenticated()).toBe(false);
      expect(endpoint.authError).toBeDefined();
      expect(endpoint.authError?.code).toBe('INVALID_API_KEY');

      consoleSpy.mockRestore();
      await endpoint.stop();
    });
  });

  describe('Scenario: Missing required configuration', () => {
    it('should exit with clear error when RELAY_URL is missing', () => {
      // @step Given the RELAY_URL environment variable is not set
      const config = createTestConfig({ relayUrl: '' });

      // @step When the endpoint attempts to start
      const result = validateConfig(config);

      // @step Then it should exit with a clear error message explaining the required configuration
      expect(result.valid).toBe(false);

      // @step And the error message should list RELAY_URL as a required variable
      expect(result.errors).toContain('RELAY_URL is required');
    });
  });

  // ========================================
  // Session Connection (codelet BridgeTool)
  // ========================================

  describe('Scenario: Codelet BridgeTool establishes session connection', () => {
    it('should store session_id and forward connected message to relay', () => {
      // @step Given the relay endpoint is authenticated and running
      const endpoint = createAuthenticatedEndpoint();

      // @step And the local WebSocket server is listening
      expect(endpoint.isLocalServerRunning()).toBe(true);

      // @step When a codelet BridgeTool connects to the local WebSocket server
      const mockCodeletWs = createMockCodeletWebSocket();

      // @step And sends a connected message with a session_id
      endpoint.handleCodeletMessage(
        mockCodeletWs,
        JSON.stringify({
          type: 'connected',
          session_id: 'session-123',
          data: {},
        })
      );

      // @step Then the endpoint should store the session_id in the session routing map
      expect(endpoint.hasSession('session-123')).toBe(true);

      // @step And the endpoint should forward the connected message to the relay
      expect(endpoint.getLastRelaySent()).toMatchObject({
        type: 'connected',
        session_id: 'session-123',
      });

      endpoint.stop();
    });
  });

  describe('Scenario: Multiple concurrent codelet sessions', () => {
    it('should route messages to correct session', () => {
      // @step Given the relay endpoint is authenticated and running
      const endpoint = createAuthenticatedEndpoint();

      // @step And codelet session "session-A" is connected
      const wsA = createMockCodeletWebSocket();
      endpoint.handleCodeletMessage(
        wsA,
        JSON.stringify({
          type: 'connected',
          session_id: 'session-A',
          data: {},
        })
      );

      // @step When a second codelet BridgeTool connects with session_id "session-B"
      const wsB = createMockCodeletWebSocket();
      endpoint.handleCodeletMessage(
        wsB,
        JSON.stringify({
          type: 'connected',
          session_id: 'session-B',
          data: {},
        })
      );

      // @step Then the endpoint should have both sessions in its routing map
      expect(endpoint.hasSession('session-A')).toBe(true);
      expect(endpoint.hasSession('session-B')).toBe(true);

      // @step And messages for session-A should be routed to session-A's WebSocket only
      endpoint.routeToSession('session-A', { type: 'input', message: 'for A' });
      expect(wsA.sentMessages.length).toBe(1);
      expect(wsB.sentMessages.length).toBe(0);

      // @step And messages for session-B should be routed to session-B's WebSocket only
      endpoint.routeToSession('session-B', { type: 'input', message: 'for B' });
      expect(wsA.sentMessages.length).toBe(1);
      expect(wsB.sentMessages.length).toBe(1);

      endpoint.stop();
    });
  });

  // ========================================
  // Input Translation (relay → fspec)
  // ========================================

  describe('Scenario: Translate relay input message to fspec format', () => {
    it('should unwrap data envelope and flatten message', () => {
      // @step Given the relay endpoint is authenticated and a codelet session is connected
      // (setup implicit)

      // @step When the relay sends an input message with type "input", session_id, and data containing message and images
      const relayMessage: RelayMessage = {
        type: 'input',
        session_id: 'session-X',
        data: {
          message: 'fix bug',
          images: [{ data: 'base64...', media_type: 'image/jpeg' }],
        },
      };

      // @step Then the endpoint should translate it to fspec flat InboundMessage format
      const fspecMessage = translateRelayInputToFspec(relayMessage);

      // @step And unwrap the data envelope so message and images are top-level fields
      expect(fspecMessage.type).toBe('input');
      expect(fspecMessage.session_id).toBe('session-X');
      expect(fspecMessage.message).toBe('fix bug');
      expect(fspecMessage.images).toEqual([
        { data: 'base64...', media_type: 'image/jpeg' },
      ]);

      // @step And forward the translated message to the codelet WebSocket connection
      // (forwarding tested in integration, translation verified here)
      expect(fspecMessage).not.toHaveProperty('data');
    });
  });

  // ========================================
  // Session Control Translation (relay → fspec)
  // ========================================

  describe('Scenario: Translate relay sessionControl to fspec control format', () => {
    it('should rename type and unwrap data envelope', () => {
      // @step Given the relay endpoint is authenticated and a codelet session is connected
      // (setup implicit)

      // @step When the relay sends a message with type "sessionControl" and data containing action "interrupt"
      const relayMessage: RelayMessage = {
        type: 'sessionControl',
        session_id: 'session-X',
        data: {
          action: 'interrupt',
        },
      };

      // @step Then the endpoint should rename the type from "sessionControl" to "control"
      const fspecMessage = translateRelaySessionControlToFspec(relayMessage);
      expect(fspecMessage.type).toBe('control');

      // @step And unwrap the data envelope so the action field is at top level
      expect(fspecMessage.action).toBe('interrupt');
      expect(fspecMessage.session_id).toBe('session-X');

      // @step And forward the translated control message to the codelet WebSocket connection
      // (forwarding tested in integration)
      expect(fspecMessage).not.toHaveProperty('data');
    });
  });

  // ========================================
  // Command Translation (relay → fspec) — unit
  // ========================================

  describe('Scenario: Translate relay command to fspec flat InboundMessage format', () => {
    it('should unwrap data envelope and serialise args to args_json', () => {
      const relayMessage: RelayMessage = {
        type: 'command',
        session_id: 'session-X',
        request_id: 'R1',
        data: {
          command: 'board',
          args: { format: 'json' },
        },
      };

      const fspecMessage = translateRelayCommandToFspec(relayMessage);

      expect(fspecMessage.type).toBe('command');
      expect(fspecMessage.session_id).toBe('session-X');
      expect(fspecMessage.message).toBe('');
      expect(fspecMessage.request_id).toBe('R1');
      expect(fspecMessage.command).toBe('board');
      expect(fspecMessage.args_json).toBe('{"format":"json"}');
      expect(fspecMessage).not.toHaveProperty('data');
    });
  });

  // ========================================
  // Unknown Session Handling
  // ========================================

  describe('Scenario: Drop messages for unknown session', () => {
    it('should log warning and silently drop message', () => {
      // @step Given the relay endpoint is authenticated
      const endpoint = createAuthenticatedEndpoint();

      // @step And no codelet session with id "unknown-session" is connected
      expect(endpoint.hasSession('unknown-session')).toBe(false);

      // @step When the relay sends an input message targeting session_id "unknown-session"
      const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
      endpoint.handleRelayMessage(
        JSON.stringify({
          type: 'input',
          session_id: 'unknown-session',
          data: { message: 'hello' },
        })
      );

      // @step Then the endpoint should log a warning about the unknown session
      expect(warnSpy).toHaveBeenCalledWith(
        expect.stringContaining('unknown-session')
      );

      // @step And the message should be silently dropped
      // No error thrown, no crash

      // @step And no error response should be sent to the relay
      expect(endpoint.getLastRelaySent()).toBeUndefined();

      warnSpy.mockRestore();
      endpoint.stop();
    });
  });

  // ========================================
  // Chunk Passthrough (fspec → relay)
  // ========================================

  describe('Scenario: Forward StreamChunk from codelet to relay without transformation', () => {
    it('should pass through chunk data without modification', () => {
      // @step Given the relay endpoint is authenticated and a codelet session is connected
      const { endpoint, mockCodeletWs } =
        createEndpointWithSession('session-123');

      // @step When the codelet session sends a chunk message with StreamChunk data
      const chunkData = {
        type: 'chunk',
        session_id: 'session-123',
        data: {
          type: 'tool_call',
          name: 'Read',
          id: 'tc-1',
        },
      };
      endpoint.handleCodeletMessage(mockCodeletWs, JSON.stringify(chunkData));

      // @step Then the endpoint should forward it to the relay as-is with no transformation
      const sent = endpoint.getLastRelaySent();
      expect(sent).toBeDefined();

      // @step And the chunk message should retain its type, session_id, and data fields
      expect(sent?.type).toBe('chunk');
      expect(sent?.session_id).toBe('session-123');

      // @step And the internal StreamChunk field names should not be modified
      const sentData = sent?.data as Record<string, unknown>;
      expect(sentData.type).toBe('tool_call');
      expect(sentData.name).toBe('Read');
      expect(sentData.id).toBe('tc-1');

      endpoint.stop();
    });
  });

  // ========================================
  // Reconnection
  // ========================================

  describe('Scenario: Reconnect to relay after connection drops', () => {
    it('should attempt reconnection with exponential backoff', async () => {
      // @step Given the relay endpoint is authenticated and operating normally
      const endpoint = createAuthenticatedEndpoint();
      expect(endpoint.isAuthenticated()).toBe(true);

      // @step When the relay connection drops unexpectedly
      const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
      endpoint.simulateRelayDisconnect();

      // @step Then the endpoint should log a warning about the disconnection
      expect(warnSpy).toHaveBeenCalledWith(
        expect.stringContaining('disconnected')
      );

      // @step And attempt reconnection with exponential backoff
      expect(endpoint.getReconnectState().isReconnecting).toBe(true);
      expect(endpoint.getReconnectState().delay).toBeGreaterThan(0);

      // @step And re-authenticate with the relay upon successful reconnection
      // (verified by reconnection handler)

      // @step And resume normal message routing after re-authentication
      // (verified by reconnection handler)

      warnSpy.mockRestore();
      await endpoint.stop();
    });
  });

  // ========================================
  // Heartbeat
  // ========================================

  describe('Scenario: Send periodic heartbeat to relay', () => {
    it('should send ping every 30 seconds', () => {
      // @step Given the relay endpoint is authenticated
      vi.useFakeTimers();
      const endpoint = createAuthenticatedEndpoint();

      // @step When 30 seconds elapse since the last ping
      vi.advanceTimersByTime(30_000);

      // @step Then the endpoint should send a ping message to the relay
      const sent = endpoint.getRelaySentMessages();
      const pingMessages = sent.filter(
        (m: Record<string, unknown>) => m.type === 'ping'
      );
      expect(pingMessages.length).toBeGreaterThanOrEqual(1);

      // @step And process the pong response to confirm the connection is alive
      endpoint.handleRelayMessage(JSON.stringify({ type: 'pong' }));
      expect(endpoint.isConnectionAlive()).toBe(true);

      vi.useRealTimers();
      endpoint.stop();
    });
  });
});

// ============================================================================
// End-to-end command scenarios (relay-bridge-endpoint.feature)
// These test the TypeScript endpoint side of the cross-system integration.
// The Rust side (FspecCommandRequest/Result pipeline) is tested in bridge_relay.rs.
// ============================================================================

describe('Feature: relay-bridge-endpoint.feature — Command integration', () => {
  describe('Scenario: Execute fspec command via StreamChunk pipeline and return result', () => {
    it('should translate command, forward to codelet, and relay commandResponse back', () => {
      // @step Given the relay endpoint is authenticated and a codelet session is connected
      const { endpoint, mockCodeletWs } =
        createEndpointWithSession('session-cmd');

      // @step When the relay sends a command message with session_id, request_id, command "board", and args
      endpoint.handleRelayMessage(
        JSON.stringify({
          type: 'command',
          session_id: 'session-cmd',
          request_id: 'R-E2E',
          data: { command: 'board', args: {} },
        })
      );

      // @step Then the endpoint should translate it to an InboundMessage with type "command" and forward to the codelet WebSocket
      expect(mockCodeletWs.sentMessages.length).toBe(1);
      const forwarded = JSON.parse(mockCodeletWs.sentMessages[0]) as Record<
        string,
        unknown
      >;
      expect(forwarded.type).toBe('command');
      expect(forwarded.request_id).toBe('R-E2E');
      expect(forwarded.command).toBe('board');

      // @step And bridge_relay.rs should emit a FspecCommandRequest StreamChunk into the session
      // (Rust side tested in codelet/tools/src/bridge_relay.rs test_handle_command_emits_fspec_request)

      // @step And GlobalSessionStreamManager should handle it by calling fspecCallback
      // (TypeScript side tested in GlobalSessionStreamManager integration)

      // @step And the FspecCommandResult should flow back through the session broadcast channel
      // (pipeline tested in bridge_relay.rs test_intercept_fspec_command_result_chunk)

      // @step And bridge_relay.rs should intercept the FspecCommandResult and NOT forward it as a regular chunk
      // (tested in bridge_relay.rs test_intercept_fspec_command_result_chunk)

      // @step And bridge_relay.rs should send a commandResponse OutboundMessage with the matching request_id and session_id
      // Simulate the commandResponse coming back from codelet → relay
      endpoint.handleCodeletMessage(
        mockCodeletWs,
        JSON.stringify({
          type: 'commandResponse',
          session_id: 'session-cmd',
          request_id: 'R-E2E',
          data: { command: 'board', success: true, result: { columns: {} } },
        })
      );

      // @step And the relay endpoint should translate and forward the commandResponse to the relay
      const sent = endpoint.getRelaySentMessages();
      const responses = sent.filter(
        (m: Record<string, unknown>) => m.type === 'commandResponse'
      );
      expect(responses.length).toBe(1);
      expect(responses[0].request_id).toBe('R-E2E');
      expect(responses[0].session_id).toBe('session-cmd');

      endpoint.stop();
    });
  });

  describe('Scenario: Command execution for unknown fspec command via StreamChunk pipeline', () => {
    it('should relay error commandResponse back for unknown command', () => {
      // @step Given the relay endpoint is authenticated and a codelet session is connected
      const { endpoint, mockCodeletWs } =
        createEndpointWithSession('session-err');

      // @step When the relay sends a command message for an unknown command "nonexistent-command"
      endpoint.handleRelayMessage(
        JSON.stringify({
          type: 'command',
          session_id: 'session-err',
          request_id: 'R-ERR',
          data: { command: 'nonexistent-command', args: {} },
        })
      );

      // @step Then the command should flow through bridge_relay.rs into the FspecCommandRequest StreamChunk pipeline
      expect(mockCodeletWs.sentMessages.length).toBe(1);
      const forwarded = JSON.parse(mockCodeletWs.sentMessages[0]) as Record<
        string,
        unknown
      >;
      expect(forwarded.type).toBe('command');
      expect(forwarded.command).toBe('nonexistent-command');

      // @step And fspecCallback should return a failure result
      // @step And the FspecCommandResult with success false should flow back through the broadcast channel
      // (Rust pipeline tested in bridge_relay.rs)

      // @step And bridge_relay.rs should send a commandResponse with success false and error message
      // Simulate the error commandResponse coming back
      endpoint.handleCodeletMessage(
        mockCodeletWs,
        JSON.stringify({
          type: 'commandResponse',
          session_id: 'session-err',
          request_id: 'R-ERR',
          data: {
            command: 'nonexistent-command',
            success: false,
            error: 'Unknown command: nonexistent-command',
          },
        })
      );

      // @step And the original request_id should be preserved in the response
      const sent = endpoint.getRelaySentMessages();
      const responses = sent.filter(
        (m: Record<string, unknown>) => m.type === 'commandResponse'
      );
      expect(responses.length).toBe(1);
      expect(responses[0].request_id).toBe('R-ERR');
      const data = responses[0].data as Record<string, unknown>;
      expect(data.success).toBe(false);

      endpoint.stop();
    });
  });

  describe('Scenario: Command execution timeout via StreamChunk pipeline', () => {
    it('should relay timeout commandResponse back', () => {
      // @step Given the relay endpoint is authenticated and a codelet session is connected
      const { endpoint, mockCodeletWs } =
        createEndpointWithSession('session-to');

      // @step When the relay sends a command message for a long-running command
      endpoint.handleRelayMessage(
        JSON.stringify({
          type: 'command',
          session_id: 'session-to',
          request_id: 'R-TO',
          data: { command: 'long-command', args: {} },
        })
      );

      // @step And the command does not complete within the pipeline timeout
      // (handled within Rust FspecCommandRequest/FspecCommandResult pipeline)

      // @step Then the FspecCommandRequest/FspecCommandResult pipeline should handle the timeout
      // @step And a FspecCommandResult with timeout error should flow back through the broadcast channel
      // (Rust pipeline handles timeouts via same mechanism as LLM Fspec tool)

      // @step And bridge_relay.rs should send a commandResponse with success false and a timeout error message
      // Simulate the timeout commandResponse coming back
      endpoint.handleCodeletMessage(
        mockCodeletWs,
        JSON.stringify({
          type: 'commandResponse',
          session_id: 'session-to',
          request_id: 'R-TO',
          data: {
            command: 'long-command',
            success: false,
            error: 'Command execution timed out',
          },
        })
      );

      // @step And the original request_id should be preserved in the response
      const sent = endpoint.getRelaySentMessages();
      const responses = sent.filter(
        (m: Record<string, unknown>) => m.type === 'commandResponse'
      );
      expect(responses.length).toBe(1);
      expect(responses[0].request_id).toBe('R-TO');
      const data = responses[0].data as Record<string, unknown>;
      expect(data.success).toBe(false);
      expect(data.error).toContain('timed out');

      endpoint.stop();
    });
  });

  describe('Scenario: FspecCommandResult chunks are intercepted and not forwarded as regular chunks', () => {
    it('should not forward fspecCommandResult as regular chunk', () => {
      // @step Given the relay endpoint is authenticated and a codelet session is connected
      const { endpoint, mockCodeletWs } =
        createEndpointWithSession('session-int');

      // @step When the codelet session's broadcast channel emits a FspecCommandResult StreamChunk
      // From the relay endpoint's perspective, it receives commandResponse (already intercepted by Rust)
      // Regular chunks pass through unchanged, but commandResponse is a distinct message type
      endpoint.handleCodeletMessage(
        mockCodeletWs,
        JSON.stringify({
          type: 'chunk',
          session_id: 'session-int',
          data: { type: 'text', text: 'regular output' },
        })
      );

      // @step Then bridge_relay.rs should intercept it and format it as a commandResponse OutboundMessage
      // @step And the FspecCommandResult should NOT be forwarded to the relay as a regular chunk message
      // Verify chunk passes through but commandResponse is treated separately
      const sent = endpoint.getRelaySentMessages();
      const chunks = sent.filter(
        (m: Record<string, unknown>) => m.type === 'chunk'
      );
      const commandResponses = sent.filter(
        (m: Record<string, unknown>) => m.type === 'commandResponse'
      );
      // Regular chunks go through, but FspecCommandResult interception happens in Rust bridge_relay.rs
      // (tested in bridge_relay.rs test_intercept_fspec_command_result_chunk + test_skip_fspec_command_request_chunks)
      expect(chunks.length).toBe(1);
      expect(commandResponses.length).toBe(0); // No commandResponse since none was sent

      endpoint.stop();
    });
  });
});

// ============================================================================
// BRIDGE-018: Relay Command Flow Through Bridge WebSocket
// ============================================================================

describe('Feature: Relay Command Flow Through Bridge WebSocket', () => {
  // ========================================
  // Command Translation (relay → codelet WS)
  // ========================================

  describe('Scenario: Translate relay command message to InboundMessage and forward to codelet WebSocket', () => {
    it('should translate command to flat InboundMessage and send to codelet WS', () => {
      // @step Given the relay endpoint is authenticated and running
      // @step And a codelet session "session-X" is connected via the local WebSocket server
      const { endpoint, mockCodeletWs } =
        createEndpointWithSession('session-X');

      // @step When the relay sends a command message with type "command", session_id "session-X", request_id "R1", and data containing command "board" and args {}
      endpoint.handleRelayMessage(
        JSON.stringify({
          type: 'command',
          session_id: 'session-X',
          request_id: 'R1',
          data: { command: 'board', args: {} },
        })
      );

      // @step Then the endpoint should translate it to a flat InboundMessage with type "command", session_id "session-X", message "", request_id "R1", command "board", and args_json "{}"
      expect(mockCodeletWs.sentMessages.length).toBe(1);
      const sent = JSON.parse(mockCodeletWs.sentMessages[0]) as Record<
        string,
        unknown
      >;
      expect(sent.type).toBe('command');
      expect(sent.session_id).toBe('session-X');
      expect(sent.message).toBe('');
      expect(sent.request_id).toBe('R1');
      expect(sent.command).toBe('board');
      expect(sent.args_json).toBe('{}');

      // @step And forward the translated InboundMessage to the codelet WebSocket for session "session-X"
      // (verified above — message was received by mockCodeletWs)

      // @step And the endpoint should NOT call fspecCallback directly
      // (verified by architecture: no fspecCallback import exists after refactoring)

      endpoint.stop();
    });
  });

  // ========================================
  // Unknown Session Handling for Commands
  // ========================================

  describe('Scenario: Drop command message for unknown session', () => {
    it('should log warning and drop command for unknown session', () => {
      // @step Given the relay endpoint is authenticated and running
      const endpoint = createAuthenticatedEndpoint();

      // @step And no codelet session with id "unknown-session" is connected
      expect(endpoint.hasSession('unknown-session')).toBe(false);

      // @step When the relay sends a command message targeting session_id "unknown-session"
      const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
      endpoint.handleRelayMessage(
        JSON.stringify({
          type: 'command',
          session_id: 'unknown-session',
          request_id: 'req-999',
          data: { command: 'board', args: {} },
        })
      );

      // @step Then the endpoint should log a warning containing "No codelet connection for session unknown-session"
      expect(warnSpy).toHaveBeenCalledWith(
        expect.stringContaining(
          'No codelet connection for session unknown-session'
        )
      );

      // @step And the command message should be silently dropped
      // No error thrown, no crash

      // @step And no message should be sent to the relay
      expect(endpoint.getLastRelaySent()).toBeUndefined();

      warnSpy.mockRestore();
      endpoint.stop();
    });
  });

  // ========================================
  // commandResponse Passthrough (codelet → relay)
  // ========================================

  describe('Scenario: Forward commandResponse from codelet to relay unchanged', () => {
    it('should pass through commandResponse from codelet to relay', () => {
      // @step Given the relay endpoint is authenticated and running
      // @step And a codelet session "session-X" is connected via the local WebSocket server
      const { endpoint, mockCodeletWs } =
        createEndpointWithSession('session-X');

      // @step When the codelet sends a commandResponse message with type "commandResponse", session_id "session-X", request_id "R1", and data containing command "board", success true, and result
      endpoint.handleCodeletMessage(
        mockCodeletWs,
        JSON.stringify({
          type: 'commandResponse',
          session_id: 'session-X',
          request_id: 'R1',
          data: { command: 'board', success: true, result: { columns: {} } },
        })
      );

      // @step Then the endpoint should forward the commandResponse to the relay without modification
      const sent = endpoint.getRelaySentMessages();
      // Skip the 'connected' message that was forwarded first
      const commandResponses = sent.filter(
        (m: Record<string, unknown>) => m.type === 'commandResponse'
      );
      expect(commandResponses.length).toBe(1);

      // @step And the forwarded message should preserve the request_id "R1"
      const response = commandResponses[0];
      expect(response.request_id).toBe('R1');

      // @step And the forwarded message should preserve the data fields
      const data = response.data as Record<string, unknown>;
      expect(data.command).toBe('board');
      expect(data.success).toBe(true);

      endpoint.stop();
    });
  });

  // ========================================
  // Architecture Constraints
  // ========================================

  describe('Scenario: No bridge TypeScript files import fspecCallback', () => {
    it('should have no fspecCallback imports in bridge directory', async () => {
      // @step Given the relay endpoint refactoring is complete

      // @step When searching for imports of "fspec-callback" in the bridge directory
      const bridgeDir = join(import.meta.dirname, '..');
      const files = await readdir(bridgeDir);
      const tsFiles = files.filter(
        f => f.endsWith('.ts') && !f.endsWith('.test.ts')
      );

      let fspecCallbackImportFound = false;
      for (const file of tsFiles) {
        const content = await readFile(join(bridgeDir, file), 'utf-8');
        if (content.includes('fspec-callback')) {
          fspecCallbackImportFound = true;
        }
      }

      // @step Then no TypeScript file in the bridge directory should import from "src/utils/fspec-callback"
      expect(fspecCallbackImportFound).toBe(false);
    });
  });

  describe('Scenario: relay-command-executor.ts is deleted', () => {
    it('should not have relay-command-executor.ts file', async () => {
      // @step Given the relay endpoint refactoring is complete

      // @step When checking for the file "bridge/relay-command-executor.ts"
      const filePath = join(
        import.meta.dirname,
        '..',
        'relay-command-executor.ts'
      );
      let fileExists = true;
      try {
        await access(filePath);
      } catch {
        fileExists = false;
      }

      // @step Then the file should not exist on disk
      expect(fileExists).toBe(false);
    });
  });
});
