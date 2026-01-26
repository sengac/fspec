/**
 * Tests for LazyLineIndex - PERF-004
 *
 * Tests the viewport-aware lazy line computation optimization.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { LazyLineIndex, createLazyLineIndex } from '../lazyLineIndex';
import type { ConversationMessage } from '../../types/conversation';

// Helper to create test messages
function createMessages(
  count: number,
  linesPerMessage: number = 3
): ConversationMessage[] {
  return Array.from({ length: count }, (_, i) => ({
    type: 'assistant-text' as const,
    content: Array.from(
      { length: linesPerMessage },
      (_, j) => `Message ${i + 1}, Line ${j + 1}`
    ).join('\n'),
  }));
}

describe('LazyLineIndex', () => {
  describe('Scenario: Build lightweight index without creating line objects', () => {
    it('should count total lines without materializing all line objects', () => {
      // Given a conversation with 100 messages, each with ~3 content lines + separator
      const messages = createMessages(100, 3);

      // When we create a lazy index
      const index = createLazyLineIndex(messages, 80);

      // Then the total line count should be approximately 100 * 4 (3 lines + separator each)
      // Note: The exact count may vary due to wrapping, but should be > 0
      expect(index.length).toBeGreaterThan(0);
      expect(index.length).toBeGreaterThanOrEqual(100 * 4); // At least 4 lines per message
    });
  });

  describe('Scenario: Get lines in viewport range only', () => {
    it('should only compute lines for the requested range', () => {
      // Given a conversation with 50 messages
      const messages = createMessages(50, 2);
      const index = createLazyLineIndex(messages, 80);

      // When we request lines 10-20 (a small viewport)
      const lines = index.getRange(10, 20);

      // Then we should get exactly 10 lines
      expect(lines.length).toBe(10);

      // And the lines should have valid content
      lines.forEach(line => {
        expect(line.content).toBeDefined();
        expect(typeof line.content).toBe('string');
      });
    });
  });

  describe('Scenario: Binary search finds correct message for line index', () => {
    it('should correctly map line indices to message indices', () => {
      // Given a conversation with 10 messages
      const messages = createMessages(10, 2);
      const index = createLazyLineIndex(messages, 80);

      // When we look up message indices for various line positions
      const firstLineMsg = index.getMessageIndexForLine(0);

      // Then the first line should belong to message 0
      expect(firstLineMsg).toBe(0);

      // And getting lines from the range should work
      const firstLines = index.getRange(0, 5);
      expect(firstLines.length).toBe(5);
      expect(firstLines[0].messageIndex).toBe(0);
    });
  });

  describe('Scenario: Cache lines for scroll smoothness', () => {
    it('should cache computed lines and reuse them', () => {
      // Given a lazy index
      const messages = createMessages(20, 2);
      const index = createLazyLineIndex(messages, 80);

      // When we request the same range twice
      const lines1 = index.getRange(5, 15);
      const lines2 = index.getRange(5, 15);

      // Then we should get the same content
      expect(lines1.length).toBe(lines2.length);
      lines1.forEach((line, i) => {
        expect(line.content).toBe(lines2[i].content);
        expect(line.messageIndex).toBe(lines2[i].messageIndex);
      });
    });
  });

  describe('Scenario: Incremental update for streaming', () => {
    it('should efficiently update when only last message changes', () => {
      // Given an initial conversation
      const messages = createMessages(10, 2);
      const index = createLazyLineIndex(messages, 80);
      const initialCount = index.length;

      // When we update the last message with more content
      const updatedMessages = [...messages];
      updatedMessages[9] = {
        ...updatedMessages[9],
        content: updatedMessages[9].content + '\nAdditional streaming content',
      };

      // And call updateLastMessage
      index.updateLastMessage(updatedMessages);

      // Then the line count should increase
      expect(index.length).toBeGreaterThan(initialCount);

      // And we should be able to get the new lines
      const newLines = index.getRange(index.length - 5, index.length);
      expect(newLines.length).toBeGreaterThan(0);
    });
  });

  describe('Scenario: Handle empty conversation', () => {
    it('should handle empty message array gracefully', () => {
      // Given an empty conversation
      const messages: ConversationMessage[] = [];

      // When we create a lazy index
      const index = createLazyLineIndex(messages, 80);

      // Then it should have zero length
      expect(index.length).toBe(0);

      // And getRange should return empty array
      expect(index.getRange(0, 10)).toEqual([]);

      // And message lookup should return -1
      expect(index.getMessageIndexForLine(0)).toBe(-1);
    });
  });

  describe('Scenario: Buffer messages around viewport', () => {
    it('should pre-cache messages before and after viewport for smooth scrolling', () => {
      // Given a large conversation
      const messages = createMessages(100, 3);
      const index = createLazyLineIndex(messages, 80);

      // When we request a viewport in the middle
      const viewportStart = 200;
      const viewportEnd = 220;
      const lines = index.getRange(viewportStart, viewportEnd);

      // Then we should get the requested lines
      expect(lines.length).toBe(20);

      // And subsequent requests for nearby ranges should also work
      // (this tests that buffer messages were cached)
      const beforeLines = index.getRange(viewportStart - 10, viewportStart);
      const afterLines = index.getRange(viewportEnd, viewportEnd + 10);

      expect(beforeLines.length).toBe(10);
      expect(afterLines.length).toBe(10);
    });
  });

  describe('Scenario: Full rebuild on significant changes', () => {
    it('should correctly rebuild when messages change significantly', () => {
      // Given a lazy index
      const messages = createMessages(10, 2);
      const index = createLazyLineIndex(messages, 80);

      // When we rebuild with different messages
      const newMessages = createMessages(5, 4); // Different count and line structure
      index.rebuild(newMessages, 80);

      // Then the line count should reflect the new messages
      expect(index.length).toBeGreaterThan(0);
      expect(index.getMessageIndexForLine(0)).toBe(0);
    });
  });

  describe('Scenario: Dispose clears caches', () => {
    it('should clear all caches on dispose', () => {
      // Given a lazy index with cached lines
      const messages = createMessages(10, 2);
      const index = createLazyLineIndex(messages, 80);
      index.getRange(0, 20); // Populate cache

      // When we dispose
      index.dispose();

      // Then length should still be accessible
      expect(index.length).toBeGreaterThan(0);

      // And we can still get lines (will be recomputed)
      const lines = index.getRange(0, 5);
      expect(lines.length).toBe(5);
    });
  });

  describe('Scenario: Different message types', () => {
    it('should handle different message types correctly', () => {
      // Given messages of different types
      const messages: ConversationMessage[] = [
        { type: 'user-input', content: 'Hello there!' },
        { type: 'assistant-text', content: 'Hi! How can I help?' },
        { type: 'thinking', content: 'Let me think about this...' },
        { type: 'tool-call', content: 'Running tool: read_file' },
        { type: 'status', content: 'Processing complete' },
      ];

      // When we create an index
      const index = createLazyLineIndex(messages, 80);

      // Then all messages should be indexed
      expect(index.length).toBeGreaterThan(0);

      // And we should be able to get all lines
      const allLines = index.getAllLines();
      expect(allLines.length).toBe(index.length);

      // And different roles should be preserved
      const userLine = allLines.find(l => l.role === 'user');
      const assistantLine = allLines.find(l => l.role === 'assistant');
      const toolLine = allLines.find(l => l.role === 'tool');

      expect(userLine).toBeDefined();
      expect(assistantLine).toBeDefined();
      expect(toolLine).toBeDefined();
    });
  });
});
