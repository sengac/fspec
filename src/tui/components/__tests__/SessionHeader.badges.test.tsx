/**
 * Feature: spec/features/session-header-realtime-status.feature
 *
 * Badge display tests for SessionHeader component.
 * Tests: reasoning [R], vision [V] badge rendering.
 *
 * Architecture:
 * - Badges are prop-driven (hasReasoning, hasVision)
 * - Each badge has a distinct color (magenta, blue)
 */

import React from 'react';
import { describe, it, expect, vi } from 'vitest';
import { render } from 'ink-testing-library';
import { SessionHeader } from '../SessionHeader';
import type { SessionHeaderProps } from '../SessionHeader';

// Mock NAPI (required by sessionStore)
vi.mock('@sengac/codelet-napi', () => ({
  sessionSetActive: vi.fn(),
  sessionClearActive: vi.fn(),
  JsThinkingLevel: { Off: 0, Low: 1, Medium: 2, High: 3 },
}));

// Mock terminalUtils
vi.mock('../../utils/terminalUtils', () => ({
  getTerminalWidth: vi.fn(() => 120),
}));

// Mock sessionHeaderUtils to avoid token calculation issues
vi.mock('../../utils/sessionHeaderUtils', () => ({
  getMaxTokens: vi.fn(() => ({ inputTokens: 1000, outputTokens: 500, reasoningTokens: 0 })),
  getContextFillColor: vi.fn(() => 'green'),
  formatContextWindow: vi.fn(() => '200K'),
}));

const defaultProps: SessionHeaderProps = {
  modelId: 'claude-sonnet-4',
  hasReasoning: false,
  hasVision: false,
  contextWindow: 200000,
  tokenUsage: {
    inputTokens: 1000,
    outputTokens: 500,
  },
  rustTokens: {
    inputTokens: 1000,
    outputTokens: 500,
  },
  contextFillPercentage: 45.5,
};

describe('Feature: Session Header Badges', () => {
  describe('reasoning and vision badges', () => {
    it('should display [R] badge when hasReasoning is true', () => {
      // @step Given hasReasoning is true
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          hasReasoning={true}
        />
      );

      // @step Then the output should contain [R]
      const output = lastFrame();
      expect(output).toContain('[R]');
    });

    it('should NOT display [R] badge when hasReasoning is false', () => {
      // @step Given hasReasoning is false (default)
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          hasReasoning={false}
        />
      );

      // @step Then the output should NOT contain [R]
      const output = lastFrame();
      expect(output).not.toContain('[R]');
    });

    it('should display [V] badge when hasVision is true', () => {
      // @step Given hasVision is true
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          hasVision={true}
        />
      );

      // @step Then the output should contain [V]
      const output = lastFrame();
      expect(output).toContain('[V]');
    });

    it('should NOT display [V] badge when hasVision is false', () => {
      // @step Given hasVision is false (default)
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          hasVision={false}
        />
      );

      // @step Then the output should NOT contain [V]
      const output = lastFrame();
      expect(output).not.toContain('[V]');
    });

    it('should display both [R] and [V] badges when both are enabled', () => {
      // @step Given both hasReasoning and hasVision are true
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          hasReasoning={true}
          hasVision={true}
        />
      );

      // @step Then the output should contain both badges
      const output = lastFrame();
      expect(output).toContain('[R]');
      expect(output).toContain('[V]');
    });
  });
});
