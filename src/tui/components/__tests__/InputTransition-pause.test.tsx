/**
 * Tests for InputTransition pause state (PAUSE-001)
 * 
 * Feature: spec/features/interactive-tool-pause-for-browser-debugging.feature
 * 
 * These tests verify the TUI transitions for tool pause functionality.
 * Tests are written BEFORE implementation (ACDD red phase).
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from 'ink-testing-library';
import { InputTransition } from '../InputTransition';

describe('InputTransition Pause State (PAUSE-001)', () => {
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
  };

  // =============================================================================
  // Scenario: TUI transitions from Thinking to Paused and back
  // =============================================================================

  describe('Scenario: TUI transitions from Thinking to Paused and back', () => {
    it('should show pause indicator when isPaused is true', () => {
      // @step Given the agent is processing a request
      // @step And the TUI shows "Thinking..." with the spinner animation
      const { lastFrame, rerender } = render(
        <InputTransition 
          {...defaultProps} 
          isLoading={true}
          isPaused={false}
        />
      );
      expect(lastFrame()).toContain('Thinking...');

      // @step When a tool requests a Continue pause
      // @step Then the TUI should replace "Thinking..." with the pause indicator
      rerender(
        <InputTransition 
          {...defaultProps} 
          isLoading={true}
          isPaused={true}
          pauseInfo={{
            kind: 'continue',
            toolName: 'WebSearch',
            message: 'Page loaded: https://example.com',
          }}
        />
      );

      const output = lastFrame();
      
      // @step And the pause indicator should show the tool name and message
      expect(output).toContain('WebSearch');
      expect(output).toContain('Page loaded');
      expect(output).toContain('Press Enter to continue');
      expect(output).not.toContain('Thinking...');
    });

    it('should return to Thinking state after resume', async () => {
      // @step Given a tool is paused
      const { lastFrame, rerender } = render(
        <InputTransition 
          {...defaultProps} 
          isLoading={true}
          isPaused={true}
          pauseInfo={{
            kind: 'continue',
            toolName: 'WebSearch',
            message: 'Page loaded',
          }}
        />
      );
      expect(lastFrame()).toContain('WebSearch');

      // @step When the user presses Enter to resume
      // @step Then the TUI should show "Thinking..." again
      rerender(
        <InputTransition 
          {...defaultProps} 
          isLoading={true}
          isPaused={false}
        />
      );

      const output = lastFrame();
      expect(output).toContain('Thinking...');
      expect(output).not.toContain('WebSearch');
    });

    it('should transition to input state when agent completes', async () => {
      // @step Given a tool was paused and then resumed
      const { lastFrame, rerender } = render(
        <InputTransition 
          {...defaultProps} 
          isLoading={true}
          isPaused={false}
        />
      );
      expect(lastFrame()).toContain('Thinking...');

      // @step When the agent completes processing
      // @step Then the TUI should transition to the input state
      rerender(
        <InputTransition 
          {...defaultProps} 
          isLoading={false}
          isPaused={false}
        />
      );

      // Fast-forward through animation
      for (let i = 0; i < 100; i++) {
        vi.advanceTimersByTime(20);
        await vi.runAllTimersAsync();
      }

      const output = lastFrame();
      expect(output).toContain('Type a message...');
    });
  });

  // =============================================================================
  // Scenario: Confirm pause shows approval dialog
  // =============================================================================

  describe('Scenario: Confirm pause shows approval dialog (@future)', () => {
    it('should show confirm dialog with Y/N options', () => {
      // @step Given the agent is processing a request
      // @step And a tool requests a Confirm pause with message "Potentially dangerous command"
      // @step And the pause details contain "rm -rf /important/*"
      const { lastFrame } = render(
        <InputTransition 
          {...defaultProps} 
          isLoading={true}
          isPaused={true}
          pauseInfo={{
            kind: 'confirm',
            toolName: 'Bash',
            message: 'Potentially dangerous command',
            details: 'rm -rf /important/*',
          }}
        />
      );

      const output = lastFrame();

      // @step Then the session status should be "paused"
      // (Status is managed in Rust, verified by isPaused prop being true)

      // @step And the TUI should show a warning with the command text
      expect(output).toContain('Bash');
      expect(output).toContain('Potentially dangerous command');
      expect(output).toContain('rm -rf /important/*');

      // @step And the TUI should show "[Y] Approve [N] Deny [Esc] Cancel"
      expect(output).toContain('Y');
      expect(output).toContain('Approve');
      expect(output).toContain('N');
      expect(output).toContain('Deny');
    });
  });

  // =============================================================================
  // Pause indicator display tests
  // =============================================================================

  describe('Pause indicator display', () => {
    it('should display pause icon when paused', () => {
      const { lastFrame } = render(
        <InputTransition 
          {...defaultProps} 
          isLoading={true}
          isPaused={true}
          pauseInfo={{
            kind: 'continue',
            toolName: 'WebSearch',
            message: 'Page loaded',
          }}
        />
      );

      const output = lastFrame();
      // Should show some kind of pause indicator (⏸ or similar)
      expect(output).toMatch(/⏸|pause/i);
    });

    it('should not show pause indicator when not paused', () => {
      const { lastFrame } = render(
        <InputTransition 
          {...defaultProps} 
          isLoading={true}
          isPaused={false}
        />
      );

      const output = lastFrame();
      expect(output).not.toContain('⏸');
      expect(output).toContain('Thinking...');
    });
  });
});
