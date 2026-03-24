/**
 * Relay Server — Local WebSocket hub
 *
 * A standalone WebSocket server that routes messages between
 * relay-endpoint.ts (fspec-side) and the mobile app (fspec-mobile).
 *
 * BRIDGE-019: Relay Server
 *
 * Architecture:
 * - Pure message router — does NOT inspect or transform payloads
 * - Channel-based routing via channel_id from auth handshake
 * - Handles auth, ping/pong directly; forwards everything else
 * - Tracks 'connected' messages to provide instance list on auth
 */

import { WebSocketServer, WebSocket } from 'ws';
import type {
  RelayServerConfig,
  RelayClient,
  ClientState,
  RelayServerInstance,
} from './relay-server-types';

export type { RelayServerConfig, RelayClient, RelayServerInstance };

// ============================================================================
// Constants
// ============================================================================

const LOG_PREFIX = '[relay-server]';

// ============================================================================
// Factory
// ============================================================================

/**
 * Create a relay server instance.
 *
 * Uses the same factory-with-closure pattern as relay-endpoint.ts
 * for testability — all state is private, public API is explicit.
 */
export function createRelayServer(
  serverConfig: RelayServerConfig
): RelayServerInstance {
  /** Channel → set of clients in that channel */
  const channels = new Map<string, Set<RelayClient>>();
  /** Client → its auth/channel state */
  const clientStates = new Map<RelayClient, ClientState>();
  /** Channel → list of known session_ids (from 'connected' messages) */
  const channelSessions = new Map<string, Array<{ session_id: string }>>();

  let running = true;

  // ------------------------------------------------------------------
  // Internal helpers
  // ------------------------------------------------------------------

  /**
   * Add a client to the specified channel's routing group.
   * Creates the channel group if it doesn't exist yet.
   */
  function addToChannel(channelId: string, client: RelayClient): void {
    let channelClients = channels.get(channelId);
    if (!channelClients) {
      channelClients = new Set();
      channels.set(channelId, channelClients);
    }
    channelClients.add(client);
  }

  /**
   * Remove a client from its channel group and clean up state.
   * Deletes the channel group if it becomes empty.
   */
  function removeFromChannel(client: RelayClient): void {
    const state = clientStates.get(client);
    if (!state) {
      return;
    }
    const channelClients = channels.get(state.channelId);
    if (channelClients) {
      channelClients.delete(client);
      if (channelClients.size === 0) {
        channels.delete(state.channelId);
      }
    }
    clientStates.delete(client);
  }

  /**
   * Forward a raw JSON message to all clients in the channel except the sender.
   * No-op if the channel doesn't exist or has no other clients.
   */
  function broadcastToChannel(
    channelId: string,
    sender: RelayClient,
    rawJson: string
  ): void {
    const channelClients = channels.get(channelId);
    if (!channelClients) {
      return;
    }
    for (const peer of channelClients) {
      if (peer !== sender) {
        peer.send(rawJson);
      }
    }
  }

  /**
   * Track a session_id from a 'connected' message so it can be included
   * in the instance list for future auth responses on this channel.
   */
  function trackConnectedSession(
    channelId: string,
    msg: Record<string, unknown>
  ): void {
    const sessionId = msg.session_id as string | undefined;
    if (!sessionId) {
      return;
    }
    let sessions = channelSessions.get(channelId);
    if (!sessions) {
      sessions = [];
      channelSessions.set(channelId, sessions);
    }
    const exists = sessions.some(s => s.session_id === sessionId);
    if (!exists) {
      sessions.push({ session_id: sessionId });
    }
  }

  // ------------------------------------------------------------------
  // Auth handler
  // ------------------------------------------------------------------

  /**
   * Handle an auth handshake message from a client.
   * Validates the API key (if configured), adds the client to its channel,
   * and responds with authSuccess or authError.
   */
  function handleAuth(client: RelayClient, msg: Record<string, unknown>): void {
    const data = (msg.data || {}) as Record<string, unknown>;
    const channelId = data.channel_id as string;
    const apiKey = data.api_key as string | undefined;

    if (!channelId) {
      client.send(
        JSON.stringify({
          type: 'authError',
          data: { code: 'INVALID_CHANNEL', message: 'channel_id is required' },
        })
      );
      client.close(4001, 'Invalid channel');
      return;
    }

    // Validate API key if server has one configured
    if (
      serverConfig.apiKey !== undefined &&
      serverConfig.apiKey !== '' &&
      apiKey !== serverConfig.apiKey
    ) {
      console.log(
        `${LOG_PREFIX} Auth failed for channel ${channelId}: invalid API key`
      );
      client.send(
        JSON.stringify({
          type: 'authError',
          data: { code: 'INVALID_API_KEY', message: 'Invalid API key' },
        })
      );
      client.close(4003, 'Invalid API key');
      return;
    }

    // Auth success
    clientStates.set(client, { channelId, authenticated: true });
    addToChannel(channelId, client);
    const instances = channelSessions.get(channelId) || [];
    client.send(JSON.stringify({ type: 'authSuccess', data: { instances } }));
    console.log(
      `${LOG_PREFIX} Client authenticated on channel ${channelId} (${channels.get(channelId)?.size || 0} clients)`
    );
  }

  // ------------------------------------------------------------------
  // Public API
  // ------------------------------------------------------------------

  const instance: RelayServerInstance = {
    handleClientMessage(client: RelayClient, data: string): void {
      let msg: Record<string, unknown>;
      try {
        msg = JSON.parse(data) as Record<string, unknown>;
      } catch {
        console.error(`${LOG_PREFIX} Failed to parse message`);
        return;
      }

      const msgType = msg.type as string;

      // Auth is always allowed
      if (msgType === 'auth') {
        handleAuth(client, msg);
        return;
      }

      // All other messages require authentication
      const state = clientStates.get(client);
      if (!state || !state.authenticated) {
        client.send(
          JSON.stringify({
            type: 'authError',
            data: {
              code: 'NOT_AUTHENTICATED',
              message: 'Must authenticate first',
            },
          })
        );
        client.close(4001, 'Not authenticated');
        return;
      }

      // Ping — respond directly, do not forward
      if (msgType === 'ping') {
        client.send(JSON.stringify({ type: 'pong', data: {} }));
        return;
      }

      // Track 'connected' messages for instance list
      if (msgType === 'connected') {
        trackConnectedSession(state.channelId, msg);
      }

      // Everything else: forward to all other clients in the channel
      broadcastToChannel(state.channelId, client, data);
    },

    handleClientDisconnect(client: RelayClient): void {
      const state = clientStates.get(client);
      if (state) {
        console.log(
          `${LOG_PREFIX} Client disconnected from channel ${state.channelId}`
        );
      }
      removeFromChannel(client);
    },

    isRunning(): boolean {
      return running;
    },

    getPort(): number {
      return serverConfig.port;
    },

    getChannelClientCount(channelId: string): number {
      return channels.get(channelId)?.size || 0;
    },

    stop(): void {
      running = false;
      channels.clear();
      clientStates.clear();
      channelSessions.clear();
    },
  };

  return instance;
}

// ============================================================================
// WebSocket Server Wrapper (for real usage — not called during tests)
// ============================================================================

/**
 * Start the relay server with a real WebSocketServer.
 * Returns a cleanup function.
 */
export function startRelayServer(serverConfig: RelayServerConfig): {
  server: RelayServerInstance;
  wss: WebSocketServer;
} {
  const server = createRelayServer(serverConfig);
  const wss = new WebSocketServer({ port: serverConfig.port });

  wss.on('connection', (ws: WebSocket) => {
    console.log(`${LOG_PREFIX} Client connected`);

    ws.on('message', (rawData: Buffer | string) => {
      server.handleClientMessage(ws, rawData.toString());
    });

    ws.on('close', () => {
      server.handleClientDisconnect(ws);
    });

    ws.on('error', (error: Error) => {
      console.error(`${LOG_PREFIX} WebSocket error:`, error.message);
    });
  });

  console.log(`${LOG_PREFIX} Listening on port ${serverConfig.port}`);

  return { server, wss };
}
