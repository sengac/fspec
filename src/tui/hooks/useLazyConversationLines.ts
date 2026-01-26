/**
 * useLazyConversationLines - React hook for viewport-aware lazy line computation
 *
 * PERF-004: Integrates LazyLineIndex with React lifecycle for efficient
 * conversation rendering in VirtualList.
 *
 * Usage:
 * ```tsx
 * const { lineCount, getLines, getMessageIndex } = useLazyConversationLines(
 *   conversation,
 *   maxWidth
 * );
 *
 * // In VirtualList:
 * <VirtualList
 *   itemCount={lineCount}
 *   getItems={getLines}
 *   groupBy={(_, index) => getMessageIndex(index)}
 *   ...
 * />
 * ```
 */

import { useRef, useCallback, useMemo } from 'react';
import type {
  ConversationMessage,
  ConversationLine,
} from '../types/conversation';
import { LazyLineIndex, createLazyLineIndex } from './lazyLineIndex';

/**
 * Result of useLazyConversationLines hook
 */
export interface LazyConversationLinesResult {
  /**
   * Total number of lines (for VirtualList itemCount)
   */
  lineCount: number;

  /**
   * Get lines in a specific range (for VirtualList getItems)
   * @param start - Start index (inclusive)
   * @param end - End index (exclusive)
   * @returns Array of ConversationLine for the range
   */
  getLines: (start: number, end: number) => ConversationLine[];

  /**
   * Get the messageIndex for a specific line index (for VirtualList groupBy)
   * @param lineIndex - Line index
   * @returns Message index, or -1 if out of range
   */
  getMessageIndex: (lineIndex: number) => number;

  /**
   * Get all lines (fallback when full array is needed)
   * Use sparingly - defeats lazy loading optimization.
   */
  getAllLines: () => ConversationLine[];

  /**
   * Whether lazy mode is active (true when conversation is large enough)
   */
  isLazyMode: boolean;
}

/**
 * Threshold for enabling lazy mode.
 * Below this message count, eager computation is fine.
 */
const LAZY_MODE_THRESHOLD = 50; // Messages

/**
 * React hook for viewport-aware lazy line computation.
 *
 * Automatically manages LazyLineIndex lifecycle and provides optimized
 * access to conversation lines for VirtualList.
 *
 * @param messages - Conversation messages
 * @param maxWidth - Maximum line width for wrapping
 * @param isWatcherView - Whether this is a watcher view (affects width calculation)
 * @returns LazyConversationLinesResult with lineCount and accessor functions
 */
export function useLazyConversationLines(
  messages: ConversationMessage[],
  maxWidth: number,
  isWatcherView: boolean = false
): LazyConversationLinesResult {
  // Track previous values for incremental updates
  const indexRef = useRef<LazyLineIndex | null>(null);
  const prevMessagesRef = useRef<ConversationMessage[]>([]);
  const prevMaxWidthRef = useRef<number>(maxWidth);
  const prevWatcherViewRef = useRef<boolean>(isWatcherView);

  // Determine if lazy mode should be active
  const isLazyMode = messages.length >= LAZY_MODE_THRESHOLD;

  // Update or create the index
  // Use useMemo to recompute only when dependencies change
  const lineCount = useMemo(() => {
    const prevMessages = prevMessagesRef.current;
    const prevMaxWidth = prevMaxWidthRef.current;
    const prevWatcherView = prevWatcherViewRef.current;

    // Store new values
    prevMessagesRef.current = messages;
    prevMaxWidthRef.current = maxWidth;
    prevWatcherViewRef.current = isWatcherView;

    if (!isLazyMode) {
      // Small conversation - don't use lazy index
      indexRef.current?.dispose();
      indexRef.current = null;
      return 0; // lineCount doesn't matter in non-lazy mode
    }

    const index = indexRef.current;

    // Check if we need to rebuild
    const widthChanged = maxWidth !== prevMaxWidth;
    const watcherViewChanged = isWatcherView !== prevWatcherView;
    const messagesChanged = messages !== prevMessages;

    if (!index || widthChanged || watcherViewChanged) {
      // Full rebuild needed
      if (index) {
        index.dispose();
      }
      indexRef.current = createLazyLineIndex(messages, maxWidth);
      return indexRef.current.length;
    }

    if (messagesChanged) {
      // Check if it's just a streaming update (last message changed)
      const isStreamingUpdate =
        messages.length === prevMessages.length ||
        messages.length === prevMessages.length + 1;

      if (isStreamingUpdate && messages.length > 0) {
        // Incremental update
        index.updateLastMessage(messages);
        return index.length;
      } else {
        // Significant change - full rebuild
        index.rebuild(messages, maxWidth);
        return index.length;
      }
    }

    return index.length;
  }, [messages, maxWidth, isWatcherView, isLazyMode]);

  // Get lines in range - stable callback
  const getLines = useCallback(
    (start: number, end: number): ConversationLine[] => {
      const index = indexRef.current;
      if (!index) {
        return [];
      }
      return index.getRange(start, end);
    },
    []
  );

  // Get message index for a line - stable callback
  const getMessageIndex = useCallback((lineIndex: number): number => {
    const index = indexRef.current;
    if (!index) {
      return -1;
    }
    return index.getMessageIndexForLine(lineIndex);
  }, []);

  // Get all lines (fallback) - stable callback
  const getAllLines = useCallback((): ConversationLine[] => {
    const index = indexRef.current;
    if (!index) {
      return [];
    }
    return index.getAllLines();
  }, []);

  return {
    lineCount,
    getLines,
    getMessageIndex,
    getAllLines,
    isLazyMode,
  };
}
