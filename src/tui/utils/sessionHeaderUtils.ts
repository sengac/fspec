/**
 * Session Header Utilities
 *
 * Shared utilities for session header display.
 * Used by both AgentView and SplitSessionView.
 *
 * SOLID: Single Responsibility - only header formatting logic
 * DRY: Shared between all session views
 */

/**
 * Format context window size for display
 * @example 200000 → "200k", 1000000 → "1M"
 */
export const formatContextWindow = (contextWindow: number): string => {
  if (contextWindow >= 1000000) {
    return `${(contextWindow / 1000000).toFixed(0)}M`;
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
}

/**
 * Get the maximum token values from two trackers
 * Used to show correct values when attaching to existing sessions
 */
export const getMaxTokens = (
  tracker1: TokenTracker,
  tracker2: TokenTracker
): { inputTokens: number; outputTokens: number } => {
  return {
    inputTokens: Math.max(tracker1.inputTokens, tracker2.inputTokens),
    outputTokens: Math.max(tracker1.outputTokens, tracker2.outputTokens),
  };
};
