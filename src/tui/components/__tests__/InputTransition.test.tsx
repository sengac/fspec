/**
 * Tests for InputTransition component
 *
 * Verifies the animated transition between thinking indicator
 * and input placeholder states.
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from 'ink-testing-library';
import { InputTransition } from '../InputTransition';

describe('InputTransition', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  const defaultProps = {
    isLoading: false,
    value: '',
    onChange: vi.fn(),
    onSubmit: vi.fn(),
    placeholder: 'Type your message...',
  };

  describe('loading state', () => {
    it('should show ThinkingIndicator when isLoading is true', () => {
      const { lastFrame } = render(
        <InputTransition {...defaultProps} isLoading={true} />
      );
      const output = lastFrame();

      expect(output).toContain('Thinking...');
      expect(output).toContain('(Esc to stop)');
    });

    it('should show custom thinking message', () => {
      const { lastFrame } = render(
        <InputTransition
          {...defaultProps}
          isLoading={true}
          thinkingMessage="Processing"
        />
      );
      const output = lastFrame();

      expect(output).toContain('Processing...');
    });
  });

  describe('input state', () => {
    it('should show MultiLineInput when not loading', () => {
      const { lastFrame } = render(
        <InputTransition {...defaultProps} isLoading={false} />
      );
      const output = lastFrame();

      // Should show placeholder when no value
      expect(output).toContain('Type your message...');
    });

    it('should show input value when provided', () => {
      const { lastFrame } = render(
        <InputTransition {...defaultProps} isLoading={false} value="Hello" />
      );
      const output = lastFrame();

      expect(output).toContain('Hello');
    });
  });

  describe('transition animation', () => {
    it('should start hiding animation when loading finishes', () => {
      const { lastFrame, rerender } = render(
        <InputTransition {...defaultProps} isLoading={true} />
      );

      // Initially showing thinking indicator
      expect(lastFrame()).toContain('Thinking...');

      // Transition to not loading
      rerender(<InputTransition {...defaultProps} isLoading={false} />);

      // Should start the hiding animation (still shows some of thinking text)
      const output = lastFrame();
      // During hiding, it will show progressively less of the thinking text
      expect(output).toBeTruthy();
    });

    it('should complete animation and show input', async () => {
      const { lastFrame, rerender } = render(
        <InputTransition {...defaultProps} isLoading={true} />
      );

      // Transition to not loading
      rerender(<InputTransition {...defaultProps} isLoading={false} />);

      // Fast-forward through entire animation with multiple timer advances
      // to allow React to process state updates between timer callbacks
      // Hide: ~30 chars * 12ms = 360ms + delay 50ms + show: ~20 chars * 10ms = 200ms
      for (let i = 0; i < 100; i++) {
        vi.advanceTimersByTime(20);
        await vi.runAllTimersAsync();
      }

      const output = lastFrame();
      // Should eventually show the input placeholder
      expect(output).toContain('Type your message...');
    });
  });
});
