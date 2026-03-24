/**
 * Relay Server Types
 *
 * Shared types for the relay server, its CLI entry point, and tests.
 *
 * BRIDGE-019: Relay Server
 */

/** Configuration for creating a relay server instance */
export interface RelayServerConfig {
  port: number;
  apiKey: string | undefined;
}

/** Minimal client interface for testability (real WebSocket or mock) */
export interface RelayClient {
  send: (data: string) => void;
  close: (code?: number, reason?: string) => void;
}

/** Internal state tracking for an authenticated client */
export interface ClientState {
  channelId: string;
  authenticated: boolean;
}

/** Public API surface of a relay server instance */
export interface RelayServerInstance {
  handleClientMessage: (client: RelayClient, data: string) => void;
  handleClientDisconnect: (client: RelayClient) => void;
  isRunning: () => boolean;
  getPort: () => number;
  getChannelClientCount: (channelId: string) => number;
  stop: () => void;
}
