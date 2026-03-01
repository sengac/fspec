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
 * - Command messages are translated to flat InboundMessage format and forwarded
 *   through the codelet bridge WebSocket to bridge_relay.rs (BRIDGE-018)
 * - StreamChunk data passes through without internal field transformation
 */

import { WebSocketServer, WebSocket } from 'ws';
import { config } from 'dotenv';

import type {
  RelayEndpointConfig,
  RelayEndpointState,
  RelayMessage,
  ReconnectState,
} from './relay-types';

import {
  validateConfig,
  translateRelayInputToFspec,
  translateRelaySessionControlToFspec,
  translateRelayCommandToFspec,
} from './relay-message-translation';

// Re-export for test imports
export {
  validateConfig,
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

/** Minimal WebSocket interface for codelet connections (allows test mocks) */
interface CodeletSocket {
  send: (data: string) => void;
}

export function createRelayEndpoint(
  endpointConfig: RelayEndpointConfig
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

  /**
   * Look up the codelet WebSocket for a session. Logs a warning and returns
   * null when the session is unknown — callers should silently drop the message.
   */
  function getSessionOrWarn(
    sessionId: string | undefined,
    context: string
  ): CodeletSocket | null {
    const id = sessionId || '';
    const ws = state.sessions.get(id);
    if (!ws) {
      console.warn(
        `${LOG_PREFIX} No codelet connection for session ${id}${context ? ` (${context})` : ''}`
      );
    }
    return ws ?? null;
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
    const port = endpointConfig.websocketPort || 8080;
    state.localWss = new WebSocketServer({ port });
    state.isRunning = true;

    state.localWss.on('connection', (ws: WebSocket) => {
      console.log(`${LOG_PREFIX} Codelet session connected`);

      ws.on('message', (rawData: Buffer | string) => {
        endpoint.handleCodeletMessage(ws, rawData.toString());
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
  // Relay message handler
  // ------------------------------------------------------------------

  function handleRelayMessageInternal(data: string): void {
    let msg: RelayMessage;
    try {
      msg = JSON.parse(data) as RelayMessage;
    } catch {
      console.error(`${LOG_PREFIX} Failed to parse relay message`);
      return;
    }

    switch (msg.type) {
      case 'authSuccess':
        state.authenticated = true;
        state.authError = undefined;
        console.log(`${LOG_PREFIX} Authenticated with relay`);
        startLocalServer();
        startPingInterval();
        break;

      case 'authError': {
        state.authenticated = false;
        const errorData = msg.data || {};
        state.authError = {
          code: (errorData.code as string) || 'UNKNOWN',
          message: (errorData.message as string) || 'Authentication failed',
        };
        console.error(
          `${LOG_PREFIX} Authentication failed`,
          state.authError.code
        );
        break;
      }

      case 'pong':
        state.connectionAlive = true;
        break;

      case 'input': {
        const sessionWs = getSessionOrWarn(msg.session_id, '');
        if (!sessionWs) {
          return;
        }
        sessionWs.send(JSON.stringify(translateRelayInputToFspec(msg)));
        break;
      }

      case 'sessionControl': {
        const sessionWs = getSessionOrWarn(msg.session_id, '');
        if (!sessionWs) {
          return;
        }
        sessionWs.send(
          JSON.stringify(translateRelaySessionControlToFspec(msg))
        );
        break;
      }

      case 'command': {
        const sessionWs = getSessionOrWarn(msg.session_id, 'command');
        if (!sessionWs) {
          return;
        }
        sessionWs.send(JSON.stringify(translateRelayCommandToFspec(msg)));
        break;
      }

      default:
        break;
    }
  }

  // ------------------------------------------------------------------
  // Codelet message handler
  // ------------------------------------------------------------------

  function handleCodeletMessageInternal(ws: CodeletSocket, data: string): void {
    let msg: Record<string, unknown>;
    try {
      msg = JSON.parse(data) as Record<string, unknown>;
    } catch {
      console.error(`${LOG_PREFIX} Failed to parse codelet message`);
      return;
    }

    const msgType = msg.type as string;

    if (msgType === 'connected') {
      const sessionId = msg.session_id as string;
      state.sessions.set(sessionId, ws);
      console.log(`${LOG_PREFIX} Session registered: ${sessionId}`);
      sendToRelay(msg);
      return;
    }

    // chunk + any other messages pass through to relay
    sendToRelay(msg);
  }

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
          handleRelayMessageInternal(rawData.toString());
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

    handleRelayMessage: handleRelayMessageInternal,
    handleCodeletMessage: handleCodeletMessageInternal,

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

    isAuthenticated(): boolean {
      return state.authenticated;
    },

    isLocalServerRunning(): boolean {
      return state.localWss !== null;
    },

    isConnectionAlive(): boolean {
      return state.connectionAlive;
    },

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
    websocketPort: parseInt(process.env.WEBSOCKET_PORT || '8080', 10),
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
