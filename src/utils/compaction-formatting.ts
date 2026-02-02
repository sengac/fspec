/**
 * Utility functions for formatting compaction-related text
 * Centralizes formatting logic to maintain consistency across components
 */

import type { CompactionProgress } from '../tui/hooks/useRustSessionState';

/**
 * Formats compaction progress for display in input placeholders
 * Used by MultiLineInput component
 *
 * @param progress - Compaction progress information
 * @returns Formatted text like "Compacting: analyzing anchors... 15/32 turns"
 */
export function formatCompactionPlaceholder(
  progress: CompactionProgress
): string {
  return `Compacting: ${progress.phase}... ${progress.current}/${progress.total} turns`;
}

/**
 * Formats compaction progress for display in thinking indicators
 * Used by InputTransition component
 *
 * @param progress - Compaction progress information
 * @returns Formatted text like "analyzing anchors... 15/32 turns"
 */
export function formatCompactionThinking(progress: CompactionProgress): string {
  return `${progress.phase}... ${progress.current}/${progress.total} turns`;
}

/**
 * Generic compaction progress formatter
 * Can be used when you need custom prefixes or formatting
 *
 * @param progress - Compaction progress information
 * @param options - Formatting options
 * @returns Formatted compaction text
 */
export function formatCompactionProgress(
  progress: CompactionProgress,
  options: {
    prefix?: string;
    suffix?: string;
    showTurns?: boolean;
  } = {}
): string {
  const { prefix = '', suffix = '', showTurns = true } = options;

  const baseText = `${progress.phase}... ${progress.current}/${progress.total}`;
  const turnsText = showTurns ? ' turns' : '';

  return `${prefix}${baseText}${turnsText}${suffix}`.trim();
}

/**
 * Validates that compaction progress data is well-formed
 * Used in tests and runtime validation
 *
 * @param progress - Compaction progress to validate
 * @returns true if valid, false otherwise
 */
export function isValidCompactionProgress(
  progress: CompactionProgress | null | undefined
): progress is CompactionProgress {
  if (!progress) return false;

  return (
    typeof progress.phase === 'string' &&
    progress.phase.length > 0 &&
    typeof progress.current === 'number' &&
    typeof progress.total === 'number' &&
    Number.isInteger(progress.current) &&
    Number.isInteger(progress.total) &&
    progress.current >= 0 &&
    progress.total > 0 &&
    progress.current <= progress.total
  );
}

/**
 * Calculates compaction progress percentage
 *
 * @param progress - Compaction progress information
 * @returns Percentage (0-100) or 0 if invalid
 */
export function calculateCompactionPercentage(
  progress: CompactionProgress | null
): number {
  if (!isValidCompactionProgress(progress)) return 0;

  if (progress.total === 0) return 0;
  return Math.round((progress.current / progress.total) * 100);
}
