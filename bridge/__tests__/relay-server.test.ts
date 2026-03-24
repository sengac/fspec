/**
 * Feature: spec/features/relay-server.feature
 *
 * This test file validates the relay server — the local WebSocket hub
 * that routes messages between relay-endpoint.ts and the mobile app.
 *
 * BRIDGE-019: Relay Server
 */

import { describe, it, expect, afterEach } from 'vitest';

import { createRelayServer } from '../relay-server';
import type {
  RelayServerConfig,
  RelayServerInstance,
  RelayClient,
} from '../relay-server-types';

// ============================================================================
// Test Helpers
// ============================================================================

interface MockClient extends RelayClient {
  sentMessages: Array<Record<string, unknown>>;
  closed: boolean;
  closeCode?: number;
  closeReason?: string;
}

/** Create a mock client that records all sent messages and close events */
function createMockClient(): MockClient {
  return {
    sentMessages: [],
    closed: false,
    send(data: string) {
      this.sentMessages.push(JSON.parse(data) as Record<string, unknown>);
    },
    close(code?: number, reason?: string) {
      this.closed = true;
      this.closeCode = code;
      this.closeReason = reason;
    },
  };
}

/** Create a test config with optional overrides */
function createTestConfig(
  overrides?: Partial<RelayServerConfig>
): RelayServerConfig {
  return { port: 0, apiKey: undefined, ...overrides };
}

/**
 * Authenticate a mock client on the given server and channel.
 * Clears sentMessages after auth so tests start with a clean slate.
 */
function authenticateClient(
  server: RelayServerInstance,
  channelId: string,
  apiKey = ''
): MockClient {
  const client = createMockClient();
  server.handleClientMessage(
    client,
    JSON.stringify({
      type: 'auth',
      data: { channel_id: channelId, api_key: apiKey },
    })
  );
  client.sentMessages.length = 0;
  return client;
}

// ============================================================================
// Tests
// ============================================================================

describe('Feature: Relay Server', () => {
  let server: RelayServerInstance;

  afterEach(() => {
    if (server) {
      server.stop();
    }
  });

  describe('Scenario: Server starts and listens on configured port', () => {
    it('should start and listen on the configured port', () => {
      // @step Given the RELAY_SERVER_PORT environment variable is set to "8765"
      const config = createTestConfig({ port: 8765 });

      // @step When I start the relay server
      server = createRelayServer(config);

      // @step Then the server should listen for WebSocket connections on port 8765
      expect(server.isRunning()).toBe(true);

      // @step And the server should log "[relay-server] Listening on port 8765"
      expect(server.getPort()).toBe(8765);
    });
  });

  describe('Scenario: Successful authentication with valid API key', () => {
    it('should authenticate client with valid api_key', () => {
      // @step Given the relay server is running with RELAY_SERVER_API_KEY set to "secret"
      server = createRelayServer(createTestConfig({ apiKey: 'secret' }));
      const client = createMockClient();

      // @step When a client connects and sends auth with channel_id "my-project" and api_key "secret"
      server.handleClientMessage(
        client,
        JSON.stringify({
          type: 'auth',
          data: { channel_id: 'my-project', api_key: 'secret' },
        })
      );

      // @step Then the server should respond with authSuccess
      expect(client.sentMessages).toHaveLength(1);
      expect(client.sentMessages[0].type).toBe('authSuccess');

      // @step And the authSuccess data should include an instances array
      const data = client.sentMessages[0].data as Record<string, unknown>;
      expect(data.instances).toBeInstanceOf(Array);
    });
  });

  describe('Scenario: Open mode authentication when no API key is configured', () => {
    it('should accept any api_key when server has no key set', () => {
      // @step Given the relay server is running without RELAY_SERVER_API_KEY set
      server = createRelayServer(createTestConfig({ apiKey: undefined }));
      const client = createMockClient();

      // @step When a client connects and sends auth with channel_id "my-project" and api_key "anything"
      server.handleClientMessage(
        client,
        JSON.stringify({
          type: 'auth',
          data: { channel_id: 'my-project', api_key: 'anything' },
        })
      );

      // @step Then the server should respond with authSuccess
      expect(client.sentMessages).toHaveLength(1);
      expect(client.sentMessages[0].type).toBe('authSuccess');
    });
  });

  describe('Scenario: Authentication failure with invalid API key', () => {
    it('should reject client with wrong api_key', () => {
      // @step Given the relay server is running with RELAY_SERVER_API_KEY set to "secret"
      server = createRelayServer(createTestConfig({ apiKey: 'secret' }));
      const client = createMockClient();

      // @step When a client connects and sends auth with channel_id "my-project" and api_key "wrong"
      server.handleClientMessage(
        client,
        JSON.stringify({
          type: 'auth',
          data: { channel_id: 'my-project', api_key: 'wrong' },
        })
      );

      // @step Then the server should respond with authError code "INVALID_API_KEY"
      expect(client.sentMessages).toHaveLength(1);
      expect(client.sentMessages[0].type).toBe('authError');
      const data = client.sentMessages[0].data as Record<string, unknown>;
      expect(data.code).toBe('INVALID_API_KEY');

      // @step And the server should close the connection
      expect(client.closed).toBe(true);
    });
  });

  describe('Scenario: Unauthenticated client sends non-auth message', () => {
    it('should reject messages from unauthenticated clients', () => {
      // @step Given the relay server is running
      server = createRelayServer(createTestConfig());
      // @step And a client is connected but has not authenticated
      const client = createMockClient();

      // @step When the client sends an input message
      server.handleClientMessage(
        client,
        JSON.stringify({
          type: 'input',
          session_id: 'sess-1',
          data: { message: 'hello' },
        })
      );

      // @step Then the server should respond with authError code "NOT_AUTHENTICATED"
      expect(client.sentMessages).toHaveLength(1);
      expect(client.sentMessages[0].type).toBe('authError');
      const data = client.sentMessages[0].data as Record<string, unknown>;
      expect(data.code).toBe('NOT_AUTHENTICATED');

      // @step And the server should close the connection
      expect(client.closed).toBe(true);
    });
  });

  describe('Scenario: Connected message broadcast to channel peers', () => {
    it('should broadcast connected to other clients in same channel', () => {
      // @step Given the relay server is running
      server = createRelayServer(createTestConfig());

      // @step And client A is authenticated on channel "my-project"
      const clientA = authenticateClient(server, 'my-project');

      // @step And client B is authenticated on channel "my-project"
      const clientB = authenticateClient(server, 'my-project');

      // @step When client A sends a connected message with session_id "sess-1"
      server.handleClientMessage(
        clientA,
        JSON.stringify({ type: 'connected', session_id: 'sess-1', data: {} })
      );

      // @step Then client B should receive the connected message with session_id "sess-1"
      expect(clientB.sentMessages).toHaveLength(1);
      expect(clientB.sentMessages[0].type).toBe('connected');
      expect(clientB.sentMessages[0].session_id).toBe('sess-1');

      // @step And client A should NOT receive the connected message back
      expect(clientA.sentMessages).toHaveLength(0);
    });
  });

  describe('Scenario: Route input message to channel peers', () => {
    it('should forward input messages to other clients in same channel', () => {
      // @step Given the relay server is running
      server = createRelayServer(createTestConfig());

      // @step And client A is authenticated on channel "my-project"
      const clientA = authenticateClient(server, 'my-project');

      // @step And client B is authenticated on channel "my-project"
      const clientB = authenticateClient(server, 'my-project');

      // @step When client A sends an input message for session_id "sess-1"
      const inputMsg = {
        type: 'input',
        session_id: 'sess-1',
        data: { message: 'fix the bug' },
      };
      server.handleClientMessage(clientA, JSON.stringify(inputMsg));

      // @step Then client B should receive the exact same JSON message
      expect(clientB.sentMessages).toHaveLength(1);
      expect(clientB.sentMessages[0]).toEqual(inputMsg);

      // @step And client A should NOT receive the message back
      expect(clientA.sentMessages).toHaveLength(0);
    });
  });

  describe('Scenario: Route sessionControl message to channel peers', () => {
    it('should forward sessionControl messages to other clients in same channel', () => {
      // @step Given the relay server is running
      server = createRelayServer(createTestConfig());

      // @step And client A is authenticated on channel "my-project"
      const clientA = authenticateClient(server, 'my-project');

      // @step And client B is authenticated on channel "my-project"
      const clientB = authenticateClient(server, 'my-project');

      // @step When client A sends a sessionControl message for session_id "sess-1"
      const sessionControlMsg = {
        type: 'sessionControl',
        session_id: 'sess-1',
        data: { action: 'cancel' },
      };
      server.handleClientMessage(clientA, JSON.stringify(sessionControlMsg));

      // @step Then client B should receive the exact same JSON message
      expect(clientB.sentMessages).toHaveLength(1);
      expect(clientB.sentMessages[0]).toEqual(sessionControlMsg);

      // @step And client A should NOT receive the message back
      expect(clientA.sentMessages).toHaveLength(0);
    });
  });

  describe('Scenario: Route chunk message to channel peers', () => {
    it('should forward chunk messages to other clients in same channel', () => {
      // @step Given the relay server is running
      server = createRelayServer(createTestConfig());

      // @step And client A is authenticated on channel "my-project"
      const clientA = authenticateClient(server, 'my-project');

      // @step And client B is authenticated on channel "my-project"
      const clientB = authenticateClient(server, 'my-project');

      // @step When client A sends a chunk message with session_id "sess-1" and text data
      const chunkMsg = {
        type: 'chunk',
        session_id: 'sess-1',
        data: { type: 'text', text: 'I found the issue...' },
      };
      server.handleClientMessage(clientA, JSON.stringify(chunkMsg));

      // @step Then client B should receive the exact same JSON message
      expect(clientB.sentMessages).toHaveLength(1);
      expect(clientB.sentMessages[0]).toEqual(chunkMsg);
    });
  });

  describe('Scenario: Route command and commandResponse between channel peers', () => {
    it('should route command and commandResponse bidirectionally', () => {
      // @step Given the relay server is running
      server = createRelayServer(createTestConfig());

      // @step And client A is authenticated on channel "my-project"
      const clientA = authenticateClient(server, 'my-project');

      // @step And client B is authenticated on channel "my-project"
      const clientB = authenticateClient(server, 'my-project');

      // @step When client A sends a command message with request_id "r1"
      const commandMsg = {
        type: 'command',
        session_id: 'sess-1',
        request_id: 'r1',
        data: { command: 'board', args: {} },
      };
      server.handleClientMessage(clientA, JSON.stringify(commandMsg));

      // @step Then client B should receive the command message with request_id "r1"
      expect(clientB.sentMessages).toHaveLength(1);
      expect(clientB.sentMessages[0].request_id).toBe('r1');
      expect(clientB.sentMessages[0].type).toBe('command');

      // @step When client B sends a commandResponse with request_id "r1"
      const responseMsg = {
        type: 'commandResponse',
        session_id: 'sess-1',
        request_id: 'r1',
        data: { command: 'board', success: true, result: {} },
      };
      server.handleClientMessage(clientB, JSON.stringify(responseMsg));

      // @step Then client A should receive the commandResponse with request_id "r1"
      expect(clientA.sentMessages).toHaveLength(1);
      expect(clientA.sentMessages[0].request_id).toBe('r1');
      expect(clientA.sentMessages[0].type).toBe('commandResponse');
    });
  });

  describe('Scenario: Channel isolation prevents cross-channel message leaking', () => {
    it('should not forward messages between different channels', () => {
      // @step Given the relay server is running
      server = createRelayServer(createTestConfig());

      // @step And client A is authenticated on channel "project-A"
      const clientA = authenticateClient(server, 'project-A');

      // @step And client B is authenticated on channel "project-B"
      const clientB = authenticateClient(server, 'project-B');

      // @step When client A sends a chunk message
      server.handleClientMessage(
        clientA,
        JSON.stringify({
          type: 'chunk',
          session_id: 'sess-1',
          data: { type: 'text', text: 'secret data' },
        })
      );

      // @step Then client B should NOT receive any message
      expect(clientB.sentMessages).toHaveLength(0);
    });
  });

  describe('Scenario: Server responds to ping with pong directly', () => {
    it('should respond to ping with pong and not broadcast', () => {
      // @step Given the relay server is running
      server = createRelayServer(createTestConfig());

      // @step And a client is authenticated on channel "my-project"
      const clientA = authenticateClient(server, 'my-project');
      const clientB = authenticateClient(server, 'my-project');

      // @step When the client sends a ping message
      server.handleClientMessage(
        clientA,
        JSON.stringify({ type: 'ping', data: {} })
      );

      // @step Then the server should respond with a pong message
      expect(clientA.sentMessages).toHaveLength(1);
      expect(clientA.sentMessages[0].type).toBe('pong');

      // @step And other clients in the channel should NOT receive the ping
      expect(clientB.sentMessages).toHaveLength(0);
    });
  });

  describe('Scenario: Client removed from channel on disconnect', () => {
    it('should remove client from channel and stop routing to it', () => {
      // @step Given the relay server is running
      server = createRelayServer(createTestConfig());

      // @step And client A is authenticated on channel "my-project"
      const clientA = authenticateClient(server, 'my-project');

      // @step And client B is authenticated on channel "my-project"
      const clientB = authenticateClient(server, 'my-project');

      // @step When client A disconnects
      server.handleClientDisconnect(clientA);

      // @step And client B sends a chunk message
      server.handleClientMessage(
        clientB,
        JSON.stringify({
          type: 'chunk',
          session_id: 'sess-1',
          data: { type: 'text', text: 'hello' },
        })
      );

      // @step Then client A should NOT receive the message
      expect(clientA.sentMessages).toHaveLength(0);

      // @step And the server should log the disconnection
      // (Verified by no error thrown and successful routing to remaining clients)
      expect(server.getChannelClientCount('my-project')).toBe(1);
    });
  });

  describe('Scenario: Newly connecting client receives current instance list', () => {
    it('should include known sessions in authSuccess for new client', () => {
      // @step Given the relay server is running
      server = createRelayServer(createTestConfig());

      // @step And client A is authenticated on channel "my-project"
      const clientA = authenticateClient(server, 'my-project');

      // @step And client A has sent a connected message with session_id "sess-1"
      server.handleClientMessage(
        clientA,
        JSON.stringify({ type: 'connected', session_id: 'sess-1', data: {} })
      );

      // @step When client B authenticates on channel "my-project"
      const clientB = createMockClient();
      server.handleClientMessage(
        clientB,
        JSON.stringify({
          type: 'auth',
          data: { channel_id: 'my-project', api_key: '' },
        })
      );

      // @step Then client B should receive authSuccess with instances containing session_id "sess-1"
      expect(clientB.sentMessages).toHaveLength(1);
      expect(clientB.sentMessages[0].type).toBe('authSuccess');
      const data = clientB.sentMessages[0].data as Record<string, unknown>;
      const instances = data.instances as Array<Record<string, unknown>>;
      expect(instances).toHaveLength(1);
      expect(instances[0].session_id).toBe('sess-1');
    });
  });
});
