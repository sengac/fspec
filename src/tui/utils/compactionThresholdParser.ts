/**
 * Compaction threshold input parser
 *
 * CTX-008: Parses user input for compaction threshold configuration.
 *
 * Formats:
 * - Plain number (e.g., "200000") → { type: 'tokens', value: 200000 }
 * - Number with % suffix (e.g., "80%") → { type: 'percentage', value: 80 }
 * - Empty or invalid → undefined (use built-in default)
 */

import type { CompactionThresholdConfig } from '../../utils/provider-config';

/** Minimum token count for absolute threshold */
const MIN_TOKEN_THRESHOLD = 1000;

/** Minimum valid percentage */
const MIN_PERCENTAGE = 1;

/** Maximum valid percentage */
const MAX_PERCENTAGE = 100;

/**
 * Parse a user-entered compaction threshold string.
 *
 * @param input - Raw string from the TUI input field
 * @returns Parsed config or undefined if invalid/empty
 */
export function parseCompactionThreshold(
  input: string
): CompactionThresholdConfig | undefined {
  const trimmed = input.trim();
  if (!trimmed) {
    return undefined;
  }

  if (trimmed.endsWith('%')) {
    const pct = parseInt(trimmed.slice(0, -1), 10);
    if (isNaN(pct) || pct < MIN_PERCENTAGE || pct > MAX_PERCENTAGE) {
      return undefined;
    }
    return { type: 'percentage', value: pct };
  }

  const tokens = parseInt(trimmed, 10);
  if (isNaN(tokens) || tokens < MIN_TOKEN_THRESHOLD) {
    return undefined;
  }
  return { type: 'tokens', value: tokens };
}

/**
 * Format a CompactionThresholdConfig for display in the TUI.
 *
 * @param config - The threshold config to format
 * @returns Human-readable string (e.g., "80%" or "200000")
 */
export function formatCompactionThreshold(
  config: CompactionThresholdConfig | undefined
): string {
  if (!config) {
    return '';
  }
  if (config.type === 'percentage') {
    return `${config.value}%`;
  }
  return String(config.value);
}
