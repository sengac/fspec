/**
 * anchorUtils.ts - Shared utilities for anchor point display
 *
 * Contains:
 * - Type label constants (no emojis)
 * - Time formatting utilities
 */

/**
 * Text labels for anchor types (text labels only, no emojis)
 */
export const ANCHOR_TYPE_LABELS: Record<string, string> = {
  ErrorResolution: '[Error]',
  TaskCompletion: '[Task]',
  UserCheckpoint: '[Check]',
  FeatureMilestone: '[Mile]',
};

/**
 * Format timestamp as relative time (e.g., "2 min ago")
 *
 * @param timestamp - Unix timestamp in milliseconds
 * @returns Human-readable relative time string
 */
export function formatRelativeTime(timestamp: number): string {
  const now = Date.now();
  const diffMs = now - timestamp;
  const diffSec = Math.floor(diffMs / 1000);
  const diffMin = Math.floor(diffSec / 60);
  const diffHr = Math.floor(diffMin / 60);

  if (diffHr > 0) {
    return `${diffHr} hr ago`;
  }
  if (diffMin > 0) {
    return `${diffMin} min ago`;
  }
  return `${diffSec} sec ago`;
}
