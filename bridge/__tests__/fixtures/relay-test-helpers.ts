/**
 * Shared test fixtures for Relay Bridge Endpoint tests.
 *
 * These helpers provide properly-typed mock objects and factory functions
 * for testing the relay endpoint, message translation, and command flow.
 *
 * DRY: Extracted from relay-endpoint.test.ts to avoid duplicated setup patterns.
 */

import type { RelayEndpointConfig } from '../../relay-types';
import { createRelayEndpoint } from '../../relay-endpoint';

// ============================================================================
// Mock WebSocket
// ============================================================================

/** Mock WebSocket for testing codelet connections */
export interface MockCodeletWebSocket {
  sentMessages: string[];
  send: (data: string) => void;
  readyState: number;
  close: () => void;
}

/**
 * Create a properly typed mock WebSocket for testing codelet connections.
 * Collects sent messages in an inspectable array.
 *
 * Unlike the Telegram fixture's vi.fn()-based mock, this uses an explicit
 * array to allow ordered inspection of messages sent to the codelet.
 */
export function createMockCodeletWebSocket(): MockCodeletWebSocket {
  return {
    sentMessages: [],
    send(data: string) {
      this.sentMessages.push(data);
    },
    readyState: 1, // OPEN
    close() {
      this.readyState = 3; // CLOSED
    },
  };
}

// ============================================================================
// Config Factory
// ============================================================================

/**
 * Create a default valid RelayEndpointConfig for testing.
 * Uses port 0 to avoid binding to real ports in unit tests.
 *
 * @param overrides - Optional fields to override defaults
 */
export function createTestConfig(
  overrides?: Partial<RelayEndpointConfig>
): RelayEndpointConfig {
  return {
    relayUrl: 'ws://relay.example.com/v1/ws',
    channelId: 'test-channel',
    apiKey: 'valid-api-key',
    websocketPort: 0,
    ...overrides,
  };
}

// ============================================================================
// Authenticated Endpoint Factory
// ============================================================================

/**
 * Create a relay endpoint that is already authenticated.
 *
 * This is the most common setup pattern in tests:
 * 1. Create endpoint with valid config
 * 2. Simulate authSuccess from relay
 *
 * @param configOverrides - Optional config field overrides
 * @returns Authenticated endpoint ready for testing
 */
export function createAuthenticatedEndpoint(
  configOverrides?: Partial<RelayEndpointConfig>
) {
  const endpointConfig = createTestConfig(configOverrides);
  const endpoint = createRelayEndpoint(endpointConfig);
  endpoint.handleRelayMessage(
    JSON.stringify({ type: 'authSuccess', data: { instances: [] } })
  );
  return endpoint;
}

/**
 * Create an authenticated endpoint with a connected codelet session.
 *
 * Builds on createAuthenticatedEndpoint and adds a session registration.
 *
 * @param sessionId - The session ID to register
 * @param configOverrides - Optional config field overrides
 * @returns Object with endpoint and the mock codelet WebSocket
 */
export function createEndpointWithSession(
  sessionId: string,
  configOverrides?: Partial<RelayEndpointConfig>
) {
  const endpoint = createAuthenticatedEndpoint(configOverrides);
  const mockCodeletWs = createMockCodeletWebSocket();
  endpoint.handleCodeletMessage(
    mockCodeletWs,
    JSON.stringify({
      type: 'connected',
      session_id: sessionId,
      data: {},
    })
  );
  return { endpoint, mockCodeletWs };
}
