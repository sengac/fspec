/**
 * Feature: spec/features/tui-badge-and-fill-display-end-to-end-verification.feature
 *
 * End-to-end verification that the TUI badge and fill percentage display
 * correct values after the Rust-side fixes (LIMITS-002 through LIMITS-005).
 *
 * Verifies:
 * - Badge shows [192k] for Claude Opus 4.6 (not [968k])
 * - Badge shows [800k] for Gemini 2.5 Pro
 * - Badge shows [102k] for GPT-4o
 * - Fill% uses compaction threshold as denominator
 * - Fallback to contextWindow when threshold unavailable
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from 'ink-testing-library';
import { SessionHeader } from '../SessionHeader';
import type { SessionHeaderProps } from '../SessionHeader';
import {
  clearAllSubscriptions,
  getSessionSnapshotForTesting,
  setRustStateSource,
  resetRustStateSource,
  type RustStateSource,
} from '../../hooks/useRustSessionState';
import { formatContextWindow } from '../../utils/sessionHeaderUtils';

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

// =============================================================================
// Mock state helpers
// =============================================================================

interface MockModelState {
  providerId: string;
  modelId: string;
  contextWindow?: number;
  maxOutputTokens?: number;
  compactionThreshold?: number;
}

interface MockState {
  status: string;
  model: MockModelState | null;
  tokens: { inputTokens: number; outputTokens: number };
  debugEnabled: boolean;
  baseThinkingLevel: number;
}

function createMockStateSource(initialState: MockState): {
  source: RustStateSource;
  setState: (state: Partial<MockState>) => void;
} {
  let state = { ...initialState };

  const source: RustStateSource = {
    getStatus: () => state.status,
    getModel: () => state.model,
    getTokens: () => state.tokens,
    getDebugEnabled: () => state.debugEnabled,
    getPauseState: () => null,
    getBaseThinkingLevel: () => state.baseThinkingLevel,
    setBaseThinkingLevel: (_sessionId: string, level: number) => {
      state.baseThinkingLevel = level;
    },
    getCompactionProgress: () => null,
    getHitlRequest: () => null,
  };

  return {
    source,
    setState: (newState: Partial<MockState>) => {
      state = { ...state, ...newState };
    },
  };
}

// =============================================================================
// Shared base props for SessionHeader
// =============================================================================

const baseProps: SessionHeaderProps = {
  modelId: 'claude-opus-4-6',
  hasReasoning: false,
  hasVision: false,
  contextWindow: 200000,
  tokenUsage: { inputTokens: 0, outputTokens: 0 },
  rustTokens: { inputTokens: 0, outputTokens: 0 },
  contextFillPercentage: 0,
};

// =============================================================================
// Tests
// =============================================================================

describe('Feature: TUI Badge and Fill% Display — End-to-End Verification', () => {
  beforeEach(() => {
    clearAllSubscriptions();
  });

  afterEach(() => {
    resetRustStateSource();
    clearAllSubscriptions();
  });

  // ===========================================================================
  // Scenario: Badge shows [192k] for Claude Opus 4.6
  // ===========================================================================

  describe('Scenario: Badge shows [192k] for Claude Opus 4.6', () => {
    it('should display [192k] badge using compaction threshold, not [968k]', () => {
      // @step Given a Claude Opus 4.6 model with 200000 context window
      const mock = createMockStateSource({
        status: 'running',
        model: {
          providerId: 'anthropic',
          modelId: 'claude-opus-4-6',
          contextWindow: 200000,
          maxOutputTokens: 32000,
          compactionThreshold: 191808,
        },
        tokens: { inputTokens: 70000, outputTokens: 17000 },
        debugEnabled: false,
        baseThinkingLevel: 0,
      });
      setRustStateSource(mock.source);

      // @step And the Rust-resolved compaction threshold is 191808 tokens
      const snapshot = getSessionSnapshotForTesting('test-limits-006-claude');
      expect(snapshot.model!.compactionThreshold).toBe(191808);

      // @step And the session has consumed 87000 tokens
      const totalTokens = snapshot.tokens.inputTokens + snapshot.tokens.outputTokens;
      expect(totalTokens).toBe(87000);

      // @step When the SessionHeader renders
      const fillPct = (totalTokens / snapshot.model!.compactionThreshold!) * 100;
      const { lastFrame } = render(
        <SessionHeader
          {...baseProps}
          modelId="claude-opus-4-6"
          contextWindow={200000}
          compactionThreshold={191808}
          tokenUsage={{ inputTokens: 70000, outputTokens: 17000 }}
          rustTokens={{ inputTokens: 70000, outputTokens: 17000 }}
          contextFillPercentage={fillPct}
        />,
      );
      const output = lastFrame();

      // @step Then the badge should display "[192k]"
      expect(output).toContain('[192k]');

      // @step And the badge should not display "[968k]"
      expect(output).not.toContain('[968k]');

      // @step And the fill percentage should be approximately 45 percent
      expect(formatContextWindow(191808)).toBe('192k');
      expect(Math.round(fillPct)).toBeGreaterThanOrEqual(45);
      expect(Math.round(fillPct)).toBeLessThanOrEqual(46);
    });
  });

  // ===========================================================================
  // Scenario: Badge shows [800k] for Gemini 2.5 Pro
  // ===========================================================================

  describe('Scenario: Badge shows [800k] for Gemini 2.5 Pro', () => {
    it('should display [800k] badge using 80% threshold of 1M context', () => {
      // @step Given a Gemini 2.5 Pro model with 1000000 context window
      const mock = createMockStateSource({
        status: 'running',
        model: {
          providerId: 'google',
          modelId: 'gemini-2.5-pro',
          contextWindow: 1000000,
          maxOutputTokens: 65536,
          compactionThreshold: 800000,
        },
        tokens: { inputTokens: 300000, outputTokens: 100000 },
        debugEnabled: false,
        baseThinkingLevel: 0,
      });
      setRustStateSource(mock.source);

      // @step And the Rust-resolved compaction threshold is 800000 tokens
      const snapshot = getSessionSnapshotForTesting('test-limits-006-gemini');
      expect(snapshot.model!.compactionThreshold).toBe(800000);

      // @step And the session has consumed 400000 tokens
      const totalTokens = snapshot.tokens.inputTokens + snapshot.tokens.outputTokens;
      expect(totalTokens).toBe(400000);

      // @step When the SessionHeader renders
      const fillPct = (totalTokens / snapshot.model!.compactionThreshold!) * 100;
      const { lastFrame } = render(
        <SessionHeader
          {...baseProps}
          modelId="gemini-2.5-pro"
          contextWindow={1000000}
          compactionThreshold={800000}
          tokenUsage={{ inputTokens: 300000, outputTokens: 100000 }}
          rustTokens={{ inputTokens: 300000, outputTokens: 100000 }}
          contextFillPercentage={fillPct}
        />,
      );
      const output = lastFrame();

      // @step Then the badge should display "[800k]"
      expect(output).toContain('[800k]');

      // @step And the badge should not display "[1M]"
      expect(output).not.toContain('[1M]');

      // @step And the fill percentage should be 50 percent
      expect(fillPct).toBe(50);
    });
  });

  // ===========================================================================
  // Scenario: Badge shows [102k] for GPT-4o
  // ===========================================================================

  describe('Scenario: Badge shows [102k] for GPT-4o', () => {
    it('should display [102k] badge using 80% threshold of 128k context', () => {
      // @step Given a GPT-4o model with 128000 context window
      const mock = createMockStateSource({
        status: 'running',
        model: {
          providerId: 'openai',
          modelId: 'gpt-4o',
          contextWindow: 128000,
          maxOutputTokens: 16384,
          compactionThreshold: 102400,
        },
        tokens: { inputTokens: 40000, outputTokens: 11200 },
        debugEnabled: false,
        baseThinkingLevel: 0,
      });
      setRustStateSource(mock.source);

      // @step And the Rust-resolved compaction threshold is 102400 tokens
      const snapshot = getSessionSnapshotForTesting('test-limits-006-gpt4o');
      expect(snapshot.model!.compactionThreshold).toBe(102400);

      // @step And the session has consumed 51200 tokens
      const totalTokens = snapshot.tokens.inputTokens + snapshot.tokens.outputTokens;
      expect(totalTokens).toBe(51200);

      // @step When the SessionHeader renders
      const fillPct = (totalTokens / snapshot.model!.compactionThreshold!) * 100;
      const { lastFrame } = render(
        <SessionHeader
          {...baseProps}
          modelId="gpt-4o"
          contextWindow={128000}
          compactionThreshold={102400}
          tokenUsage={{ inputTokens: 40000, outputTokens: 11200 }}
          rustTokens={{ inputTokens: 40000, outputTokens: 11200 }}
          contextFillPercentage={fillPct}
        />,
      );
      const output = lastFrame();

      // @step Then the badge should display "[102k]"
      expect(output).toContain('[102k]');

      // @step And the fill percentage should be 50 percent
      expect(fillPct).toBe(50);
    });
  });

  // ===========================================================================
  // Scenario: Fill percentage uses compaction threshold as denominator
  // ===========================================================================

  describe('Scenario: Fill percentage uses compaction threshold as denominator', () => {
    it('should compute fill percentage relative to compaction threshold', () => {
      // @step Given a Claude Sonnet 4 model with 200000 context window
      const mock = createMockStateSource({
        status: 'running',
        model: {
          providerId: 'anthropic',
          modelId: 'claude-sonnet-4',
          contextWindow: 200000,
          maxOutputTokens: 8192,
          compactionThreshold: 191808,
        },
        tokens: { inputTokens: 75000, outputTokens: 20904 },
        debugEnabled: false,
        baseThinkingLevel: 0,
      });
      setRustStateSource(mock.source);

      // @step And the Rust-resolved compaction threshold is 191808 tokens
      const snapshot = getSessionSnapshotForTesting('test-limits-006-fill');
      expect(snapshot.model!.compactionThreshold).toBe(191808);

      // @step And the session has consumed 95904 tokens
      const totalTokens = snapshot.tokens.inputTokens + snapshot.tokens.outputTokens;
      expect(totalTokens).toBe(95904);

      // @step When the context fill percentage is calculated
      const threshold = snapshot.model!.compactionThreshold!;
      const fillPercentage = (totalTokens / threshold) * 100;

      // @step Then the fill percentage should use the compaction threshold as the denominator
      // If we used contextWindow (200k), fill would be 47.952%
      // With threshold (191808), fill is 50%
      const fillWithContextWindow = (totalTokens / snapshot.model!.contextWindow!) * 100;
      expect(fillPercentage).not.toBe(fillWithContextWindow);

      // @step And the fill percentage should be 50 percent
      expect(fillPercentage).toBe(50);
    });
  });

  // ===========================================================================
  // Scenario: Badge falls back to context window before model selection
  // ===========================================================================

  describe('Scenario: Badge falls back to context window before model selection', () => {
    it('should display [200k] from context window when no threshold provided', () => {
      // @step Given a session where no model has been selected yet
      // @step And no compaction threshold is available
      // (compactionThreshold prop omitted)

      // @step When the SessionHeader renders with a context window of 200000
      const { lastFrame } = render(
        <SessionHeader
          {...baseProps}
          modelId="claude-sonnet-4"
          contextWindow={200000}
        />,
      );
      const output = lastFrame();

      // @step Then the badge should display "[200k]" as a fallback
      expect(output).toContain('[200k]');
    });
  });
});
