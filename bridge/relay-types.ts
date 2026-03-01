/**
 * Relay Bridge Endpoint — Type Definitions
 *
 * BRIDGE-015: Platform-Agnostic Relay Bridge Endpoint
 */

import type { WebSocket } from 'ws';
import type { WebSocketServer } from 'ws';

// ============================================================================
// Configuration
// ============================================================================

export interface RelayEndpointConfig {
  relayUrl: string;
  channelId: string;
  apiKey: string;
  websocketPort: number;
}

export interface ConfigValidationResult {
  valid: boolean;
  errors: string[];
}

// ============================================================================
// Relay Protocol Messages (camelCase, {data:{...}} envelope)
// ============================================================================

export interface RelayMessage {
  type: string;
  session_id?: string;
  request_id?: string;
  instance_id?: string;
  data: Record<string, unknown>;
}

// ============================================================================
// fspec InboundMessage (flat fields, 'control' instead of 'sessionControl')
// ============================================================================

export interface FspecInboundMessage {
  type: string;
  session_id: string;
  message?: string;
  images?: Array<{ data: string; media_type: string }>;
  action?: string;
  response?: string;
  // BRIDGE-018: Command fields for bridge_relay.rs InboundMessage
  request_id?: string;
  command?: string;
  args_json?: string;
}

// ============================================================================
// Endpoint State
// ============================================================================

export interface ReconnectState {
  isReconnecting: boolean;
  delay: number;
  attempts: number;
}

/** Minimal WebSocket-like interface for session routing */
export interface SessionSocket {
  send: (data: string) => void;
}

export interface RelayEndpointState {
  relayWs: WebSocket | null;
  localWss: WebSocketServer | null;
  config: RelayEndpointConfig;
  authenticated: boolean;
  authError?: { code: string; message: string };
  sessions: Map<string, SessionSocket>;
  relaySentMessages: Array<Record<string, unknown>>;
  reconnectState: ReconnectState;
  pingInterval: ReturnType<typeof setInterval> | null;
  connectionAlive: boolean;
  isRunning: boolean;
}
