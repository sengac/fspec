/**
 * Relay Bridge Endpoint
 *
 * Standalone WebSocket endpoint that connects fspec to a platform-agnostic relay server.
 * Unlike the Telegram endpoint which translates to Telegram Bot API, this endpoint speaks
 * the relay protocol directly — handling 5 message types plus auth handshake.
 *
 * BRIDGE-015: Platform-Agnostic Relay Bridge Endpoint
 *
 * Architecture:
 * - WebSocket CLIENT connects TO the relay server
 * - Local WebSocket SERVER accepts codelet BridgeTool connections
 * - Supports multiple concurrent sessions via session_id→WS map
 * - Message handlers are in relay-endpoint-handlers.ts
 */

import { WebSocketServer, WebSocket } from 'ws';
import { config } from 'dotenv';

import type {
  RelayEndpointConfig,
  RelayEndpointState,
  ReconnectState,
  SessionCreateCallback,
  ConfigValidationResult,
} from './relay-types';

import {
  handleRelayMessage,
  handleCodeletMessage,
  translateRelayInputToFspec,
  translateRelaySessionControlToFspec,
  translateRelayCommandToFspec,
} from './relay-endpoint-handlers';
import type { HandlerContext, CodeletSocket } from './relay-endpoint-handlers';

// Re-export for test imports
export {
  translateRelayInputToFspec,
  translateRelaySessionControlToFspec,
  translateRelayCommandToFspec,
};
export type {
  RelayEndpointConfig,
  RelayEndpointState,
  RelayMessage,
  FspecInboundMessage,
} from './relay-types';

// ---------------------------------------------------------------------------
// Config Validation
// ---------------------------------------------------------------------------

export function validateConfig(
  configInput: Partial<RelayEndpointConfig>
): ConfigValidationResult {
  const errors: string[] = [];

  if (!configInput.relayUrl) {
    errors.push('RELAY_URL is required');
  }
  if (!configInput.channelId) {
    errors.push('RELAY_CHANNEL_ID is required');
  }
  if (!configInput.apiKey) {
    errors.push('RELAY_API_KEY is required');
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}

// Load environment variables
config();

// ============================================================================
// Constants
// ============================================================================

const PING_INTERVAL_MS = 30_000;
const INITIAL_RECONNECT_DELAY_MS = 1000;
const MAX_RECONNECT_DELAY_MS = 30_000;
const LOG_PREFIX = '[relay-endpoint]';

// ============================================================================
// Relay Endpoint Factory
// ============================================================================

export function createRelayEndpoint(
  endpointConfig: RelayEndpointConfig,
  onSessionCreate?: SessionCreateCallback
): RelayEndpointState & {
  buildAuthMessage: () => Record<string, unknown>;
  handleRelayMessage: (data: string) => void;
  handleCodeletMessage: (ws: CodeletSocket, data: string) => void;
  hasSession: (sessionId: string) => boolean;
  routeToSession: (sessionId: string, msg: Record<string, unknown>) => void;
  getLastRelaySent: () => Record<string, unknown> | undefined;
  getRelaySentMessages: () => Array<Record<string, unknown>>;
  simulateRelayDisconnect: () => void;
  getReconnectState: () => ReconnectState;
  isAuthenticated: () => boolean;
  isLocalServerRunning: () => boolean;
  isConnectionAlive: () => boolean;
  start: () => Promise<void>;
  stop: () => Promise<void>;
} {
  const state: RelayEndpointState = {
    relayWs: null,
    localWss: null,
    config: endpointConfig,
    authenticated: false,
    authError: undefined,
    sessions: new Map(),
    relaySentMessages: [],
    reconnectState: {
      isReconnecting: false,
      delay: INITIAL_RECONNECT_DELAY_MS,
      attempts: 0,
    },
    pingInterval: null,
    connectionAlive: true,
    isRunning: false,
  };

  // ------------------------------------------------------------------
  // Internal helpers
  // ------------------------------------------------------------------

  function sendToRelay(msg: Record<string, unknown>): void {
    state.relaySentMessages.push(msg);
    if (state.relayWs && state.relayWs.readyState === WebSocket.OPEN) {
      state.relayWs.send(JSON.stringify(msg));
    }
  }

  function startPingInterval(): void {
    if (state.pingInterval) {
      clearInterval(state.pingInterval);
    }
    state.pingInterval = setInterval(() => {
      sendToRelay({ type: 'ping' });
    }, PING_INTERVAL_MS);
  }

  function stopPingInterval(): void {
    if (state.pingInterval) {
      clearInterval(state.pingInterval);
      state.pingInterval = null;
    }
  }

  function startLocalServer(): void {
    if (state.localWss) {
      return;
    }
    const port = endpointConfig.websocketPort || 8181;
    state.localWss = new WebSocketServer({ port });
    state.isRunning = true;

    state.localWss.on('connection', (ws: WebSocket) => {
      console.log(`${LOG_PREFIX} Codelet session connected`);

      ws.on('message', (rawData: Buffer | string) => {
        handleCodeletMessage(ws, rawData.toString(), ctx);
      });

      ws.on('close', () => {
        for (const [sessionId, sessionWs] of state.sessions.entries()) {
          if (sessionWs === ws) {
            console.log(`${LOG_PREFIX} Session ${sessionId} disconnected`);
            state.sessions.delete(sessionId);
            break;
          }
        }
      });

      ws.on('error', (error: Error) => {
        console.error(`${LOG_PREFIX} Codelet WebSocket error:`, error.message);
      });
    });
  }

  // ------------------------------------------------------------------
  // Handler context — passed to extracted message handlers
  // ------------------------------------------------------------------

  const ctx: HandlerContext = {
    state,
    sendToRelay,
    startLocalServer,
    startPingInterval,
    onSessionCreate,
  };

  // ------------------------------------------------------------------
  // Relay connection + reconnection
  // ------------------------------------------------------------------

  async function connectToRelay(): Promise<void> {
    return new Promise<void>((resolve, reject) => {
      try {
        const ws = new WebSocket(endpointConfig.relayUrl);

        ws.on('open', () => {
          state.relayWs = ws;
          state.reconnectState = {
            isReconnecting: false,
            delay: INITIAL_RECONNECT_DELAY_MS,
            attempts: 0,
          };
          console.log(
            `${LOG_PREFIX} Connected to relay: ${endpointConfig.relayUrl}`
          );
          ws.send(JSON.stringify(endpoint.buildAuthMessage()));
          resolve();
        });

        ws.on('message', (rawData: Buffer | string) => {
          handleRelayMessage(rawData.toString(), ctx);
        });

        ws.on('close', () => {
          console.warn(`${LOG_PREFIX} Relay connection disconnected`);
          state.relayWs = null;
          state.authenticated = false;
          stopPingInterval();
          if (state.isRunning) {
            scheduleReconnect();
          }
        });

        ws.on('error', (error: Error) => {
          console.error(`${LOG_PREFIX} Relay connection error:`, error.message);
          reject(error);
        });
      } catch (error) {
        reject(error);
      }
    });
  }

  function scheduleReconnect(): void {
    state.reconnectState.isReconnecting = true;
    state.reconnectState.attempts += 1;

    const delay = Math.min(
      state.reconnectState.delay *
        Math.pow(2, state.reconnectState.attempts - 1),
      MAX_RECONNECT_DELAY_MS
    );
    state.reconnectState.delay = delay;

    console.log(
      `${LOG_PREFIX} Reconnecting in ${delay}ms (attempt ${state.reconnectState.attempts})`
    );

    setTimeout(() => {
      if (state.isRunning) {
        void connectToRelay().catch((error: Error) => {
          console.error(`${LOG_PREFIX} Reconnection failed:`, error.message);
          if (state.isRunning) {
            scheduleReconnect();
          }
        });
      }
    }, delay);
  }

  // ------------------------------------------------------------------
  // Public API
  // ------------------------------------------------------------------

  const endpoint = {
    buildAuthMessage(): Record<string, unknown> {
      return {
        type: 'auth',
        data: {
          channel_id: endpointConfig.channelId,
          api_key: endpointConfig.apiKey,
        },
      };
    },

    handleRelayMessage: (data: string) => handleRelayMessage(data, ctx),
    handleCodeletMessage: (ws: CodeletSocket, data: string) =>
      handleCodeletMessage(ws, data, ctx),

    hasSession(sessionId: string): boolean {
      return state.sessions.has(sessionId);
    },

    routeToSession(sessionId: string, msg: Record<string, unknown>): void {
      const ws = state.sessions.get(sessionId);
      if (ws) {
        ws.send(JSON.stringify(msg));
      }
    },

    getLastRelaySent(): Record<string, unknown> | undefined {
      return state.relaySentMessages[state.relaySentMessages.length - 1];
    },

    getRelaySentMessages(): Array<Record<string, unknown>> {
      return state.relaySentMessages;
    },

    simulateRelayDisconnect(): void {
      console.warn(`${LOG_PREFIX} Relay connection disconnected`);
      state.relayWs = null;
      state.authenticated = false;
      stopPingInterval();
      state.reconnectState.isReconnecting = true;
      state.reconnectState.delay = INITIAL_RECONNECT_DELAY_MS;
    },

    getReconnectState(): ReconnectState {
      return { ...state.reconnectState };
    },

    isAuthenticated: (): boolean => state.authenticated,
    isLocalServerRunning: (): boolean => state.localWss !== null,
    isConnectionAlive: (): boolean => state.connectionAlive,

    async start(): Promise<void> {
      const validation = validateConfig(endpointConfig);
      if (!validation.valid) {
        console.error(
          `${LOG_PREFIX} Configuration errors:`,
          validation.errors.join(', ')
        );
        process.exit(1);
      }
      state.isRunning = true;
      await connectToRelay();
    },

    async stop(): Promise<void> {
      state.isRunning = false;
      stopPingInterval();
      if (state.relayWs) {
        state.relayWs.close();
        state.relayWs = null;
      }
      if (state.localWss) {
        state.localWss.close();
        state.localWss = null;
      }
      state.sessions.clear();
      state.authenticated = false;
      state.relaySentMessages = [];
    },

    get authError() {
      return state.authError;
    },
  };

  return endpoint;
}

// ============================================================================
// CLI Entry Point
// ============================================================================

const runAsMain = process.argv[1]?.includes('relay-endpoint');

if (runAsMain) {
  config();

  const endpointConfig: RelayEndpointConfig = {
    relayUrl: process.env.RELAY_URL || '',
    channelId: process.env.RELAY_CHANNEL_ID || '',
    apiKey: process.env.RELAY_API_KEY || '',
    websocketPort: parseInt(process.env.WEBSOCKET_PORT || '8181', 10),
  };

  const validation = validateConfig(endpointConfig);
  if (!validation.valid) {
    console.error(`${LOG_PREFIX} Configuration errors:`);
    for (const error of validation.errors) {
      console.error(`  - ${error}`);
    }
    process.exit(1);
  }

  const endpoint = createRelayEndpoint(endpointConfig);

  void endpoint
    .start()
    .then(() => {
      console.log(`${LOG_PREFIX} Endpoint started successfully`);
    })
    .catch((error: Error) => {
      console.error(`${LOG_PREFIX} Failed to start:`, error.message);
      process.exit(1);
    });

  process.on('SIGINT', async () => {
    console.log(`\n${LOG_PREFIX} Shutting down...`);
    await endpoint.stop();
    process.exit(0);
  });

  process.on('SIGTERM', async () => {
    console.log(`${LOG_PREFIX} Received SIGTERM, shutting down...`);
    await endpoint.stop();
    process.exit(0);
  });
}
