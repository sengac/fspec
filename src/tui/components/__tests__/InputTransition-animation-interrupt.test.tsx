/**
 * Tests for InputTransition animation behavior
 *
 * Verifies:
 * - skipAnimation prop works correctly
 * - Enter propagates to MultiLineInput
 * - Loading state shows thinking indicator
 *
 * INPUT-001: Tests input handling integration
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from 'ink-testing-library';
import { InputTransition } from '../InputTransition';
import { InputManager } from '../../input/InputManager';

describe('InputTransition animation interrupt', () => {
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
    placeholder: "Type a message...",
    isActive: true,
  };

  describe('skipAnimation prop', () => {
    it('should immediately show input when skipAnimation is true', () => {
      const { lastFrame } = render(
        <InputManager>
          <InputTransition {...defaultProps} isLoading={false} skipAnimation={true} />
        </InputManager>
      );

      // Should immediately show placeholder without animation
      expect(lastFrame()).toContain('Type a message...');
    });

    it('should skip animation after loading ends with skipAnimation', async () => {
      vi.useRealTimers();
      
      const { lastFrame, rerender } = render(
        <InputManager>
          <InputTransition {...defaultProps} isLoading={true} />
        </InputManager>
      );

      // Initially showing loading
      expect(lastFrame()).toContain('Thinking');

      // Transition with skipAnimation (simulating session switch)
      rerender(
        <InputManager>
          <InputTransition {...defaultProps} isLoading={false} skipAnimation={true} />
        </InputManager>
      );

      await new Promise(resolve => setTimeout(resolve, 50));

      // Should immediately show input
      expect(lastFrame()).toContain('Type a message...');
    });
  });

  describe('Enter key handling', () => {
    it('should call onSubmit when Enter is pressed and not loading', async () => {
      vi.useRealTimers();
      const onSubmit = vi.fn();

      // Start in non-loading state with skipAnimation to avoid animation
      const { stdin } = render(
        <InputManager>
          <InputTransition 
            {...defaultProps} 
            isLoading={false} 
            onSubmit={onSubmit}
            skipAnimation={true}
          />
        </InputManager>
      );

      await new Promise(resolve => setTimeout(resolve, 50));

      // Press Enter
      stdin.write('\r');
      await new Promise(resolve => setTimeout(resolve, 50));

      expect(onSubmit).toHaveBeenCalledTimes(1);
    });

    it('should NOT call onSubmit when loading', async () => {
      vi.useRealTimers();
      const onSubmit = vi.fn();

      const { stdin } = render(
        <InputManager>
          <InputTransition {...defaultProps} isLoading={true} onSubmit={onSubmit} />
        </InputManager>
      );

      // Press Enter while loading
      stdin.write('\r');
      await new Promise(resolve => setTimeout(resolve, 50));

      // Should not submit while loading
      expect(onSubmit).not.toHaveBeenCalled();
    });
  });

  describe('control keys during animation', () => {
    it('should ignore Escape during animation (not modify input)', async () => {
      vi.useRealTimers();
      const onChange = vi.fn();

      const { rerender, stdin } = render(
        <InputManager>
          <InputTransition {...defaultProps} isLoading={true} onChange={onChange} />
        </InputManager>
      );

      // Start animation
      rerender(
        <InputManager>
          <InputTransition {...defaultProps} isLoading={false} onChange={onChange} />
        </InputManager>
      );

      // Press Escape during animation
      stdin.write('\x1b');
      await new Promise(resolve => setTimeout(resolve, 50));

      // onChange should NOT be called (Escape doesn't capture input)
      expect(onChange).not.toHaveBeenCalled();
    });

    it('should ignore Ctrl keys during animation', async () => {
      vi.useRealTimers();
      const onChange = vi.fn();

      const { rerender, stdin } = render(
        <InputManager>
          <InputTransition {...defaultProps} isLoading={true} onChange={onChange} />
        </InputManager>
      );

      rerender(
        <InputManager>
          <InputTransition {...defaultProps} isLoading={false} onChange={onChange} />
        </InputManager>
      );

      // Press Ctrl+C during animation
      stdin.write('\x03');
      await new Promise(resolve => setTimeout(resolve, 50));

      // onChange should NOT be called
      expect(onChange).not.toHaveBeenCalled();
    });
  });

  describe('loading state', () => {
    it('should show thinking indicator when loading', () => {
      const { lastFrame } = render(
        <InputManager>
          <InputTransition {...defaultProps} isLoading={true} />
        </InputManager>
      );

      expect(lastFrame()).toContain('Thinking');
    });

    it('should show custom thinking message', () => {
      const { lastFrame } = render(
        <InputManager>
          <InputTransition {...defaultProps} isLoading={true} thinkingMessage="Processing" />
        </InputManager>
      );

      expect(lastFrame()).toContain('Processing');
    });
  });
});
