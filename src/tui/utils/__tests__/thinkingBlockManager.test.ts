/**
 * Feature: spec/features/thinking-block-manager.feature
 *
 * Tests for the Thinking Block Manager module.
 * Validates thinking block streaming, accumulation, and finalization.
 */

import { describe, it, expect } from 'vitest';
import {
  findActiveThinkingBlock,
  findAppendableThinkingBlock,
  appendThinking,
  finalizeThinkingBlock,
  appendThinkingBulk,
  createThinkingUpdate,
  createFinalizationUpdate,
} from '../thinkingBlockManager';
import type { ConversationMessage } from '../../types/conversation';

describe('Feature: Thinking Block Manager', () => {
  describe('Scenario: Find active thinking block', () => {
    it('should return -1 when no thinking blocks exist', () => {
      // @step Given an empty conversation
      const messages: ConversationMessage[] = [];

      // @step When I find the active thinking block
      const result = findActiveThinkingBlock(messages);

      // @step Then the result should be -1
      expect(result).toBe(-1);
    });

    it('should return -1 when thinking block is finalized', () => {
      // @step Given a conversation with a finalized thinking block
      const messages: ConversationMessage[] = [
        {
          type: 'thinking',
          content: '[Thinking]\nSome thought',
          isStreaming: false,
        },
      ];

      // @step When I find the active thinking block
      const result = findActiveThinkingBlock(messages);

      // @step Then the result should be -1
      expect(result).toBe(-1);
    });

    it('should return index when thinking block is streaming', () => {
      // @step Given a conversation with a streaming thinking block
      const messages: ConversationMessage[] = [
        { type: 'user-input', content: 'Hello' },
        {
          type: 'thinking',
          content: '[Thinking]\nSome thought',
          isStreaming: true,
        },
      ];

      // @step When I find the active thinking block
      const result = findActiveThinkingBlock(messages);

      // @step Then the result should be 1
      expect(result).toBe(1);
    });

    it('should return last active thinking block when multiple exist', () => {
      // @step Given a conversation with multiple thinking blocks
      const messages: ConversationMessage[] = [
        { type: 'thinking', content: '[Thinking]\nFirst', isStreaming: false },
        { type: 'tool-call', content: '● Edit(file.ts)', toolCallId: '1' },
        { type: 'thinking', content: '[Thinking]\nSecond', isStreaming: true },
      ];

      // @step When I find the active thinking block
      const result = findActiveThinkingBlock(messages);

      // @step Then the result should be 2 (the second thinking block)
      expect(result).toBe(2);
    });

    it('should return -1 when user-input comes after streaming thinking block', () => {
      // @step Given a streaming thinking block followed by user input
      const messages: ConversationMessage[] = [
        {
          type: 'thinking',
          content: '[Thinking]\nOld thought',
          isStreaming: true,
        },
        { type: 'user-input', content: 'New question' },
      ];

      // @step When I find the active thinking block
      const result = findActiveThinkingBlock(messages);

      // @step Then the result should be -1 (thinking block is from previous turn)
      expect(result).toBe(-1);
    });

    it('should return -1 when watcher-input comes after streaming thinking block', () => {
      // @step Given a streaming thinking block followed by watcher input
      const messages: ConversationMessage[] = [
        {
          type: 'thinking',
          content: '[Thinking]\nOld thought',
          isStreaming: true,
        },
        { type: 'watcher-input', content: '[W] system> Update' },
      ];

      // @step When I find the active thinking block
      const result = findActiveThinkingBlock(messages);

      // @step Then the result should be -1 (thinking block is from previous turn)
      expect(result).toBe(-1);
    });

    it('should return active thinking block when assistant messages follow but no user input', () => {
      // @step Given a streaming thinking block followed by assistant text
      const messages: ConversationMessage[] = [
        {
          type: 'thinking',
          content: '[Thinking]\nCurrent thought',
          isStreaming: true,
        },
        { type: 'assistant-text', content: 'Hello', isStreaming: true },
      ];

      // @step When I find the active thinking block
      const result = findActiveThinkingBlock(messages);

      // @step Then the result should be 0 (still same turn)
      expect(result).toBe(0);
    });
  });

  describe('Scenario: Append thinking content to active block', () => {
    it('should create new thinking block when none exists', () => {
      // @step Given an empty conversation
      const messages: ConversationMessage[] = [];

      // @step When I append thinking content
      appendThinking(messages, 'First thought');

      // @step Then a new thinking block should be created
      expect(messages).toHaveLength(1);
      expect(messages[0].type).toBe('thinking');
      expect(messages[0].content).toBe('[Thinking]\nFirst thought');
      expect(messages[0].isStreaming).toBe(true);
    });

    it('should append to existing active thinking block', () => {
      // @step Given a conversation with an active thinking block
      const messages: ConversationMessage[] = [
        { type: 'thinking', content: '[Thinking]\nFirst', isStreaming: true },
      ];

      // @step When I append more thinking content
      appendThinking(messages, ' Second');

      // @step Then the content should be appended
      expect(messages).toHaveLength(1);
      expect(messages[0].content).toBe('[Thinking]\nFirst Second');
      expect(messages[0].isStreaming).toBe(true);
    });

    it('should create new block after finalized thinking', () => {
      // @step Given a conversation with a finalized thinking block
      const messages: ConversationMessage[] = [
        { type: 'thinking', content: '[Thinking]\nOld', isStreaming: false },
      ];

      // @step When I append new thinking content
      appendThinking(messages, 'New thought');

      // @step Then a new thinking block should be created
      expect(messages).toHaveLength(2);
      expect(messages[1].content).toBe('[Thinking]\nNew thought');
      expect(messages[1].isStreaming).toBe(true);
    });

    it('should create new block when user-input follows streaming thinking', () => {
      // @step Given a streaming thinking block followed by user input
      const messages: ConversationMessage[] = [
        { type: 'thinking', content: '[Thinking]\nOld', isStreaming: true },
        { type: 'user-input', content: 'Next question' },
      ];

      // @step When I append new thinking content
      appendThinking(messages, 'New thought');

      // @step Then a new thinking block should be created (not appended to old)
      expect(messages).toHaveLength(3);
      expect(messages[0].content).toBe('[Thinking]\nOld');
      expect(messages[0].isStreaming).toBe(true); // Old block unchanged
      expect(messages[2].content).toBe('[Thinking]\nNew thought');
      expect(messages[2].isStreaming).toBe(true);
    });

    it('should create new block when watcher-input follows streaming thinking', () => {
      // @step Given a streaming thinking block followed by watcher input
      const messages: ConversationMessage[] = [
        { type: 'thinking', content: '[Thinking]\nOld', isStreaming: true },
        { type: 'watcher-input', content: '[W] system> Update' },
      ];

      // @step When I append new thinking content
      appendThinking(messages, 'New thought');

      // @step Then a new thinking block should be created (not appended to old)
      expect(messages).toHaveLength(3);
      expect(messages[0].content).toBe('[Thinking]\nOld');
      expect(messages[2].content).toBe('[Thinking]\nNew thought');
      expect(messages[2].isStreaming).toBe(true);
    });

    it('should insert before streaming assistant message', () => {
      // @step Given a conversation with a streaming assistant message
      const messages: ConversationMessage[] = [
        { type: 'user-input', content: 'Hello' },
        { type: 'assistant-text', content: '', isStreaming: true },
      ];

      // @step When I append thinking content
      appendThinking(messages, 'Analyzing request');

      // @step Then thinking should be inserted before assistant message
      expect(messages).toHaveLength(3);
      expect(messages[1].type).toBe('thinking');
      expect(messages[2].type).toBe('assistant-text');
    });

    it('should not modify array when content is empty', () => {
      // @step Given an empty conversation
      const messages: ConversationMessage[] = [];

      // @step When I append empty content
      appendThinking(messages, '');

      // @step Then the conversation should remain empty
      expect(messages).toHaveLength(0);
    });
  });

  describe('Scenario: Finalize thinking block on tool call', () => {
    it('should mark active thinking block as complete', () => {
      // @step Given a conversation with an active thinking block
      const messages: ConversationMessage[] = [
        {
          type: 'thinking',
          content: '[Thinking]\nAnalyzing',
          isStreaming: true,
        },
      ];

      // @step When I finalize the thinking block
      finalizeThinkingBlock(messages);

      // @step Then the thinking block should be marked as not streaming
      expect(messages[0].isStreaming).toBe(false);
    });

    it('should do nothing when no active thinking block exists', () => {
      // @step Given a conversation without active thinking
      const messages: ConversationMessage[] = [
        { type: 'user-input', content: 'Hello' },
      ];

      // @step When I finalize thinking blocks
      finalizeThinkingBlock(messages);

      // @step Then the conversation should be unchanged
      expect(messages).toHaveLength(1);
      expect(messages[0].type).toBe('user-input');
    });
  });

  describe('Scenario: Create new thinking block after tool call', () => {
    it('should create separate thinking blocks after tool call', () => {
      // @step Given thinking before tool call
      const messages: ConversationMessage[] = [
        {
          type: 'thinking',
          content: '[Thinking]\nBefore tool',
          isStreaming: true,
        },
      ];

      // @step When tool call arrives, finalize thinking
      finalizeThinkingBlock(messages);

      // @step And tool call is added
      messages.push({
        type: 'tool-call',
        content: '● Edit(file.ts)',
        toolCallId: '1',
      });

      // @step And new thinking content arrives
      appendThinking(messages, 'After tool');

      // @step Then there should be two separate thinking blocks
      expect(messages).toHaveLength(3);
      expect(messages[0].type).toBe('thinking');
      expect(messages[0].isStreaming).toBe(false);
      expect(messages[2].type).toBe('thinking');
      expect(messages[2].isStreaming).toBe(true);
      expect(messages[2].content).toBe('[Thinking]\nAfter tool');
    });
  });

  describe('Scenario: Correlation ID propagation', () => {
    it('should set correlation ID on new thinking block', () => {
      // @step Given an empty conversation
      const messages: ConversationMessage[] = [];

      // @step When I append thinking with correlation ID
      appendThinking(messages, 'Thought', { correlationId: 'corr-123' });

      // @step Then the thinking block should have the correlation ID
      expect(messages[0].correlationId).toBe('corr-123');
    });

    it('should set correlation ID on first append to block', () => {
      // @step Given an active thinking block without correlation ID
      const messages: ConversationMessage[] = [
        { type: 'thinking', content: '[Thinking]\nFirst', isStreaming: true },
      ];

      // @step When I append with correlation ID
      appendThinking(messages, ' more', { correlationId: 'corr-456' });

      // @step Then the correlation ID should be set
      expect(messages[0].correlationId).toBe('corr-456');
    });

    it('should not overwrite existing correlation ID', () => {
      // @step Given an active thinking block with correlation ID
      const messages: ConversationMessage[] = [
        {
          type: 'thinking',
          content: '[Thinking]\nFirst',
          isStreaming: true,
          correlationId: 'original',
        },
      ];

      // @step When I append with different correlation ID
      appendThinking(messages, ' more', { correlationId: 'new' });

      // @step Then the original correlation ID should be preserved
      expect(messages[0].correlationId).toBe('original');
    });
  });

  describe('Scenario: Bulk processing (non-streaming)', () => {
    it('should append to last thinking in same turn', () => {
      // @step Given a thinking block in current turn (no tool call after it)
      const messages: ConversationMessage[] = [
        { type: 'user-input', content: 'Hello' },
        { type: 'thinking', content: '[Thinking]\nPart 1', isStreaming: false },
      ];

      // @step When I append thinking in bulk mode
      appendThinkingBulk(messages, ' Part 2');

      // @step Then content should be appended to existing block
      expect(messages).toHaveLength(2);
      expect(messages[1].content).toBe('[Thinking]\nPart 1 Part 2');
    });

    it('should create new block after tool call', () => {
      // @step Given a thinking block before a tool call
      const messages: ConversationMessage[] = [
        { type: 'thinking', content: '[Thinking]\nOld', isStreaming: false },
        { type: 'tool-call', content: '● Edit(file.ts)', toolCallId: '1' },
      ];

      // @step When I append thinking in bulk mode
      appendThinkingBulk(messages, 'New');

      // @step Then a new thinking block should be created
      expect(messages).toHaveLength(3);
      expect(messages[2].content).toBe('[Thinking]\nNew');
    });

    it('should create new block after user-input (new turn)', () => {
      // @step Given a thinking block followed by user input
      const messages: ConversationMessage[] = [
        { type: 'thinking', content: '[Thinking]\nOld', isStreaming: false },
        { type: 'user-input', content: 'Next question' },
      ];

      // @step When I append thinking in bulk mode
      appendThinkingBulk(messages, 'New thought');

      // @step Then a new thinking block should be created (not appended to old)
      expect(messages).toHaveLength(3);
      expect(messages[0].content).toBe('[Thinking]\nOld');
      expect(messages[2].content).toBe('[Thinking]\nNew thought');
    });

    it('should create new block after watcher-input (new turn)', () => {
      // @step Given a thinking block followed by watcher input
      const messages: ConversationMessage[] = [
        { type: 'thinking', content: '[Thinking]\nOld', isStreaming: true },
        { type: 'watcher-input', content: '[W] system> Update' },
      ];

      // @step When I append thinking in bulk mode
      appendThinkingBulk(messages, 'New thought');

      // @step Then a new thinking block should be created (not appended to old)
      expect(messages).toHaveLength(3);
      expect(messages[0].content).toBe('[Thinking]\nOld');
      expect(messages[2].content).toBe('[Thinking]\nNew thought');
    });
  });

  describe('Scenario: React state updates (immutable)', () => {
    it('should return new array for thinking update', () => {
      // @step Given an empty conversation
      const original: ConversationMessage[] = [];

      // @step When I create a thinking update
      const updated = createThinkingUpdate(original, 'Thought');

      // @step Then the result should be a new array
      expect(updated).not.toBe(original);
      expect(updated).toHaveLength(1);
      expect(original).toHaveLength(0);
    });

    it('should return same array when no change needed', () => {
      // @step Given an empty conversation
      const original: ConversationMessage[] = [];

      // @step When I create update with empty content
      const updated = createThinkingUpdate(original, '');

      // @step Then the same array should be returned
      expect(updated).toBe(original);
    });

    it('should return new array for finalization', () => {
      // @step Given a conversation with active thinking
      const original: ConversationMessage[] = [
        { type: 'thinking', content: '[Thinking]\nTest', isStreaming: true },
      ];

      // @step When I create a finalization update
      const updated = createFinalizationUpdate(original);

      // @step Then the result should be a new array
      expect(updated).not.toBe(original);
      expect(updated[0].isStreaming).toBe(false);
      expect(original[0].isStreaming).toBe(true);
    });

    it('should return same array when no active thinking to finalize', () => {
      // @step Given a conversation without active thinking
      const original: ConversationMessage[] = [
        { type: 'user-input', content: 'Hello' },
      ];

      // @step When I create a finalization update
      const updated = createFinalizationUpdate(original);

      // @step Then the same array should be returned
      expect(updated).toBe(original);
    });
  });

  describe('Scenario: Full streaming flow simulation', () => {
    it('should handle thinking → text → tool → thinking → text flow', () => {
      // @step Given a fresh conversation
      const messages: ConversationMessage[] = [];

      // @step When thinking chunks stream in
      appendThinking(messages, 'Analyzing ');
      appendThinking(messages, 'the request...');

      // @step Then there should be one thinking block
      expect(messages).toHaveLength(1);
      expect(messages[0].content).toBe('[Thinking]\nAnalyzing the request...');

      // @step When text starts streaming
      // (In real code, text would be added separately)
      messages.push({
        type: 'assistant-text',
        content: 'I will ',
        isStreaming: true,
      });

      // @step And more thinking arrives (shouldn't happen normally, but testing robustness)
      appendThinking(messages, 'more thought');

      // @step Then thinking should be appended to existing block (still active)
      expect(messages[0].content).toBe(
        '[Thinking]\nAnalyzing the request...more thought'
      );

      // @step When tool call arrives
      finalizeThinkingBlock(messages);
      messages.push({
        type: 'tool-call',
        content: '● Read(file.ts)',
        toolCallId: '1',
      });

      // @step And thinking starts again
      appendThinking(messages, 'After reading...');

      // @step Then a new thinking block should be created
      const thinkingBlocks = messages.filter(m => m.type === 'thinking');
      expect(thinkingBlocks).toHaveLength(2);
      expect(thinkingBlocks[0].isStreaming).toBe(false);
      expect(thinkingBlocks[1].isStreaming).toBe(true);
    });
  });
});
