/**
 * Tests for ThinkingBlockHandler
 *
 * BRIDGE-006: Intelligent Content-Aware Chunking for Telegram Display
 *
 * The ThinkingBlockHandler manages the state and formatting of thinking blocks
 * that are wrapped in <think>...</think> tags for Telegram display.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { ThinkingBlockHandler } from '../telegram-thinking-handler';

describe('Feature: Thinking Block Handler', () => {
  let handler: ThinkingBlockHandler;

  beforeEach(() => {
    handler = new ThinkingBlockHandler();
  });

  describe('Scenario: First thinking chunk opens block', () => {
    it('should return opening tag + content for first thinking chunk', () => {
      // @step Given a new handler with no open block
      expect(handler.isOpen()).toBe(false);

      // @step When processing the first thinking chunk
      const result = handler.processThinking('First thought');

      // @step Then it returns the opening tag followed by escaped content
      expect(result).toBe('\\<think\\>First thought');

      // @step And the block is now open
      expect(handler.isOpen()).toBe(true);
    });
  });

  describe('Scenario: Subsequent thinking chunks append content only', () => {
    it('should return only content for subsequent thinking chunks', () => {
      // @step Given a handler with an open thinking block
      handler.processThinking('First thought');
      expect(handler.isOpen()).toBe(true);

      // @step When processing another thinking chunk
      const result = handler.processThinking('Second thought');

      // @step Then it returns only the content (no opening tag)
      expect(result).toBe('Second thought');

      // @step And the block remains open
      expect(handler.isOpen()).toBe(true);
    });
  });

  describe('Scenario: Closing an open block', () => {
    it('should return closing tag when block is open', () => {
      // @step Given a handler with an open thinking block
      handler.processThinking('Some thought');
      expect(handler.isOpen()).toBe(true);

      // @step When closing the block
      const result = handler.close();

      // @step Then it returns the closing tag with newline separator
      expect(result).toBe('\\</think\\>\n\n');

      // @step And the block is now closed
      expect(handler.isOpen()).toBe(false);
    });
  });

  describe('Scenario: Closing an already closed block', () => {
    it('should return empty string when block is already closed', () => {
      // @step Given a handler with no open block
      expect(handler.isOpen()).toBe(false);

      // @step When attempting to close
      const result = handler.close();

      // @step Then it returns empty string (idempotent)
      expect(result).toBe('');

      // @step And the block remains closed
      expect(handler.isOpen()).toBe(false);
    });
  });

  describe('Scenario: Reset clears state', () => {
    it('should close block and reset state', () => {
      // @step Given a handler with an open thinking block
      handler.processThinking('Some thought');
      expect(handler.isOpen()).toBe(true);

      // @step When resetting
      handler.reset();

      // @step Then the block is closed
      expect(handler.isOpen()).toBe(false);
    });
  });

  describe('Scenario: Multiple open/close cycles', () => {
    it('should handle multiple thinking blocks correctly', () => {
      // @step Given a handler
      // First block
      const first1 = handler.processThinking('First block, first thought');
      expect(first1).toBe('\\<think\\>First block, first thought');

      const first2 = handler.processThinking('First block, second thought');
      expect(first2).toBe('First block, second thought');

      const close1 = handler.close();
      expect(close1).toBe('\\</think\\>\n\n');

      // Second block
      const second1 = handler.processThinking('Second block, first thought');
      expect(second1).toBe('\\<think\\>Second block, first thought');

      const close2 = handler.close();
      expect(close2).toBe('\\</think\\>\n\n');

      // Verify final state
      expect(handler.isOpen()).toBe(false);
    });
  });

  describe('Scenario: closeIfOpen convenience method', () => {
    it('should close and return tag if open', () => {
      handler.processThinking('thought');
      const result = handler.closeIfOpen();
      expect(result).toBe('\\</think\\>\n\n');
      expect(handler.isOpen()).toBe(false);
    });

    it('should return empty string if not open', () => {
      const result = handler.closeIfOpen();
      expect(result).toBe('');
    });
  });
});
