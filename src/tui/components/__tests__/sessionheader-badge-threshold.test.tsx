/**
 * Feature: spec/features/sessionheader-badge-threshold.feature
 *
 * This test file validates that the SessionHeader badge displays the compaction
 * threshold rather than the raw context window, ensuring the badge and fill
 * percentage agree on the same denominator.
 *
 * Architecture:
 * - SessionHeader accepts compactionThreshold prop (optional)
 * - Badge displays compactionThreshold when available, falls back to contextWindow
 * - formatContextWindow formats any token count (unchanged)
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

// We do NOT mock sessionHeaderUtils here — we want the real formatContextWindow
// so we can verify the badge shows the correct formatted threshold value.

const baseProps: SessionHeaderProps = {
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
  contextFillPercentage: 45,
};

describe('Feature: SessionHeader badge shows compaction threshold', () => {
  describe('Scenario: Badge shows compaction threshold for Claude model', () => {
    it('should display [192k] badge using compaction threshold instead of [200k]', () => {
      // @step Given a Claude Sonnet 4 model with 200000 token context window
      const props: SessionHeaderProps = {
        ...baseProps,
        modelId: 'claude-sonnet-4',
        contextWindow: 200000,
      };

      // @step And the Rust-resolved compaction threshold is 191808
      const propsWithThreshold = {
        ...props,
        compactionThreshold: 191808,
      };

      // @step When the SessionHeader renders with both values
      const { lastFrame } = render(
        <SessionHeader {...propsWithThreshold} />
      );
      const output = lastFrame();

      // @step Then the badge should display "[192k]" using the compaction threshold
      expect(output).toContain('[192k]');

      // @step And the badge should not display "[200k]" from the raw context window
      expect(output).not.toContain('[200k]');
    });
  });

  describe('Scenario: Badge shows compaction threshold for large-context model', () => {
    it('should display [800k] badge using compaction threshold instead of [1M]', () => {
      // @step Given a Gemini 2.5 Pro model with 1000000 token context window
      const props: SessionHeaderProps = {
        ...baseProps,
        modelId: 'gemini-2.5-pro',
        contextWindow: 1000000,
      };

      // @step And the Rust-resolved compaction threshold is 800000
      const propsWithThreshold = {
        ...props,
        compactionThreshold: 800000,
      };

      // @step When the SessionHeader renders with both values
      const { lastFrame } = render(
        <SessionHeader {...propsWithThreshold} />
      );
      const output = lastFrame();

      // @step Then the badge should display "[800k]" using the compaction threshold
      expect(output).toContain('[800k]');

      // @step And the badge should not display "[1M]" from the raw context window
      expect(output).not.toContain('[1M]');
    });
  });

  describe('Scenario: Badge shows user-configured custom threshold', () => {
    it('should display [150k] badge using the user-configured threshold', () => {
      // @step Given a custom model with 200000 token context window
      const props: SessionHeaderProps = {
        ...baseProps,
        modelId: 'custom-model',
        contextWindow: 200000,
      };

      // @step And the user has configured a compaction threshold of 150000
      const propsWithThreshold = {
        ...props,
        compactionThreshold: 150000,
      };

      // @step When the SessionHeader renders with both values
      const { lastFrame } = render(
        <SessionHeader {...propsWithThreshold} />
      );
      const output = lastFrame();

      // @step Then the badge should display "[150k]" using the custom threshold
      expect(output).toContain('[150k]');

      // @step And the badge should not display "[200k]" from the raw context window
      expect(output).not.toContain('[200k]');
    });
  });

  describe('Scenario: Badge falls back to context window when threshold is unavailable', () => {
    it('should display [200k] from context window when no threshold provided', () => {
      // @step Given a model with 200000 token context window
      const props: SessionHeaderProps = {
        ...baseProps,
        modelId: 'claude-sonnet-4',
        contextWindow: 200000,
      };

      // @step And no compaction threshold is available yet
      // (compactionThreshold prop omitted)

      // @step When the SessionHeader renders without a compaction threshold
      const { lastFrame } = render(
        <SessionHeader {...props} />
      );
      const output = lastFrame();

      // @step Then the badge should display "[200k]" from the context window as fallback
      expect(output).toContain('[200k]');
    });
  });

  describe('Scenario: Badge shows threshold after session resume', () => {
    it('should display [192k] badge using the restored compaction threshold', () => {
      // @step Given a resumed session with Rust state containing compaction threshold 191808
      const restoredThreshold = 191808;

      // @step And the context window from model data is 200000
      const props: SessionHeaderProps = {
        ...baseProps,
        modelId: 'claude-sonnet-4',
        contextWindow: 200000,
        compactionThreshold: restoredThreshold,
      };

      // @step When the SessionHeader renders with the restored threshold
      const { lastFrame } = render(
        <SessionHeader {...props} />
      );
      const output = lastFrame();

      // @step Then the badge should display "[192k]" using the restored compaction threshold
      expect(output).toContain('[192k]');
    });
  });
});
