/**
 * fspec Browser Agent - MCP Server (Stub)
 *
 * Placeholder for the Streamable HTTP MCP server.
 * The actual server runs in the Native Messaging Host (Node.js process),
 * not in the extension itself.
 *
 * This module will contain types and utilities shared between
 * the extension and the native host.
 *
 * Real functionality added by:
 * - EXT-003: Full MCP server implementation in native messaging host
 */

export const MCP_DEFAULT_PORT = 19876;
export const MCP_ENDPOINT = '/mcp';
export const NATIVE_MESSAGING_HOST_NAME = 'com.fspec.browser.agent';
