/**
 * Tests for ThinkingIndicator component
 *
 * Verifies the animated thinking indicator displays correctly
 * and handles different spinner types and states.
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from 'ink-testing-library';
import { ThinkingIndicator } from '../ThinkingIndicator';

describe('ThinkingIndicator', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe('basic rendering', () => {
    it('should render the default message with spinner', () => {
      const { lastFrame } = render(<ThinkingIndicator />);
      const output = lastFrame();

      // Should contain "Thinking..." and "(Esc to stop)"
      expect(output).toContain('Thinking...');
      expect(output).toContain('(Esc to stop)');
    });

    it('should render a custom message', () => {
      const { lastFrame } = render(<ThinkingIndicator message="Processing" />);
      const output = lastFrame();

      expect(output).toContain('Processing...');
    });

    it('should render a custom hint', () => {
      const { lastFrame } = render(
        <ThinkingIndicator hint="(press any key)" />
      );
      const output = lastFrame();

      expect(output).toContain('(press any key)');
    });

    it('should render nothing when isActive is false', () => {
      const { lastFrame } = render(<ThinkingIndicator isActive={false} />);
      const output = lastFrame();

      expect(output).toBe('');
    });
  });

  describe('animation', () => {
    it('should render with a spinner character from the dots set', () => {
      const { lastFrame } = render(<ThinkingIndicator type="dots" />);

      // First frame should contain the first dots spinner character
      const output = lastFrame();
      // The dots spinner uses braille characters: ⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏
      expect(output).toMatch(/[⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏]/);
    });

    it('should use ellipsis spinner type', () => {
      const { lastFrame } = render(<ThinkingIndicator type="ellipsis" />);

      const output = lastFrame();
      // Ellipsis starts with ". " (dot followed by spaces)
      expect(output).toMatch(/\. {2}/);
    });

    it('should use line spinner type', () => {
      const { lastFrame } = render(<ThinkingIndicator type="line" />);

      const output = lastFrame();
      // Line spinner starts with "-"
      expect(output).toContain('-');
    });
  });

  describe('props', () => {
    it('should accept all spinner types', () => {
      const spinnerTypes = [
        'dots',
        'ellipsis',
        'bounce',
        'line',
        'arc',
        'growVertical',
      ] as const;

      spinnerTypes.forEach((type) => {
        const { lastFrame } = render(<ThinkingIndicator type={type} />);
        const output = lastFrame();
        // Should render without throwing
        expect(output).toBeTruthy();
      });
    });
  });
});
