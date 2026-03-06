/**
 * fspec WebMCP Extension - Tool Naming Utilities
 *
 * Shared utilities for constructing and parsing WebMCP-namespaced tool names.
 * Format: webmcp__<origin>__<toolName>
 *
 * Used by message-router.ts for registration, unregistration, and invocation.
 *
 * Implemented by: EXT-006
 */

/** Prefix for all WebMCP tool names */
const WEBMCP_PREFIX = 'webmcp';

/** Separator between segments */
const SEPARATOR = '__';

/**
 * Build a namespaced WebMCP tool name from origin and tool name.
 *
 * Always produces the 3-segment format: webmcp__<origin>__<toolName>
 * Origin is required — the WebMCP discovery script always provides it.
 */
export function buildWebmcpToolName(origin: string, toolName: string): string {
  return `${WEBMCP_PREFIX}${SEPARATOR}${origin}${SEPARATOR}${toolName}`;
}

/**
 * Parse a namespaced WebMCP tool name into its origin and original tool name.
 *
 * Returns undefined if the name doesn't match the expected format.
 */
export function parseWebmcpToolName(
  namespacedName: string
): { origin: string; toolName: string } | undefined {
  const firstSep = namespacedName.indexOf(SEPARATOR);
  if (firstSep === -1) {
    return undefined;
  }

  const prefix = namespacedName.slice(0, firstSep);
  if (prefix !== WEBMCP_PREFIX) {
    return undefined;
  }

  const rest = namespacedName.slice(firstSep + SEPARATOR.length);
  const secondSep = rest.indexOf(SEPARATOR);
  if (secondSep === -1) {
    return undefined;
  }

  const origin = rest.slice(0, secondSep);
  const toolName = rest.slice(secondSep + SEPARATOR.length);

  if (!origin || !toolName) {
    return undefined;
  }

  return { origin, toolName };
}
