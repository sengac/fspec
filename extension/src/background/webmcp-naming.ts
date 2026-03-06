/**
 * fspec Browser Agent - Tool Naming Utilities
 *
 * Shared utilities for constructing and parsing WebMCP-namespaced tool names.
 * Format: webmcp__<sanitized-origin>__<toolName>
 *
 * Origins (hostnames) are sanitized to comply with the Anthropic API tool name
 * pattern: ^[a-zA-Z0-9_-]{1,128}$. Dots and other disallowed characters in
 * hostnames are replaced with hyphens.
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
 * Sanitize an origin (hostname) for use in tool names.
 *
 * Replaces any character not in [a-zA-Z0-9_-] with a hyphen.
 * This ensures the resulting tool name complies with the Anthropic API
 * pattern: ^[a-zA-Z0-9_-]{1,128}$
 *
 * Examples:
 *   "app.example.com" → "app-example-com"
 *   "localhost:3000"     → "localhost-3000"
 *   "example.com"        → "example-com"
 */
export function sanitizeOrigin(origin: string): string {
  return origin.replace(/[^a-zA-Z0-9_-]/g, '-');
}

/**
 * Build a namespaced WebMCP tool name from origin and tool name.
 *
 * Always produces the 3-segment format: webmcp__<sanitized-origin>__<toolName>
 * Origin is required — the WebMCP discovery script always provides it.
 * The origin is sanitized to replace dots and other special characters with hyphens.
 */
export function buildWebmcpToolName(origin: string, toolName: string): string {
  const sanitized = sanitizeOrigin(origin);
  return `${WEBMCP_PREFIX}${SEPARATOR}${sanitized}${SEPARATOR}${toolName}`;
}

/**
 * Parse a namespaced WebMCP tool name into its (sanitized) origin and original tool name.
 *
 * Note: the returned origin will be the sanitized form (hyphens instead of dots).
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
