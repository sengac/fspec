/**
 * Turn selection utilities for VirtualList-based conversation views
 *
 * SOLID: Single responsibility - utilities for turn selection logic
 * DRY: Shared between AgentView and SplitSessionView
 *
 * Used by:
 * - AgentView.tsx (main conversation view)
 * - SplitSessionView.tsx (watcher split view)
 */

import type { ConversationLine } from '../types/conversation';

/**
 * Determine if a separator line is at the top or bottom of the selected turn.
 * Used to render selection indicator bars (▼ above, ▲ below selected turn).
 *
 * @param line - The current line being rendered
 * @param lineIndex - Index of the line in the array
 * @param allLines - All lines in the conversation
 * @param selectedIndex - Currently selected line index
 * @param isSelectMode - Whether selection mode is active
 * @returns 'top' if separator is above selected turn, 'bottom' if below, null otherwise
 */
export function getSelectionSeparatorType(
  line: ConversationLine,
  lineIndex: number,
  allLines: ConversationLine[],
  selectedIndex: number,
  isSelectMode: boolean
): 'top' | 'bottom' | null {
  if (!line.isSeparator || !isSelectMode) return null;

  const selectedMessageIndex = allLines[selectedIndex]?.messageIndex;
  if (selectedMessageIndex === undefined) return null;

  // Bottom bar: separator belongs to the selected turn (appears after turn content)
  if (line.messageIndex === selectedMessageIndex) {
    return 'bottom';
  }

  // Top bar: next non-separator line belongs to selected turn
  for (let i = lineIndex + 1; i < allLines.length; i++) {
    const nextLine = allLines[i];
    if (!nextLine.isSeparator) {
      if (nextLine.messageIndex === selectedMessageIndex) {
        return 'top';
      }
      break;
    }
  }

  return null;
}

/**
 * Generate arrow bar string for selection indicator.
 * Creates pattern like "▼   ▼   ▼   ▼" or "▲   ▲   ▲   ▲"
 *
 * @param width - Total width of the bar in characters
 * @param direction - 'top' for ▼ arrows, 'bottom' for ▲ arrows
 * @param spacing - Characters between arrows (default: 4)
 * @returns Arrow bar string
 */
export function generateArrowBar(
  width: number,
  direction: 'top' | 'bottom',
  spacing: number = 4
): string {
  const arrow = direction === 'top' ? '▼' : '▲';
  let result = '';
  for (let i = 0; i < width; i++) {
    result += i % spacing === 0 ? arrow : ' ';
  }
  return result;
}

/**
 * Get the first content line of a turn by messageIndex.
 * Used for generating previews in "Discuss Selected" feature.
 *
 * @param lines - All conversation lines
 * @param messageIndex - The message index to find
 * @returns First non-separator content of the turn, or empty string
 */
export function getFirstContentOfTurn(
  lines: ConversationLine[],
  messageIndex: number
): string {
  for (const line of lines) {
    if (
      line.messageIndex === messageIndex &&
      !line.isSeparator &&
      line.content.trim()
    ) {
      return line.content;
    }
  }
  return '';
}

/**
 * Generate pre-fill content for "Discuss Selected" feature.
 * When user selects a turn in parent pane and presses Enter,
 * this generates context text to pre-fill the input.
 *
 * @param turnNumber - 1-indexed turn number for display
 * @param turnContent - Content of the selected turn
 * @param maxPreviewLength - Maximum characters for preview (default: 50)
 * @returns Formatted pre-fill string with turn context
 */
export function generateDiscussSelectedPrefill(
  turnNumber: number,
  turnContent: string,
  maxPreviewLength: number = 50
): string {
  const preview =
    turnContent.slice(0, maxPreviewLength) +
    (turnContent.length > maxPreviewLength ? '...' : '');
  return `Regarding turn ${turnNumber} in parent session:\n\`\`\`\n${preview}\n\`\`\`\n`;
}

/**
 * Count content lines (excluding separators) in a conversation.
 * Used for displaying line counts in pane headers.
 *
 * @param lines - Conversation lines to count
 * @returns Number of non-separator lines
 */
export function getContentLineCount(lines: ConversationLine[]): number {
  return lines.filter(l => !l.isSeparator).length;
}
