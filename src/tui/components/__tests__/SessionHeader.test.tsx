/**
 * Tests for SessionHeader component - Work unit display functionality
 *
 * SESS-001: Session header should display attached work unit
 */

import React from 'react';
import { describe, it, expect, vi } from 'vitest';
import { render } from '@testing-library/react';
import type { SessionHeaderProps } from '../SessionHeader';

// Mock Ink components
vi.mock('ink', () => ({
  Box: ({ children, ...props }: any) => <div {...props}>{children}</div>,
  Text: ({ children, color, bold, dimColor }: any) => (
    <span data-color={color} data-bold={bold} data-dim={dimColor}>{children}</span>
  ),
}));

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
    it('should display work unit ID when provided', async () => {
      // Dynamically import to ensure mocks are applied
      const { SessionHeader } = await import('../SessionHeader');
      
      const { container } = render(
        <SessionHeader
          {...defaultProps}
          workUnitId="STORY-001"
        />
      );

      // Look for the agent label with work unit
      const agentText = container.textContent;
      expect(agentText).toContain('(STORY-001)');
    });

    it('should not display work unit when not provided', async () => {
      const { SessionHeader } = await import('../SessionHeader');
      
      const { container } = render(
        <SessionHeader {...defaultProps} />
      );

      const agentText = container.textContent;
      expect(agentText).not.toContain('(STORY-');
    });

    it('should display both session number and work unit when both provided', async () => {
      const { SessionHeader } = await import('../SessionHeader');
      
      const { container } = render(
        <SessionHeader
          {...defaultProps}
          sessionNumber={2}
          workUnitId="STORY-001"
        />
      );

      const agentText = container.textContent;
      expect(agentText).toContain('#2');
      expect(agentText).toContain('(STORY-001)');
    });

    it('should display session number without work unit when only session number provided', async () => {
      const { SessionHeader } = await import('../SessionHeader');
      
      const { container } = render(
        <SessionHeader
          {...defaultProps}
          sessionNumber={3}
        />
      );

      const agentText = container.textContent;
      expect(agentText).toContain('#3');
      expect(agentText).not.toContain('(STORY-');
    });
  });

  describe('reasoning and vision badges', () => {
    it('should display reasoning badge when hasReasoning is true', async () => {
      const { SessionHeader } = await import('../SessionHeader');
      
      const { container } = render(
        <SessionHeader
          {...defaultProps}
          hasReasoning={true}
          workUnitId="STORY-001"
        />
      );

      expect(container.textContent).toContain('[R]');
    });

    it('should display vision badge when hasVision is true', async () => {
      const { SessionHeader } = await import('../SessionHeader');
      
      const { container } = render(
        <SessionHeader
          {...defaultProps}
          hasVision={true}
          workUnitId="STORY-001"
        />
      );

      expect(container.textContent).toContain('[V]');
    });

    it('should display both badges when both capabilities are true', async () => {
      const { SessionHeader } = await import('../SessionHeader');
      
      const { container } = render(
        <SessionHeader
          {...defaultProps}
          hasReasoning={true}
          hasVision={true}
          workUnitId="STORY-001"
        />
      );

      expect(container.textContent).toContain('[R]');
      expect(container.textContent).toContain('[V]');
    });
  });

  describe('work unit formatting edge cases', () => {
    it('should handle empty work unit ID gracefully', async () => {
      const { SessionHeader } = await import('../SessionHeader');
      
      const { container } = render(
        <SessionHeader
          {...defaultProps}
          workUnitId=""
        />
      );

      const agentText = container.textContent;
      expect(agentText).not.toContain('()');
    });

    it('should handle work unit with special characters', async () => {
      const { SessionHeader } = await import('../SessionHeader');
      
      const { container } = render(
        <SessionHeader
          {...defaultProps}
          workUnitId="STORY-001-PART-A"
        />
      );

      const agentText = container.textContent;
      expect(agentText).toContain('(STORY-001-PART-A)');
    });

    it('should handle work unit with numbers', async () => {
      const { SessionHeader } = await import('../SessionHeader');
      
      const { container } = render(
        <SessionHeader
          {...defaultProps}
          workUnitId="BUG-123"
        />
      );

      const agentText = container.textContent;
      expect(agentText).toContain('(BUG-123)');
    });
  });

  describe('integration with session numbering', () => {
    it('should properly format with session 1 and work unit', async () => {
      const { SessionHeader } = await import('../SessionHeader');
      
      const { container } = render(
        <SessionHeader
          {...defaultProps}
          sessionNumber={1}
          workUnitId="FEATURE-456"
        />
      );

      const agentText = container.textContent;
      expect(agentText).toContain('#1');
      expect(agentText).toContain('(FEATURE-456)');
    });

    it('should properly format with high session numbers and work unit', async () => {
      const { SessionHeader } = await import('../SessionHeader');
      
      const { container } = render(
        <SessionHeader
          {...defaultProps}
          sessionNumber={15}
          workUnitId="TASK-789"
        />
      );

      const agentText = container.textContent;
      expect(agentText).toContain('#15');
      expect(agentText).toContain('(TASK-789)');
    });
  });
});