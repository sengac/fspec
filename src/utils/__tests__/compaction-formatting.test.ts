/**
 * Unit tests for compaction formatting utilities
 * Tests core business logic in isolation
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
      // Test each fixture against expected output
      Object.entries(compactionProgressFixtures).forEach(([key, progress]) => {
        const result = formatCompactionPlaceholder(progress);
        const expected =
          expectedCompactionTexts[key as keyof typeof expectedCompactionTexts];
        expect(result).toBe(expected);
      });
    });

    it('should handle single digit numbers correctly', () => {
      const progress: CompactionProgress = {
        phase: 'test phase',
        current: 1,
        total: 5,
      };
      expect(formatCompactionPlaceholder(progress)).toBe(
        'Compacting: test phase... 1/5 turns'
      );
    });

    it('should handle large numbers correctly', () => {
      const progress = compactionProgressFixtures.largeNumbers;
      expect(formatCompactionPlaceholder(progress)).toBe(
        'Compacting: processing chunks... 157/892 turns'
      );
    });

    it('should handle phases with special characters', () => {
      const progress: CompactionProgress = {
        phase: 'analyzing "quoted" content',
        current: 3,
        total: 10,
      };
      expect(formatCompactionPlaceholder(progress)).toBe(
        'Compacting: analyzing "quoted" content... 3/10 turns'
      );
    });
  });

  describe('formatCompactionThinking', () => {
    it('should format thinking text without "Compacting:" prefix', () => {
      const progress = compactionProgressFixtures.analyzingAnchors;
      const result = formatCompactionThinking(progress);
      expect(result).toBe('analyzing anchors... 15/32 turns');
    });

    it('should handle all fixture scenarios', () => {
      Object.values(compactionProgressFixtures).forEach(progress => {
        const result = formatCompactionThinking(progress);
        expect(result).toMatch(/^.+\.\.\. \d+\/\d+ turns$/);
        expect(result).not.toContain('Compacting:');
      });
    });

    it('should format generatingSummary correctly', () => {
      const progress = compactionProgressFixtures.generatingSummary;
      expect(formatCompactionThinking(progress)).toBe(
        'generating summary... 1/1 turns'
      );
    });
  });

  describe('formatCompactionProgress', () => {
    it('should use default formatting when no options provided', () => {
      const progress = compactionProgressFixtures.analyzingAnchors;
      const result = formatCompactionProgress(progress);
      expect(result).toBe('analyzing anchors... 15/32 turns');
    });

    it('should add custom prefix', () => {
      const progress = compactionProgressFixtures.analyzingAnchors;
      const result = formatCompactionProgress(progress, { prefix: 'Status: ' });
      expect(result).toBe('Status: analyzing anchors... 15/32 turns');
    });

    it('should add custom suffix', () => {
      const progress = compactionProgressFixtures.analyzingAnchors;
      const result = formatCompactionProgress(progress, {
        suffix: ' [ACTIVE]',
      });
      expect(result).toBe('analyzing anchors... 15/32 turns [ACTIVE]');
    });

    it('should hide "turns" when showTurns is false', () => {
      const progress = compactionProgressFixtures.analyzingAnchors;
      const result = formatCompactionProgress(progress, { showTurns: false });
      expect(result).toBe('analyzing anchors... 15/32');
    });

    it('should combine all options', () => {
      const progress = compactionProgressFixtures.generatingSummary;
      const result = formatCompactionProgress(progress, {
        prefix: '[COMP] ',
        suffix: ' - FINAL',
        showTurns: false,
      });
      expect(result).toBe('[COMP] generating summary... 1/1 - FINAL');
    });

    it('should trim whitespace correctly', () => {
      const progress = compactionProgressFixtures.analyzingAnchors;
      const result = formatCompactionProgress(progress, {
        prefix: '',
        suffix: '',
      });
      expect(result).toBe('analyzing anchors... 15/32 turns');
      expect(result).not.toMatch(/^\s|\s$/); // No leading/trailing whitespace
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
        { phase: 'test', current: -1, total: 5 }, // Negative current
        { phase: 'test', current: 1, total: 0 }, // Zero total
        { phase: 'test', current: 6, total: 5 }, // Current > total
        { phase: 'test', current: 1.5, total: 5 }, // Non-integer current
        { phase: 'test', current: 1, total: 5.5 }, // Non-integer total
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
    it('should calculate correct percentages for fixtures', () => {
      const scenarios = [
        { progress: compactionProgressFixtures.analyzingAnchors, expected: 47 }, // 15/32 = ~46.875 rounds to 47
        {
          progress: compactionProgressFixtures.generatingSummary,
          expected: 100,
        }, // 1/1 = 100
        {
          progress: compactionProgressFixtures.analyzingAnchorsStart,
          expected: 3,
        }, // 1/32 = ~3.125 rounds to 3
        { progress: compactionProgressFixtures.singleItem, expected: 100 }, // 1/1 = 100
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

    it('should handle large numbers correctly', () => {
      const progress = compactionProgressFixtures.largeNumbers;
      const result = calculateCompactionPercentage(progress);
      expect(result).toBe(18); // 157/892 = ~17.6% rounds to 18
      expect(result).toBeGreaterThanOrEqual(0);
      expect(result).toBeLessThanOrEqual(100);
    });

    it('should always return integers', () => {
      Object.values(compactionProgressFixtures).forEach(progress => {
        const result = calculateCompactionPercentage(progress);
        expect(Number.isInteger(result)).toBe(true);
      });
    });
  });

  describe('Consistency between formatters', () => {
    it('should maintain consistent phase/current/total formatting', () => {
      Object.values(compactionProgressFixtures).forEach(progress => {
        const placeholder = formatCompactionPlaceholder(progress);
        const thinking = formatCompactionThinking(progress);

        // Both should contain the same progress numbers
        const progressText = `${progress.current}/${progress.total} turns`;
        expect(placeholder).toContain(progressText);
        expect(thinking).toContain(progressText);

        // Both should contain the same phase
        expect(placeholder).toContain(progress.phase);
        expect(thinking).toContain(progress.phase);

        // Placeholder should have "Compacting:" prefix, thinking should not
        expect(placeholder).toContain('Compacting:');
        expect(thinking).not.toContain('Compacting:');
      });
    });
  });
});
