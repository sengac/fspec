/**
 * Unit tests for compaction formatting utilities
 *
 * Compaction is a brief in-memory setup (~5ms) followed by agent-driven
 * DAG construction. Formatting no longer shows "X/Y turns".
 */

import { describe, it, expect } from 'vitest';
import {
  formatCompactionPlaceholder,
  formatCompactionThinking,
  formatCompactionProgress,
  isValidCompactionProgress,
  calculateCompactionPercentage,
} from '../compaction-formatting';
import {
  compactionProgressFixtures,
  expectedCompactionTexts,
} from '../../test-helpers/compaction-fixtures';
import type { CompactionProgress } from '../../tui/hooks/useRustSessionState';

describe('Compaction Formatting Utilities', () => {
  describe('formatCompactionPlaceholder', () => {
    it('should format all fixture scenarios correctly', () => {
      Object.entries(compactionProgressFixtures).forEach(([key, progress]) => {
        const result = formatCompactionPlaceholder(progress);
        const expected =
          expectedCompactionTexts[key as keyof typeof expectedCompactionTexts];
        expect(result).toBe(expected);
      });
    });

    it('should format as "Compacting: phase..."', () => {
      const progress: CompactionProgress = {
        phase: 'test phase',
        current: 1,
        total: 5,
      };
      expect(formatCompactionPlaceholder(progress)).toBe(
        'Compacting: test phase...'
      );
    });

    it('should not include turn counts', () => {
      const progress = compactionProgressFixtures.analyzingContext;
      const result = formatCompactionPlaceholder(progress);
      expect(result).not.toContain('turns');
      expect(result).not.toMatch(/\d+\/\d+/);
    });

    it('should handle phases with special characters', () => {
      const progress: CompactionProgress = {
        phase: 'analyzing "quoted" content',
        current: 3,
        total: 10,
      };
      expect(formatCompactionPlaceholder(progress)).toBe(
        'Compacting: analyzing "quoted" content...'
      );
    });
  });

  describe('formatCompactionThinking', () => {
    it('should format thinking text without "Compacting:" prefix', () => {
      const progress = compactionProgressFixtures.analyzingContext;
      const result = formatCompactionThinking(progress);
      expect(result).toBe('Analyzing context...');
    });

    it('should handle all fixture scenarios', () => {
      Object.values(compactionProgressFixtures).forEach(progress => {
        const result = formatCompactionThinking(progress);
        expect(result).toMatch(/^.+\.\.\.$/);
        expect(result).not.toContain('Compacting:');
        expect(result).not.toContain('turns');
      });
    });

    it('should format generatingSummary correctly', () => {
      const progress = compactionProgressFixtures.generatingSummary;
      expect(formatCompactionThinking(progress)).toBe(
        'Preparing compaction...'
      );
    });
  });

  describe('formatCompactionProgress', () => {
    it('should use default formatting when no options provided', () => {
      const progress = compactionProgressFixtures.analyzingContext;
      const result = formatCompactionProgress(progress);
      expect(result).toBe('Analyzing context...');
    });

    it('should add custom prefix', () => {
      const progress = compactionProgressFixtures.analyzingContext;
      const result = formatCompactionProgress(progress, { prefix: 'Status: ' });
      expect(result).toBe('Status: Analyzing context...');
    });

    it('should add custom suffix', () => {
      const progress = compactionProgressFixtures.analyzingContext;
      const result = formatCompactionProgress(progress, {
        suffix: ' [ACTIVE]',
      });
      expect(result).toBe('Analyzing context... [ACTIVE]');
    });

    it('should combine all options', () => {
      const progress = compactionProgressFixtures.generatingSummary;
      const result = formatCompactionProgress(progress, {
        prefix: '[COMP] ',
        suffix: ' - FINAL',
      });
      expect(result).toBe('[COMP] Preparing compaction... - FINAL');
    });

    it('should trim whitespace correctly', () => {
      const progress = compactionProgressFixtures.analyzingContext;
      const result = formatCompactionProgress(progress, {
        prefix: '',
        suffix: '',
      });
      expect(result).toBe('Analyzing context...');
      expect(result).not.toMatch(/^\s|\s$/);
    });
  });

  describe('isValidCompactionProgress', () => {
    it('should return true for valid progress objects', () => {
      Object.values(compactionProgressFixtures).forEach(progress => {
        expect(isValidCompactionProgress(progress)).toBe(true);
      });
    });

    it('should return false for null or undefined', () => {
      expect(isValidCompactionProgress(null)).toBe(false);
      expect(isValidCompactionProgress(undefined)).toBe(false);
    });

    it('should return false for invalid phase', () => {
      const invalidProgress = {
        phase: '',
        current: 1,
        total: 5,
      } as CompactionProgress;
      expect(isValidCompactionProgress(invalidProgress)).toBe(false);
    });

    it('should return false for invalid numbers', () => {
      const scenarios = [
        { phase: 'test', current: -1, total: 5 },
        { phase: 'test', current: 1, total: 0 },
        { phase: 'test', current: 6, total: 5 },
        { phase: 'test', current: 1.5, total: 5 },
        { phase: 'test', current: 1, total: 5.5 },
      ];

      scenarios.forEach(scenario => {
        expect(isValidCompactionProgress(scenario as CompactionProgress)).toBe(
          false
        );
      });
    });

    it('should return true for edge case: current equals total', () => {
      const edgeCase: CompactionProgress = {
        phase: 'finalizing',
        current: 5,
        total: 5,
      };
      expect(isValidCompactionProgress(edgeCase)).toBe(true);
    });

    it('should return true for edge case: current is zero', () => {
      const edgeCase: CompactionProgress = {
        phase: 'starting',
        current: 0,
        total: 5,
      };
      expect(isValidCompactionProgress(edgeCase)).toBe(true);
    });
  });

  describe('calculateCompactionPercentage', () => {
    it('should calculate correct percentages', () => {
      const scenarios = [
        { progress: { phase: 'test', current: 1, total: 1 }, expected: 100 },
        { progress: { phase: 'test', current: 0, total: 1 }, expected: 0 },
        { progress: { phase: 'test', current: 1, total: 2 }, expected: 50 },
      ];

      scenarios.forEach(({ progress, expected }) => {
        const result = calculateCompactionPercentage(progress);
        expect(result).toBe(expected);
      });
    });

    it('should return 0 for invalid progress', () => {
      expect(calculateCompactionPercentage(null)).toBe(0);
      expect(calculateCompactionPercentage(undefined)).toBe(0);

      const invalidProgress = {
        phase: '',
        current: 1,
        total: 5,
      } as CompactionProgress;
      expect(calculateCompactionPercentage(invalidProgress)).toBe(0);
    });

    it('should return 0 for zero total (edge case)', () => {
      const zeroTotal = {
        phase: 'test',
        current: 0,
        total: 0,
      } as CompactionProgress;
      expect(calculateCompactionPercentage(zeroTotal)).toBe(0);
    });

    it('should always return integers', () => {
      Object.values(compactionProgressFixtures).forEach(progress => {
        const result = calculateCompactionPercentage(progress);
        expect(Number.isInteger(result)).toBe(true);
      });
    });
  });

  describe('Consistency between formatters', () => {
    it('should maintain consistent phase formatting', () => {
      Object.values(compactionProgressFixtures).forEach(progress => {
        const placeholder = formatCompactionPlaceholder(progress);
        const thinking = formatCompactionThinking(progress);

        // Both should contain the same phase
        expect(placeholder).toContain(progress.phase);
        expect(thinking).toContain(progress.phase);

        // Placeholder should have "Compacting:" prefix, thinking should not
        expect(placeholder).toContain('Compacting:');
        expect(thinking).not.toContain('Compacting:');

        // Neither should contain turn counts
        expect(placeholder).not.toContain('turns');
        expect(thinking).not.toContain('turns');
      });
    });
  });
});
