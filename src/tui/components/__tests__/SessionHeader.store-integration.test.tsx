/**
 * Feature: spec/features/session-header-realtime-status.feature
 *
 * Store integration tests for SessionHeader component.
 * Replaces source-code-reading tests with proper behavioral tests
 * that verify SessionHeader reads work unit info from the Zustand store.
 *
 * Architecture:
 * - SessionHeader uses useCurrentWorkUnitId/useCurrentWorkUnitStatus hooks
 * - These hooks select from sessionStore (Zustand)
 * - Tests set store state, render the component, and verify output
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

describe('Feature: Session Header Zustand Store Integration', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useSessionStore.setState({
      currentWorkUnitId: null,
      currentWorkUnitStatus: null,
    });
  });

  // ----------------------------------------
  // Scenario: SessionHeader subscribes to sessionStore for work unit info
  // ----------------------------------------

  describe('Scenario: SessionHeader subscribes to sessionStore for work unit info', () => {
    it('should display work unit ID from store via useCurrentWorkUnitId', () => {
      // @step Given SessionHeader component is rendered
      // @step And sessionStore has currentWorkUnitId set to AUTH-001
      useSessionStore.setState({ currentWorkUnitId: 'AUTH-001' });

      const { lastFrame } = render(
        <SessionHeader {...defaultProps} sessionNumber={1} />
      );

      // @step Then it should use useCurrentWorkUnitId hook to get work unit ID
      const output = lastFrame();
      expect(output).toContain('AUTH-001');
    });

    it('should display work unit status from store via useCurrentWorkUnitStatus', () => {
      // @step Given sessionStore has work unit status set to implementing
      useSessionStore.setState({
        currentWorkUnitId: 'AUTH-001',
        currentWorkUnitStatus: 'implementing',
      });

      const { lastFrame } = render(
        <SessionHeader {...defaultProps} sessionNumber={1} />
      );

      // @step Then it should use useCurrentWorkUnitStatus hook to get status
      const output = lastFrame();
      expect(output).toContain('implementing');
    });

    it('should NOT display work unit info when store has no work unit', () => {
      // @step Given sessionStore has no currentWorkUnitId
      useSessionStore.setState({
        currentWorkUnitId: null,
        currentWorkUnitStatus: null,
      });

      const { lastFrame } = render(
        <SessionHeader {...defaultProps} sessionNumber={1} />
      );

      // @step Then the header should NOT contain work unit parentheses
      const output = lastFrame();
      // Should just show "#1: claude-sonnet-4" without parenthesized work unit
      expect(output).toContain('#1');
      expect(output).toContain('claude-sonnet-4');
      expect(output).not.toContain('(');
    });
  });

  // ----------------------------------------
  // Scenario: sessionStore provides currentWorkUnitId and currentWorkUnitStatus
  // ----------------------------------------

  describe('Scenario: sessionStore provides currentWorkUnitId and currentWorkUnitStatus', () => {
    it('should render work unit ID from sessionStore.currentWorkUnitId', () => {
      // @step Given sessionStore has currentWorkUnitId = INFRA-042
      useSessionStore.setState({ currentWorkUnitId: 'INFRA-042' });

      const { lastFrame } = render(
        <SessionHeader {...defaultProps} sessionNumber={3} />
      );

      // @step Then the header displays the work unit ID
      expect(lastFrame()).toContain('INFRA-042');
    });

    it('should render work unit status from sessionStore.currentWorkUnitStatus', () => {
      // @step Given sessionStore has status = testing
      useSessionStore.setState({
        currentWorkUnitId: 'INFRA-042',
        currentWorkUnitStatus: 'testing',
      });

      const { lastFrame } = render(
        <SessionHeader {...defaultProps} sessionNumber={3} />
      );

      // @step Then the header displays the status
      expect(lastFrame()).toContain('testing');
    });

    it('should display values set via setCurrentWorkUnit action', () => {
      // @step Given setCurrentWorkUnit is called with SEC-010 and validating
      useSessionStore.getState().setCurrentWorkUnit('SEC-010', 'validating');

      // @step When SessionHeader renders
      const { lastFrame } = render(
        <SessionHeader {...defaultProps} sessionNumber={1} />
      );

      // @step Then the header should display the work unit and status
      const output = lastFrame();
      expect(output).toContain('SEC-010');
      expect(output).toContain('validating');
    });

    it('should export useCurrentWorkUnitId hook (verified via rendered output)', () => {
      // @step Given sessionStore has a currentWorkUnitId
      useSessionStore.setState({ currentWorkUnitId: 'HOOK-TEST-001' });

      const { lastFrame } = render(
        <SessionHeader {...defaultProps} />
      );

      // @step Then the hook delivers the value to the rendered component
      expect(lastFrame()).toContain('HOOK-TEST-001');
    });

    it('should export useCurrentWorkUnitStatus hook (verified via rendered output)', () => {
      // @step Given sessionStore has a currentWorkUnitStatus
      useSessionStore.setState({
        currentWorkUnitId: 'HOOK-TEST-002',
        currentWorkUnitStatus: 'specifying',
      });

      const { lastFrame } = render(
        <SessionHeader {...defaultProps} />
      );

      // @step Then the hook delivers the status to the rendered component
      expect(lastFrame()).toContain('specifying');
    });
  });

  // ----------------------------------------
  // Scenario: AgentView syncs Rust snapshot to sessionStore
  // ----------------------------------------

  describe('Scenario: AgentView syncs Rust snapshot to sessionStore', () => {
    it('should display values when sessionStore is updated before render (simulating Rust sync)', () => {
      // @step Given AgentView syncs Rust state via setCurrentWorkUnit before render
      useSessionStore.getState().setCurrentWorkUnit('TUI-060', 'implementing');

      // @step When SessionHeader renders with session #1
      const { lastFrame } = render(
        <SessionHeader {...defaultProps} sessionNumber={1} />
      );

      // @step Then SessionHeader should display the synced values
      const output = lastFrame();
      expect(output).toContain('TUI-060');
      expect(output).toContain('implementing');
    });
  });

  // ----------------------------------------
  // Scenario: Opening AgentView shows work unit info from sessionStore
  // ----------------------------------------

  describe('Scenario: Opening AgentView shows work unit info from sessionStore', () => {
    it('should display work unit info immediately when store is pre-populated', () => {
      // @step Given sessionStore already has work unit TUI-060 with status implementing
      useSessionStore.setState({
        currentWorkUnitId: 'TUI-060',
        currentWorkUnitStatus: 'implementing',
      });

      // @step When AgentView renders SessionHeader (simulated by rendering component)
      const { lastFrame } = render(
        <SessionHeader {...defaultProps} sessionNumber={1} />
      );

      // @step Then the header should display "#1 (TUI-060: implementing): claude-sonnet-4"
      const output = lastFrame();
      expect(output).toContain('#1');
      expect(output).toContain('TUI-060');
      expect(output).toContain('implementing');
      expect(output).toContain('claude-sonnet-4');
    });
  });

  // ----------------------------------------
  // Header format: '#N (WORK-ID: status): model'
  // ----------------------------------------

  describe('header format composition', () => {
    it('should format as "#N (WORK-ID: status): model"', () => {
      // @step Given a full set of session state
      useSessionStore.setState({
        currentWorkUnitId: 'AUTH-001',
        currentWorkUnitStatus: 'implementing',
      });

      const { lastFrame } = render(
        <SessionHeader {...defaultProps} sessionNumber={2} />
      );

      // @step Then the format should match the expected pattern
      const output = lastFrame();
      expect(output).toContain('#2');
      expect(output).toContain('AUTH-001');
      expect(output).toContain('implementing');
      expect(output).toContain('claude-sonnet-4');
    });

    it('should format as "#N: model" when no work unit is attached', () => {
      // @step Given no work unit in the store
      const { lastFrame } = render(
        <SessionHeader {...defaultProps} sessionNumber={5} />
      );

      // @step Then just session number and model should show
      const output = lastFrame();
      expect(output).toContain('#5');
      expect(output).toContain('claude-sonnet-4');
      expect(output).not.toContain('(');
    });
  });
});
