/**
 * Session Header Utilities
 *
 * Shared utilities for session header display.
 * Used by AgentView.
 *
 * SOLID: Single Responsibility - only header formatting logic
 * DRY: Shared between all session views
 */

/**
 * Format context window size for display
 * @param contextWindow - Token count
 * @param precision - 'compact' (default): no decimals for M values, 'precise': 1 decimal for M values
 * @example formatContextWindow(200000) → "200k"
 * @example formatContextWindow(1000000) → "1M"
 * @example formatContextWindow(1500000, 'precise') → "1.5M"
 */
export const formatContextWindow = (
  contextWindow: number,
  precision: 'compact' | 'precise' = 'compact'
): string => {
  if (contextWindow >= 1000000) {
    const decimals = precision === 'precise' ? 1 : 0;
    return `${(contextWindow / 1000000).toFixed(decimals)}M`;
  }
  return `${Math.round(contextWindow / 1000)}k`;
};

/**
 * Get color based on context fill percentage
 * - Green: < 50% (plenty of room)
 * - Yellow: 50-70% (getting full)
 * - Magenta: 70-85% (nearly full)
 * - Red: > 85% (critical)
 */
export const getContextFillColor = (percentage: number): string => {
  if (percentage < 50) return 'green';
  if (percentage < 70) return 'yellow';
  if (percentage < 85) return 'magenta';
  return 'red';
};

/**
 * Token usage tracker interface
 */
export interface TokenTracker {
  inputTokens: number;
  outputTokens: number;
  cacheReadInputTokens?: number;
  cacheCreationInputTokens?: number;
  reasoningTokens?: number;
}

/**
 * Get the maximum token values from two trackers
 * Used to show correct values when attaching to existing sessions
 */
export const getMaxTokens = (
  tracker1: TokenTracker,
  tracker2: TokenTracker
): { inputTokens: number; outputTokens: number; reasoningTokens: number } => {
  return {
    inputTokens: Math.max(tracker1.inputTokens, tracker2.inputTokens),
    outputTokens: Math.max(tracker1.outputTokens, tracker2.outputTokens),
    reasoningTokens: Math.max(
      tracker1.reasoningTokens ?? 0,
      tracker2.reasoningTokens ?? 0
    ),
  };
};
