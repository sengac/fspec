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
    placeholder: "Type a message... ('Shift+↑/↓' history | 'Tab' select turn)",
  };

  describe('loading state', () => {
    it('should show ThinkingIndicator when isLoading is true', () => {
      const { lastFrame } = render(
        <InputTransition {...defaultProps} isLoading={true} />
      );
      const output = lastFrame();

      expect(output).toContain('Thinking...');
      expect(output).toContain("(Esc to stop | 'Shift+←/→' sessions | 'Tab' select turn)");
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
      expect(output).toContain("Type a message... ('Shift+↑/↓' history | 'Tab' select turn)");
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
      expect(output).toContain("Type a message... ('Shift+↑/↓' history | 'Tab' select turn)");
    });
  });

  // Feature: spec/features/compaction-post-inject-loading-state.feature
  describe('Scenario: UI transitions smoothly from Compacting to Thinking without idle flicker', () => {
    it('should keep isThinking=true when transitioning from isCompacting=true to isLoading=true', () => {
      // @step Given the compaction hook state isActive is true
      // @step And the Rust session status is Running from CompactionContinuing
      // Start with both isLoading=true AND isCompacting=true
      // (this is the state during DAG construction after CompactionContinuing)
      const { lastFrame, rerender } = render(
        <InputTransition
          {...defaultProps}
          isLoading={true}
          isCompacting={true}
          compactionProgress={{ phase: 'Building DAG', current: 1, total: 3 }}
        />
      );

      // During compaction, should show compaction status
      const frameDuringCompaction = lastFrame();
      expect(frameDuringCompaction).toMatch(/Compacting|Building DAG/i);

      // @step When CompactionComplete arrives and endCompaction sets isActive to false
      // isCompacting goes false, but isLoading stays true (Rust status still Running)
      rerender(
        <InputTransition
          {...defaultProps}
          isLoading={true}
          isCompacting={false}
        />
      );

      // @step Then isLoading must be true because Rust status is still Running
      // @step And InputTransition isThinking must remain true throughout the transition
      // @step And the display must change from Compacting text to Thinking text without showing idle
      const frameAfterCompaction = lastFrame();
      // Must show Thinking indicator (NOT the input placeholder)
      expect(frameAfterCompaction).toContain('Thinking...');
      // Must NOT show the idle input placeholder
      expect(frameAfterCompaction).not.toContain("Type a message...");
    });

    it('should NOT show idle input when only isCompacting changes to false while isLoading is true', () => {
      // Start with compacting active and loading active
      const { lastFrame, rerender } = render(
        <InputTransition
          {...defaultProps}
          isLoading={true}
          isCompacting={true}
          compactionProgress={{ phase: 'Analyzing context', current: 0, total: 1 }}
        />
      );

      // Transition: isCompacting false, isLoading still true
      rerender(
        <InputTransition
          {...defaultProps}
          isLoading={true}
          isCompacting={false}
        />
      );

      // Must show Thinking indicator, not idle
      const output = lastFrame();
      expect(output).toContain('Thinking...');
      expect(output).toContain("(Esc to stop");
    });
  });
});
