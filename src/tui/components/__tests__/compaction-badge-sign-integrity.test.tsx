/**
 * Feature: spec/features/compaction-badge-sign-integrity-ink.feature
 * CMPCT-040: COMPACTED badge sign-integrity (TS Ink twin).
 *
 * The header badge must never sign-flip a negative compaction reduction
 * into a fake positive percentage:
 * - SessionHeader renders compactionReduction verbatim (no Math.abs).
 * - The two raw sessionCompact writers in AgentView.tsx clamp with
 *   Math.max(0, result.compressionRatio) so the component invariant
 *   (compactionReduction >= 0 when non-null) holds even against a
 *   stale/unclamped backend.
 *
 * Rendering tests follow SessionHeader.rendering.test.tsx; the writer
 * clamp is pinned via source-shape assertions following the
 * NAPI-010-stream-chunk-discriminated-union.test.tsx precedent (the
 * AgentView write sites are inline in a component too large to render
 * in isolation).
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render } from 'ink-testing-library';
import * as fs from 'fs';
import * as path from 'path';
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

describe('Feature: Compaction badge sign-integrity (CMPCT-040)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useSessionStore.setState({
      currentWorkUnitId: null,
      currentWorkUnitStatus: null,
    });
  });

  describe('Scenario: TS SessionHeader renders a negative compactionReduction honestly', () => {
    it('should never sign-flip a negative reduction into a fake positive badge', () => {
      // @step Given the Ink SessionHeader receives contextFillPercentage 22.123 and compactionReduction -35
      const props: SessionHeaderProps = {
        ...defaultProps,
        contextFillPercentage: 22.123,
        compactionReduction: -35,
      };

      // @step When the header renders
      const { lastFrame } = render(<SessionHeader {...props} />);
      const output = lastFrame();

      // @step Then the output does NOT contain "COMPACTED 35%"
      expect(output).not.toContain('COMPACTED 35%');

      // @step And the output contains "COMPACTED -35%"
      expect(output).toContain('COMPACTED -35%');
    });
  });

  describe('Scenario: TS raw sessionCompact writers clamp a negative compressionRatio at both write sites', () => {
    // Source-shape assertions (NAPI-010 precedent): the two sessionCompact
    // write sites are inline in AgentView.tsx — a component too large to
    // render in isolation — so the negative-compressionRatio behaviour is
    // pinned by asserting the shape of the write-site source.
    // @step Given a sessionCompact RPC result whose compressionRatio is negative
    const agentViewSource = fs.readFileSync(
      path.join(process.cwd(), 'src/tui/components/AgentView.tsx'),
      'utf-8'
    );
    const sessionHeaderSource = fs.readFileSync(
      path.join(process.cwd(), 'src/tui/components/SessionHeader.tsx'),
      'utf-8'
    );

    it('should clamp the sessionCompact result at both raw write sites', () => {
      // @step When the handler stores the result
      const clampedWrites = agentViewSource.match(
        /setCompactionReduction\(\s*Math\.max\(0,\s*result\.compressionRatio\)\s*\)/g
      );

      // @step Then the manual /compact write site stores a compactionReduction clamped to a minimum of 0
      // @step And the retry dialog write site stores a compactionReduction clamped to a minimum of 0
      expect(
        clampedWrites?.length ?? 0,
        'both raw sessionCompact write sites (manual /compact handler and ' +
          'retry dialog) must clamp with Math.max(0, result.compressionRatio)'
      ).toBe(2);

      // @step And no write site stores the raw unclamped compressionRatio
      expect(agentViewSource).not.toMatch(
        /setCompactionReduction\(\s*result\.compressionRatio\s*\)/
      );
    });

    it('should not apply Math.abs to compactionReduction in SessionHeader', () => {
      // @step And the SessionHeader never applies an absolute value to compactionReduction
      expect(sessionHeaderSource).not.toMatch(
        /Math\.abs\(\s*compactionReduction\s*\)/
      );
    });
  });

  describe('Scenario: TS positive compactionReduction still renders unchanged without Math.abs', () => {
    it('should keep the existing positive formatting untouched', () => {
      // @step Given the Ink SessionHeader receives contextFillPercentage 22.123 and compactionReduction 35.567
      const props: SessionHeaderProps = {
        ...defaultProps,
        contextFillPercentage: 22.123,
        compactionReduction: 35.567,
      };

      // @step When the header renders
      const { lastFrame } = render(<SessionHeader {...props} />);
      const output = lastFrame();

      // @step Then the output contains "[22.12%: COMPACTED 35.57%]"
      expect(output).toContain('[22.12%: COMPACTED 35.57%]');
    });
  });
});
