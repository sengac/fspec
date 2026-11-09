/**
 * Feature: spec/features/relay-command-flow-through-bridge.feature
 *
 * This test file validates the acceptance criteria defined in the feature file
 * for the BRIDGE-018 command flow refactoring (moved out of relay-endpoint.test.ts
 * so each test file maps one feature, COV-075).
 */

import { describe, it, expect, vi } from 'vitest';
import { readdir, readFile, access } from 'fs/promises';
import { join } from 'path';

import {
  createAuthenticatedEndpoint,
  createEndpointWithSession,
} from './fixtures/relay-test-helpers';

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
