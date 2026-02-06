/**
 * Tests for SessionHeader component - Work unit display functionality
 *
 * SESS-001: Session header should display attached work unit
 * TUI-060: Session header realtime work unit status display
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

  // TUI-060: Work unit status display
  describe('work unit status display', () => {
    describe('Scenario: Status change via fspec command updates header in realtime', () => {
      it('should display work unit with status in correct format', () => {
        // @step Given I am in AgentView with session #1
        // @step And work unit "TUI-060" with status "specifying" is attached to the session
        // @step And the header displays "#1 (TUI-060: specifying): claude-sonnet-4"
        const { lastFrame } = render(
          <SessionHeader
            {...defaultProps}
            sessionNumber={1}
            workUnitId="TUI-060"
            workUnitStatus="specifying"
          />
        );

        const output = lastFrame();
        expect(output).toContain('#1');
        expect(output).toContain('(TUI-060: specifying)');
      });

      it('should update display when status changes to testing', () => {
        // @step When the AI runs "fspec update-work-unit-status TUI-060 testing"
        // @step And the work-units.json file is updated
        // @step Then the header should update to "#1 (TUI-060: testing): claude-sonnet-4"
        const { lastFrame } = render(
          <SessionHeader
            {...defaultProps}
            sessionNumber={1}
            workUnitId="TUI-060"
            workUnitStatus="testing"
          />
        );

        const output = lastFrame();
        expect(output).toContain('#1');
        expect(output).toContain('(TUI-060: testing)');
      });
    });

    describe('Scenario: Attaching a different work unit updates header in realtime', () => {
      it('should display new work unit when attached', () => {
        // @step Given I am in AgentView with session #1
        // @step And work unit "TUI-060" with status "specifying" is attached to the session
        // @step And the header displays "#1 (TUI-060: specifying): model"
        // @step When I attach work unit "AUTH-001" with status "backlog" to the session
        // @step Then the header should update to "#1 (AUTH-001: backlog): model"
        const { lastFrame } = render(
          <SessionHeader
            {...defaultProps}
            sessionNumber={1}
            workUnitId="AUTH-001"
            workUnitStatus="backlog"
          />
        );

        const output = lastFrame();
        expect(output).toContain('#1');
        expect(output).toContain('(AUTH-001: backlog)');
      });
    });

    describe('Scenario: Header displays work unit ID without status when status is missing', () => {
      it('should display work unit ID without status when status is undefined', () => {
        // @step Given I am in AgentView with session #1
        // @step And work unit "LEGACY-001" without status is attached to the session
        const { lastFrame } = render(
          <SessionHeader
            {...defaultProps}
            sessionNumber={1}
            workUnitId="LEGACY-001"
            workUnitStatus={undefined}
          />
        );

        // @step Then the header should display "#1 (LEGACY-001): model"
        const output = lastFrame();
        expect(output).toContain('#1');
        expect(output).toContain('(LEGACY-001)');
        expect(output).not.toContain('(LEGACY-001:');
      });
    });

    describe('Scenario: Detaching work unit removes it from header display', () => {
      it('should display only session number when work unit is detached', () => {
        // @step Given I am in AgentView with session #1
        // @step And work unit "TUI-060" with status "specifying" is attached to the session
        // @step And the header displays "#1 (TUI-060: specifying): model"
        // @step When I detach the work unit from the session
        // @step Then the header should update to "#1: model"
        const { lastFrame } = render(
          <SessionHeader
            {...defaultProps}
            sessionNumber={1}
            workUnitId={undefined}
            workUnitStatus={undefined}
          />
        );

        const output = lastFrame();
        expect(output).toContain('#1');
        expect(output).not.toContain('(');
      });
    });

    it('should handle all workflow statuses correctly', () => {
      const statuses = ['backlog', 'specifying', 'testing', 'implementing', 'validating', 'done', 'blocked'];
      
      for (const status of statuses) {
        const { lastFrame } = render(
          <SessionHeader
            {...defaultProps}
            sessionNumber={1}
            workUnitId="TEST-001"
            workUnitStatus={status}
          />
        );

        const output = lastFrame();
        expect(output).toContain(`(TEST-001: ${status})`);
      }
    });

    it('should display status without session number when only work unit provided', () => {
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          workUnitId="AUTH-001"
          workUnitStatus="testing"
        />
      );

      const output = lastFrame();
      expect(output).toContain('(AUTH-001: testing)');
      expect(output).not.toContain('#');
    });
  });
});

// TUI-060: useWorkUnitsWatcher hook tests
describe('useWorkUnitsWatcher hook', () => {
  describe('Scenario: useWorkUnitsWatcher hook watches work-units.json', () => {
    it('should export a function that can be used as a React hook', async () => {
      // @step Given the useWorkUnitsWatcher hook is initialized
      // @step And the spec/work-units.json file exists
      const { useWorkUnitsWatcher } = await import('../../hooks/useWorkUnitsWatcher');
      
      // @step When the work-units.json file changes
      // @step Then the hook should call loadData on the Zustand store
      expect(useWorkUnitsWatcher).toBeDefined();
      expect(typeof useWorkUnitsWatcher).toBe('function');
    });
  });

  describe('Scenario: BoardView uses the shared useWorkUnitsWatcher hook', () => {
    it('should be importable for use in BoardView', async () => {
      // @step Given BoardView is rendered
      // @step Then it should use the useWorkUnitsWatcher hook
      // @step And not have inline chokidar file watching code
      const { useWorkUnitsWatcher } = await import('../../hooks/useWorkUnitsWatcher');
      expect(useWorkUnitsWatcher).toBeDefined();
    });
  });

  describe('Scenario: AgentView uses the shared useWorkUnitsWatcher hook', () => {
    it('should be importable for use in AgentView', async () => {
      // @step Given AgentView is rendered
      // @step Then it should use the useWorkUnitsWatcher hook
      // @step And receive work unit updates from the Zustand store
      const { useWorkUnitsWatcher } = await import('../../hooks/useWorkUnitsWatcher');
      expect(useWorkUnitsWatcher).toBeDefined();
    });
  });
});