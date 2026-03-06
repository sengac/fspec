/**
 * fspec Browser Agent - Shared Type Definitions
 *
 * Types shared across all extension components:
 * - Service worker
 * - Content scripts
 * - Popup
 * - Native messaging host (EXT-003)
 */

/** Message types for communication between extension components */
export interface ExtensionMessage {
  type: string;
  payload?: unknown;
}

/** WebMCP tool metadata discovered from a web page */
export interface WebMCPToolInfo {
  name: string;
  description: string;
  inputSchema?: Record<string, unknown>;
  origin: string;
  tabId: number;
}

/** Native messaging host connection status */
export interface ConnectionStatus {
  connected: boolean;
  port: number;
  clientCount: number;
}

/** Tool registry entry */
export interface ToolRegistryEntry {
  name: string;
  description: string;
  inputSchema?: Record<string, unknown>;
  source: 'native' | 'webmcp';
  origin?: string;
  tabId?: number;
}

/** Tool summary sent from service worker to popup for grouped display */
export interface PopupToolSummary {
  name: string;
  source: 'native' | 'webmcp';
  origin?: string;
  tabId?: number;
}

/** Status response from service worker to popup */
export interface StatusResponse {
  connected: boolean;
  nativeConnected: boolean;
  toolCount: number;
  port: number;
  clientCount: number;
  tools: PopupToolSummary[];
}

/**
 * Message type constants for inter-component communication.
 *
 * Content → Service Worker:
 *   FSPEC_WEBMCP_TOOL_REGISTERED   - WebMCP tool discovered by main world script
 *   FSPEC_WEBMCP_TOOL_UNREGISTERED - WebMCP tool removed by main world script
 *   FSPEC_INVOKE_RESULT            - Result of tool invocation from main world
 *
 * Service Worker → Content → Main World:
 *   FSPEC_INVOKE_TOOL              - Request to invoke a WebMCP tool
 *
 * Popup → Service Worker:
 *   FSPEC_GET_STATUS               - Request connection/tool status
 *
 * Native Host → Service Worker:
 *   TOOL_CALL                      - Incoming tool call from MCP client
 *
 * Service Worker → Native Host:
 *   TOOLS_CHANGED                  - Tool registry has been updated
 *   NOTIFICATION                   - Browser event to forward as MCP notification
 */
export const MESSAGE_TYPES = {
  // Content → SW
  TOOL_REGISTERED: 'FSPEC_WEBMCP_TOOL_REGISTERED',
  TOOL_UNREGISTERED: 'FSPEC_WEBMCP_TOOL_UNREGISTERED',
  INVOKE_RESULT: 'FSPEC_INVOKE_RESULT',
  // SW → Content → Main
  INVOKE_TOOL: 'FSPEC_INVOKE_TOOL',
  // Popup → SW
  GET_STATUS: 'FSPEC_GET_STATUS',
  // Native Host ↔ SW
  TOOL_CALL: 'TOOL_CALL',
  TOOLS_CHANGED: 'TOOLS_CHANGED',
  NOTIFICATION: 'NOTIFICATION',
} as const;
