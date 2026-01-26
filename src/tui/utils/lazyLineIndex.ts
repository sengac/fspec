/**
 * Lazy Line Index for VirtualList Optimization
 *
 * PERF-004: Viewport-aware lazy line computation
 *
 * Problem:
 * For very large conversations (1000+ messages, 100K+ lines), eagerly computing
 * all ConversationLine objects is expensive and unnecessary since VirtualList
 * only renders a small viewport window.
 *
 * Solution:
 * Build a lightweight index that tracks line boundaries per message without
 * creating line objects. Lines are computed on-demand only for the viewport
 * range, with caching for scroll smoothness.
 *
 * Architecture:
 * ```
 * ┌──────────────────────────────────────────────────────────────────┐
 * │  Message 0 (lines 0-14)      [15 lines] - NOT COMPUTED          │
 * ├──────────────────────────────────────────────────────────────────┤
 * │  Message 1 (lines 15-22)     [8 lines]  - NOT COMPUTED          │
 * ├──────────────────────────────────────────────────────────────────┤
 * │  Message 2 (lines 23-45)     [23 lines] - CACHED (near viewport)│
 * ├──────────────────────────────────────────────────────────────────┤
 * │  Message 3 (lines 46-60)     [15 lines] - IN VIEWPORT ◄─────────│
 * ├──────────────────────────────────────────────────────────────────┤
 * │  Message 4 (lines 61-75)     [15 lines] - CACHED (near viewport)│
 * ├──────────────────────────────────────────────────────────────────┤
 * │  Message 5 (lines 76-100)    [25 lines] - NOT COMPUTED          │
 * └──────────────────────────────────────────────────────────────────┘
 * ```
 */

import type {
  ConversationMessage,
  ConversationLine,
} from '../types/conversation';
import { wrapMessageToLines } from './conversationUtils';
import { normalizeEmojiWidth, getVisualWidth } from './stringWidth';

/**
 * Index entry tracking line boundaries for a single message
 */
interface MessageLineEntry {
  messageIndex: number;
  startLine: number; // First line index (inclusive)
  endLine: number; // Last line index (exclusive)
  lineCount: number;
  // Cache state
  cachedLines?: ConversationLine[];
  cacheKey?: string; // For invalidation (content + width)
}

/**
 * Fast line count estimation for a message.
 *
 * Counts lines WITHOUT creating ConversationLine objects by:
 * 1. Counting newlines in content
 * 2. Estimating wrap points based on visual width
 * 3. Adding separator line
 *
 * This is O(content.length) but much faster than full line wrapping
 * since it skips object creation, array allocation, and most string operations.
 */
function estimateLineCount(msg: ConversationMessage, maxWidth: number): number {
  // Normalize content for consistent width calculation
  const content = normalizeEmojiWidth(msg.content);

  // Split on newlines first
  const paragraphs = content.split('\n');
  let totalLines = 0;

  // Add prefix width for first line
  const isThinking = msg.type === 'thinking';
  const isWatcher = msg.type === 'watcher-input';
  const prefixWidth =
    isThinking || isWatcher
      ? 0
      : msg.type === 'user-input'
        ? 5 // "You: "
        : msg.type === 'assistant-text'
          ? 2
          : 0; // "● "

  for (let i = 0; i < paragraphs.length; i++) {
    const paragraph = paragraphs[i];

    // Empty paragraph = 1 line (space placeholder)
    if (paragraph.length === 0) {
      totalLines += 1;
      continue;
    }

    // Calculate effective width (first line has prefix)
    const effectiveMaxWidth = i === 0 ? maxWidth - prefixWidth : maxWidth;

    // Get visual width of paragraph
    const paragraphWidth = getVisualWidth(paragraph);

    // Estimate lines needed for this paragraph
    // Add 1 to handle partial lines (ceiling division)
    const linesNeeded = Math.max(
      1,
      Math.ceil(paragraphWidth / Math.max(1, effectiveMaxWidth))
    );
    totalLines += linesNeeded;
  }

  // Add streaming indicator if applicable (might add to last line, not extra line)
  // Add separator line at end
  totalLines += 1; // Separator

  return totalLines;
}

/**
 * Cache entry for line ranges (LRU-style management)
 */
interface LineCacheEntry {
  lines: ConversationLine[];
  lastAccess: number;
}

/**
 * LazyLineIndex provides efficient viewport-aware access to conversation lines.
 *
 * Key features:
 * - O(n) index build (fast - just counts lines per message)
 * - O(log n) lookup for any line index (binary search)
 * - O(viewport) materialization (only creates lines in range)
 * - LRU caching for scroll smoothness
 */
export class LazyLineIndex {
  private messages: ConversationMessage[];
  private maxWidth: number;
  private index: MessageLineEntry[] = [];
  private totalLines: number = 0;
  private lineCache: Map<number, LineCacheEntry> = new Map();

  // Cache configuration
  private static readonly MAX_CACHED_MESSAGES = 50; // Keep ~50 messages worth of lines cached
  private static readonly BUFFER_MESSAGES = 5; // Pre-cache N messages before/after viewport

  constructor(messages: ConversationMessage[], maxWidth: number) {
    this.messages = messages;
    this.maxWidth = maxWidth;
    this.buildIndex();
  }

  /**
   * Build the lightweight line index.
   * O(n) pass through messages, counting lines without creating objects.
   */
  private buildIndex(): void {
    this.index = [];
    let currentLine = 0;

    for (let i = 0; i < this.messages.length; i++) {
      const msg = this.messages[i];
      const lineCount = estimateLineCount(msg, this.maxWidth);

      this.index.push({
        messageIndex: i,
        startLine: currentLine,
        endLine: currentLine + lineCount,
        lineCount,
      });

      currentLine += lineCount;
    }

    this.totalLines = currentLine;
  }

  /**
   * Get total line count (for VirtualList items.length)
   */
  get length(): number {
    return this.totalLines;
  }

  /**
   * Binary search to find the message containing a specific line index.
   * Returns the index into this.index (not the messageIndex).
   */
  private findMessageForLine(lineIndex: number): number {
    if (this.index.length === 0) {
      return -1;
    }

    let left = 0;
    let right = this.index.length - 1;

    while (left < right) {
      const mid = Math.floor((left + right) / 2);
      if (this.index[mid].endLine <= lineIndex) {
        left = mid + 1;
      } else {
        right = mid;
      }
    }

    return left;
  }

  /**
   * Get the messageIndex for a given line index.
   * Used by VirtualList's groupBy function.
   */
  getMessageIndexForLine(lineIndex: number): number {
    if (lineIndex < 0 || lineIndex >= this.totalLines) {
      return -1;
    }
    const entryIdx = this.findMessageForLine(lineIndex);
    return entryIdx >= 0 ? this.index[entryIdx].messageIndex : -1;
  }

  /**
   * Compute and cache lines for a specific message.
   */
  private computeLinesForMessage(entryIdx: number): ConversationLine[] {
    const entry = this.index[entryIdx];
    if (!entry) {
      return [];
    }

    const msg = this.messages[entry.messageIndex];
    const cacheKey = `${msg.content}-${msg.isStreaming}-${this.maxWidth}`;

    // Check if already cached with same key
    if (entry.cachedLines && entry.cacheKey === cacheKey) {
      return entry.cachedLines;
    }

    // Compute lines
    const lines = wrapMessageToLines(
      msg,
      entry.messageIndex,
      this.maxWidth,
      true
    );

    // Cache them
    entry.cachedLines = lines;
    entry.cacheKey = cacheKey;

    // Update line cache for LRU management
    this.lineCache.set(entry.messageIndex, {
      lines,
      lastAccess: Date.now(),
    });

    // Prune cache if too large
    this.pruneCache();

    return lines;
  }

  /**
   * Prune old cache entries to prevent memory bloat.
   */
  private pruneCache(): void {
    if (this.lineCache.size <= LazyLineIndex.MAX_CACHED_MESSAGES) {
      return;
    }

    // Sort by last access time, remove oldest
    const entries = Array.from(this.lineCache.entries()).sort(
      (a, b) => a[1].lastAccess - b[1].lastAccess
    );

    const toRemove = entries.slice(
      0,
      entries.length - LazyLineIndex.MAX_CACHED_MESSAGES
    );
    for (const [msgIdx] of toRemove) {
      this.lineCache.delete(msgIdx);
      // Also clear from index entry
      const entry = this.index.find(e => e.messageIndex === msgIdx);
      if (entry) {
        entry.cachedLines = undefined;
        entry.cacheKey = undefined;
      }
    }
  }

  /**
   * Get lines in a specific range (for VirtualList's visibleItems).
   *
   * This is the core optimization: only computes lines for messages
   * that overlap with the requested range, plus a buffer for smooth scrolling.
   */
  getRange(start: number, end: number): ConversationLine[] {
    if (this.index.length === 0 || start >= end) {
      return [];
    }

    // Clamp range
    const clampedStart = Math.max(0, start);
    const clampedEnd = Math.min(this.totalLines, end);

    // Find first message containing start line
    const firstEntryIdx = this.findMessageForLine(clampedStart);
    if (firstEntryIdx < 0) {
      return [];
    }

    const result: ConversationLine[] = [];

    // Pre-cache buffer messages before viewport (for scroll up)
    const bufferStartIdx = Math.max(
      0,
      firstEntryIdx - LazyLineIndex.BUFFER_MESSAGES
    );
    for (let i = bufferStartIdx; i < firstEntryIdx; i++) {
      this.computeLinesForMessage(i); // Just cache, don't add to result
    }

    // Iterate through messages that overlap with range
    for (
      let entryIdx = firstEntryIdx;
      entryIdx < this.index.length;
      entryIdx++
    ) {
      const entry = this.index[entryIdx];

      // Stop if message starts after our range
      if (entry.startLine >= clampedEnd) {
        // Pre-cache a few messages after viewport (for scroll down)
        const bufferEndIdx = Math.min(
          this.index.length,
          entryIdx + LazyLineIndex.BUFFER_MESSAGES
        );
        for (let i = entryIdx; i < bufferEndIdx; i++) {
          this.computeLinesForMessage(i);
        }
        break;
      }

      // Compute lines for this message
      const messageLines = this.computeLinesForMessage(entryIdx);

      // Extract the portion that falls within [clampedStart, clampedEnd)
      const localStart = Math.max(0, clampedStart - entry.startLine);
      const localEnd = Math.min(
        messageLines.length,
        clampedEnd - entry.startLine
      );

      for (let j = localStart; j < localEnd; j++) {
        result.push(messageLines[j]);
      }
    }

    return result;
  }

  /**
   * Get all lines (fallback for small lists or when full array is needed).
   * Use sparingly - defeats the purpose of lazy loading.
   */
  getAllLines(): ConversationLine[] {
    return this.getRange(0, this.totalLines);
  }

  /**
   * Check if the index is still valid for the given messages.
   * Returns false if messages have changed and index needs rebuild.
   */
  isValidFor(messages: ConversationMessage[], maxWidth: number): boolean {
    if (
      messages.length !== this.messages.length ||
      maxWidth !== this.maxWidth
    ) {
      return false;
    }

    // For streaming updates, only the last message typically changes
    // Quick check: compare last message content and streaming state
    if (messages.length > 0) {
      const lastIdx = messages.length - 1;
      const newLast = messages[lastIdx];
      const oldLast = this.messages[lastIdx];

      if (
        newLast.content !== oldLast.content ||
        newLast.isStreaming !== oldLast.isStreaming
      ) {
        return false;
      }
    }

    return true;
  }

  /**
   * Update the index for a change in the last message (common streaming case).
   * More efficient than full rebuild when only appending/updating last message.
   */
  updateLastMessage(messages: ConversationMessage[]): void {
    if (messages.length === 0) {
      this.messages = messages;
      this.index = [];
      this.totalLines = 0;
      return;
    }

    this.messages = messages;
    const lastIdx = messages.length - 1;
    const lastMsg = messages[lastIdx];

    if (lastIdx < this.index.length) {
      // Update existing entry
      const entry = this.index[lastIdx];
      const newLineCount = estimateLineCount(lastMsg, this.maxWidth);
      const oldEndLine = entry.endLine;
      const delta = newLineCount - entry.lineCount;

      entry.lineCount = newLineCount;
      entry.endLine = entry.startLine + newLineCount;

      // Invalidate cache for this message
      entry.cachedLines = undefined;
      entry.cacheKey = undefined;
      this.lineCache.delete(lastIdx);

      // Update total lines
      this.totalLines += delta;

      // No need to update subsequent entries since this is the last one
    } else {
      // New message added - append to index
      const prevEnd =
        this.index.length > 0 ? this.index[this.index.length - 1].endLine : 0;
      const lineCount = estimateLineCount(lastMsg, this.maxWidth);

      this.index.push({
        messageIndex: lastIdx,
        startLine: prevEnd,
        endLine: prevEnd + lineCount,
        lineCount,
      });

      this.totalLines = prevEnd + lineCount;
    }
  }

  /**
   * Full rebuild of the index. Call when messages change significantly.
   */
  rebuild(messages: ConversationMessage[], maxWidth: number): void {
    this.messages = messages;
    this.maxWidth = maxWidth;
    this.lineCache.clear();
    this.buildIndex();
  }

  /**
   * Clear all caches (call on unmount or when index is disposed)
   */
  dispose(): void {
    this.lineCache.clear();
    this.index.forEach(entry => {
      entry.cachedLines = undefined;
      entry.cacheKey = undefined;
    });
  }
}

/**
 * Create a new LazyLineIndex for a conversation.
 *
 * @param messages - Conversation messages
 * @param maxWidth - Maximum line width for wrapping
 * @returns LazyLineIndex instance
 */
export function createLazyLineIndex(
  messages: ConversationMessage[],
  maxWidth: number
): LazyLineIndex {
  return new LazyLineIndex(messages, maxWidth);
}
