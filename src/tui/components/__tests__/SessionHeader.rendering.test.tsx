/**
 * Feature: spec/features/session-header-realtime-status.feature
 *
 * Behavioral render tests for SessionHeader component.
 * Tests: status display, compaction formatting, ISOLATED badge.
 *
 * Architecture:
 * - SessionHeader reads work unit info from Zustand sessionStore (not props)
 * - These tests verify rendered output for various prop combinations
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render } from 'ink-testing-library';
import { useSessionStore } from '../../store/sessionStore';
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

describe('Feature: Session Header Work Unit Status Display', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useSessionStore.setState({
      currentWorkUnitId: null,
      currentWorkUnitStatus: null,
    });
  });

  // ----------------------------------------
  // Integration: Status Change Updates Header
  // ----------------------------------------

  describe('Scenario: Status change via fspec command updates header in realtime', () => {
    it('should display session number and model from rendered output', () => {
      // @step Given I am in AgentView with session #1
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          sessionNumber={1}
        />
      );

      // @step Then the header should contain the session number and model
      const output = lastFrame();
      expect(output).toContain('#1');
      expect(output).toContain('claude-sonnet-4');
    });

    it('should display work unit ID and status from Zustand store', () => {
      // @step Given sessionStore has work unit TUI-060 with status specifying
      useSessionStore.setState({
        currentWorkUnitId: 'TUI-060',
        currentWorkUnitStatus: 'specifying',
      });

      // @step When SessionHeader renders with session #1
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          sessionNumber={1}
        />
      );

      // @step Then the header should display "#1 (TUI-060: specifying): claude-sonnet-4"
      const output = lastFrame();
      expect(output).toContain('#1');
      expect(output).toContain('TUI-060');
      expect(output).toContain('specifying');
      expect(output).toContain('claude-sonnet-4');
    });
  });

  // ----------------------------------------
  // Header Without Status
  // ----------------------------------------

  describe('Scenario: Header displays work unit ID without status when status is missing', () => {
    it('should handle undefined status gracefully', () => {
      // @step Given Rust provides workUnitId LEGACY-001 but workUnitStatus is undefined
      useSessionStore.setState({
        currentWorkUnitId: 'LEGACY-001',
        currentWorkUnitStatus: null,
      });

      // @step When SessionHeader renders with session #1
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          sessionNumber={1}
        />
      );

      // @step Then the header should display the work unit ID without status
      const output = lastFrame();
      expect(output).toContain('#1');
      expect(output).toContain('LEGACY-001');
      // Should NOT contain a colon-separated status
      expect(output).not.toContain('LEGACY-001:');
    });
  });

  // ----------------------------------------
  // Compaction Percentage Formatting
  // ----------------------------------------

  describe('compaction percentage formatting', () => {
    it('should format context fill percentage with up to 2 decimal places', () => {
      // @step Given contextFillPercentage is 45.678
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          contextFillPercentage={45.678}
        />
      );

      // @step Then the output should contain the formatted percentage
      const output = lastFrame();
      expect(output).toContain('[45.68%]');
    });

    it('should format compaction reduction with up to 2 decimal places', () => {
      // @step Given contextFillPercentage is 22.123 and compactionReduction is 35.567
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          contextFillPercentage={22.123}
          compactionReduction={35.567}
        />
      );

      // @step Then the output should contain COMPACTED text with formatted values
      const output = lastFrame();
      expect(output).toContain('[22.12%: COMPACTED 35.57%]');
    });
  });

  // ----------------------------------------
  // GIT-029: Isolation State Badge
  // ----------------------------------------

  describe('Scenario: SessionHeader displays ISOLATED badge for isolated session', () => {
    it('should display [ISOLATED] badge when isIsolated is true', () => {
      // @step Given I have created an isolated session
      // @step When the SessionHeader renders
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          isIsolated={true}
        />
      );

      // @step Then I should see an "[ISOLATED]" badge
      const output = lastFrame();
      expect(output).toContain('[ISOLATED]');
    });
  });

  describe('Scenario: SessionHeader does not display ISOLATED badge for normal session', () => {
    it('should not display [ISOLATED] badge when isIsolated is false', () => {
      // @step Given I have created a normal (non-isolated) session
      // @step When the SessionHeader renders
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
          isIsolated={false}
        />
      );

      // @step Then I should not see an "[ISOLATED]" badge
      const output = lastFrame();
      expect(output).not.toContain('[ISOLATED]');
    });

    it('should not display [ISOLATED] badge when isIsolated is undefined (default)', () => {
      // @step Given I have created a normal session without isIsolated prop
      // @step When the SessionHeader renders
      const { lastFrame } = render(
        <SessionHeader
          {...defaultProps}
        />
      );

      // @step Then I should not see an "[ISOLATED]" badge
      const output = lastFrame();
      expect(output).not.toContain('[ISOLATED]');
    });
  });
});
