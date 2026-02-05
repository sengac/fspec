/**
 * anchorUtils.ts - Shared utilities for anchor point display
 *
 * Contains:
 * - Type label constants (no emojis)
 * - Time formatting utilities
 * - Turn details conversion utilities
 */

import type { AnchorTurnDetails } from '../types/anchor';

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

/**
 * Convert AnchorTurnDetails to array of lines for display
 *
 * Used by VirtualList to render turn content in the preview pane.
 *
 * @param details - Turn details to convert, or null if not available
 * @returns Array of strings, one per display line
 */
export function turnDetailsToLines(
  details: AnchorTurnDetails | null
): string[] {
  if (!details) {
    return ['No turn details available'];
  }

  const lines: string[] = [];

  // Turn header
  lines.push(`TURN ${details.turnIndex}`);
  lines.push('─'.repeat(40));
  lines.push('');

  // User message
  if (details.userMessage) {
    lines.push('User:');
    const userLines = details.userMessage.split('\n');
    userLines.forEach(line => lines.push(`  ${line}`));
    lines.push('');
  }

  // Assistant response
  if (details.assistantResponse) {
    lines.push('Assistant:');
    const assistantLines = details.assistantResponse.split('\n');
    assistantLines.forEach(line => lines.push(`  ${line}`));
    lines.push('');
  }

  // Tool calls
  if (details.toolCalls && details.toolCalls.length > 0) {
    lines.push('Tools:');
    details.toolCalls.forEach(tc => {
      const status = tc.success ? '+' : '-';
      lines.push(`  [${status}] ${tc.tool}`);
    });
    lines.push('');
  }

  // Status
  if (details.status) {
    lines.push(`Status: ${details.status}`);
  }

  return lines;
}
