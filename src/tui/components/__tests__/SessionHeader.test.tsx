/**
 * Tests for SessionHeader component - Work unit display functionality
 *
 * SESS-001: Session header should display attached work unit
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render } from 'ink-testing-library';
import { SessionHeader } from '../SessionHeader';
import type { SessionHeaderProps } from '../SessionHeader';

// Mock terminalUtils
vi.mock('../../utils/terminalUtils', () => ({
  getTerminalWidth: vi.fn(() => 120),
}));

// Mock sessionHeaderUtils to avoid token calculation issues
vi.mock('../../utils/sessionHeaderUtils', () => ({
  getMaxTokens: vi.fn(() => ({ inputTokens: 1000, outputTokens: 500 })),
  getContextFillColor: vi.fn(() => 'green'),
  formatContextWindow: vi.fn(() => '200K'),
}));

describe('SessionHeader', () => {
  const defaultProps: SessionHeaderProps = {
    sessionId: 'test-session',
    modelId: 'claude-sonnet',
    hasReasoning: false,
    hasVision: false,
    contextWindow: 200000,
    tokenUsage: {
      inputTokens: 1000,
      outputTokens: 500,
      cacheReadInputTokens: 100,
      cacheCreationInputTokens: 50,
    },
    rustTokens: {
      inputTokens: 1000,
      outputTokens: 500,
      cacheReadInputTokens: 100,
      cacheCreationInputTokens: 50,
      cumulativeBilledInput: 1000,
      cumulativeBilledOutput: 500,
    },
    contextFillPercentage: 45.5,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('work unit display', () => {
    it('should display work unit ID when provided', () => {
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          workUnitId="STORY-001"
        />
      );

      const output = lastFrame();
      expect(output).toContain('(STORY-001)');
    });

    it('should not display work unit when not provided', () => {
      const { lastFrame } = render(
        <SessionHeader {...defaultProps} />
      );

      const output = lastFrame();
      expect(output).not.toContain('(STORY-');
    });

    it('should display both session number and work unit when both provided', () => {
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          sessionNumber={2}
          workUnitId="STORY-001"
        />
      );

      const output = lastFrame();
      expect(output).toContain('#2');
      expect(output).toContain('(STORY-001)');
    });

    it('should display session number without work unit when only session number provided', () => {
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          sessionNumber={3}
        />
      );

      const output = lastFrame();
      expect(output).toContain('#3');
      expect(output).not.toContain('(STORY-');
    });
  });

  describe('reasoning and vision badges', () => {
    it('should display reasoning badge when hasReasoning is true', () => {
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          hasReasoning={true}
          workUnitId="STORY-001"
        />
      );

      const output = lastFrame();
      expect(output).toContain('[R]');
    });

    it('should display vision badge when hasVision is true', () => {
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          hasVision={true}
          workUnitId="STORY-001"
        />
      );

      const output = lastFrame();
      expect(output).toContain('[V]');
    });

    it('should display both badges when both capabilities are true', () => {
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          hasReasoning={true}
          hasVision={true}
          workUnitId="STORY-001"
        />
      );

      const output = lastFrame();
      expect(output).toContain('[R]');
      expect(output).toContain('[V]');
    });
  });

  describe('work unit formatting edge cases', () => {
    it('should handle empty work unit ID gracefully', () => {
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          workUnitId=""
        />
      );

      const output = lastFrame();
      expect(output).not.toContain('()');
    });

    it('should handle work unit with special characters', () => {
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          workUnitId="STORY-001-PART-A"
        />
      );

      const output = lastFrame();
      expect(output).toContain('(STORY-001-PART-A)');
    });

    it('should handle work unit with numbers', () => {
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          workUnitId="BUG-123"
        />
      );

      const output = lastFrame();
      expect(output).toContain('(BUG-123)');
    });
  });

  describe('integration with session numbering', () => {
    it('should properly format with session 1 and work unit', () => {
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          sessionNumber={1}
          workUnitId="FEATURE-456"
        />
      );

      const output = lastFrame();
      expect(output).toContain('#1');
      expect(output).toContain('(FEATURE-456)');
    });

    it('should properly format with high session numbers and work unit', () => {
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          sessionNumber={15}
          workUnitId="TASK-789"
        />
      );

      const output = lastFrame();
      expect(output).toContain('#15');
      expect(output).toContain('(TASK-789)');
    });
  });

  describe('compaction percentage formatting', () => {
    it('should format context fill percentage with 2 decimal places', () => {
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          contextFillPercentage={45.678}
        />
      );

      const output = lastFrame();
      expect(output).toContain('[45.68%]');
    });

    it('should format compaction reduction with 2 decimal places and no double negative', () => {
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          contextFillPercentage={22.123}
          compactionReduction={35.567}
        />
      );

      const output = lastFrame();
      expect(output).toContain('[22.12%: COMPACTED 35.57%]');
    });

    it('should handle negative compaction reduction values correctly', () => {
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          contextFillPercentage={22.1}
          compactionReduction={-35.9}
        />
      );

      const output = lastFrame();
      expect(output).toContain('[22.1%: COMPACTED 35.9%]');
    });

    it('should format zero compaction reduction as natural zero', () => {
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          contextFillPercentage={45.0}
          compactionReduction={0}
        />
      );

      const output = lastFrame();
      expect(output).toContain('[45%: COMPACTED 0%]');
    });

    it('should remove trailing zeros naturally', () => {
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          contextFillPercentage={45.500}
          compactionReduction={35.000}
        />
      );

      const output = lastFrame();
      expect(output).toContain('[45.5%: COMPACTED 35%]');
    });

    it('should round percentage values properly', () => {
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          contextFillPercentage={45.995}
          compactionReduction={35.996}
        />
      );

      const output = lastFrame();
      // Note: JavaScript toFixed() rounds 45.995 to 45.99 due to floating point precision
      expect(output).toContain('[45.99%: COMPACTED 36%]');
    });
  });
});