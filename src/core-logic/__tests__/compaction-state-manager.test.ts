/**
 * Unit Tests: Core Compaction State Management Logic
 *
 * Tests the business logic of compaction state coordination in isolation
 * NO React components, NO mocks, just pure business logic testing
 */

import { describe, it, expect } from 'vitest';
import { performance } from 'perf_hooks';
import type { CompactionProgress } from '../../tui/hooks/useRustSessionState';
import {
  isCompactionActive,
  getCurrentCompactionProgress,
  getCurrentCompactionTrigger,
  validateCompactionStateConsistency,
  createUnifiedCompactionState,
  shouldBlockInput,
  getPlaceholderText,
  type CompactionStateSources,
  type CompactionTrigger,
} from '../compaction-state-manager';
import { formatCompactionPlaceholder } from '../../utils/compaction-formatting';

// Test fixtures for consistent testing
const mockProgressAnalyzing: CompactionProgress = {
  phase: 'Analyzing context',
  current: 0,
  total: 1,
};

const mockProgressSummary: CompactionProgress = {
  phase: 'Preparing compaction',
  current: 1,
  total: 1,
};

const manualTrigger: CompactionTrigger = {
  type: 'manual',
  reason: 'User executed /compact command',
};

const hookTrigger: CompactionTrigger = {
  type: 'hook-triggered',
  reason: 'Token threshold exceeded',
  metadata: { threshold: 100000, current: 150000 },
};

describe('Core Logic: Compaction State Management', () => {
  describe('isCompactionActive', () => {
    it('should return false when both sources are inactive', () => {
      const sources: CompactionStateSources = {
        localProgressState: { isActive: false, progress: null, trigger: null },
        rustBackendState: { isCompacting: false, compactionProgress: null },
      };

      expect(isCompactionActive(sources)).toBe(false);
    });

    it('should return true when local state is active', () => {
      const sources: CompactionStateSources = {
        localProgressState: {
          isActive: true,
          progress: mockProgressAnalyzing,
          trigger: manualTrigger,
        },
        rustBackendState: { isCompacting: false, compactionProgress: null },
      };

      expect(isCompactionActive(sources)).toBe(true);
    });

    it('should return true when Rust state is active', () => {
      const sources: CompactionStateSources = {
        localProgressState: { isActive: false, progress: null, trigger: null },
        rustBackendState: {
          isCompacting: true,
          compactionProgress: mockProgressAnalyzing,
        },
      };

      expect(isCompactionActive(sources)).toBe(true);
    });

    it('should return true when both sources are active', () => {
      const sources: CompactionStateSources = {
        localProgressState: {
          isActive: true,
          progress: mockProgressAnalyzing,
          trigger: manualTrigger,
        },
        rustBackendState: {
          isCompacting: true,
          compactionProgress: mockProgressSummary,
        },
      };

      expect(isCompactionActive(sources)).toBe(true);
    });
  });

  describe('getCurrentCompactionProgress', () => {
    it('should return null when no compaction is active', () => {
      const sources: CompactionStateSources = {
        localProgressState: { isActive: false, progress: null, trigger: null },
        rustBackendState: { isCompacting: false, compactionProgress: null },
      };

      expect(getCurrentCompactionProgress(sources)).toBeNull();
    });

    it('should prioritize local progress when local state is active', () => {
      const sources: CompactionStateSources = {
        localProgressState: {
          isActive: true,
          progress: mockProgressAnalyzing,
          trigger: manualTrigger,
        },
        rustBackendState: {
          isCompacting: true,
          compactionProgress: mockProgressSummary,
        },
      };

      const result = getCurrentCompactionProgress(sources);
      expect(result).toEqual(mockProgressAnalyzing);
      expect(result?.phase).toBe('Analyzing context');
    });

    it('should use Rust progress when only Rust state is active', () => {
      const sources: CompactionStateSources = {
        localProgressState: { isActive: false, progress: null, trigger: null },
        rustBackendState: {
          isCompacting: true,
          compactionProgress: mockProgressSummary,
        },
      };

      const result = getCurrentCompactionProgress(sources);
      expect(result).toEqual(mockProgressSummary);
      expect(result?.phase).toBe('Preparing compaction');
    });

    it('should return null when local is active but has no progress', () => {
      const sources: CompactionStateSources = {
        localProgressState: {
          isActive: true,
          progress: null,
          trigger: manualTrigger,
        },
        rustBackendState: { isCompacting: false, compactionProgress: null },
      };

      expect(getCurrentCompactionProgress(sources)).toBeNull();
    });

    it('should return null when Rust is active but has no progress', () => {
      const sources: CompactionStateSources = {
        localProgressState: { isActive: false, progress: null, trigger: null },
        rustBackendState: { isCompacting: true, compactionProgress: null },
      };

      expect(getCurrentCompactionProgress(sources)).toBeNull();
    });
  });

  describe('getCurrentCompactionTrigger', () => {
    it('should return null when no compaction is active', () => {
      const sources: CompactionStateSources = {
        localProgressState: { isActive: false, progress: null, trigger: null },
        rustBackendState: { isCompacting: false, compactionProgress: null },
      };

      expect(getCurrentCompactionTrigger(sources)).toBeNull();
    });

    it('should return local trigger when local state is active', () => {
      const sources: CompactionStateSources = {
        localProgressState: {
          isActive: true,
          progress: mockProgressAnalyzing,
          trigger: manualTrigger,
        },
        rustBackendState: { isCompacting: false, compactionProgress: null },
      };

      const result = getCurrentCompactionTrigger(sources);
      expect(result).toEqual(manualTrigger);
      expect(result?.type).toBe('manual');
    });

    it('should infer automatic trigger when only Rust state is active', () => {
      const sources: CompactionStateSources = {
        localProgressState: { isActive: false, progress: null, trigger: null },
        rustBackendState: {
          isCompacting: true,
          compactionProgress: mockProgressSummary,
        },
      };

      const result = getCurrentCompactionTrigger(sources);
      expect(result).not.toBeNull();
      expect(result?.type).toBe('hook-triggered');
      expect(result?.reason).toContain('Automatic compaction');
      expect(result?.metadata?.source).toBe('rust-backend');
    });

    it('should prioritize local trigger when both are active', () => {
      const sources: CompactionStateSources = {
        localProgressState: {
          isActive: true,
          progress: mockProgressAnalyzing,
          trigger: hookTrigger,
        },
        rustBackendState: {
          isCompacting: true,
          compactionProgress: mockProgressSummary,
        },
      };

      const result = getCurrentCompactionTrigger(sources);
      expect(result).toEqual(hookTrigger);
      expect(result?.type).toBe('hook-triggered');
      expect(result?.metadata?.threshold).toBe(100000);
    });
  });

  describe('validateCompactionStateConsistency', () => {
    it('should pass validation when states are consistent and inactive', () => {
      const sources: CompactionStateSources = {
        localProgressState: { isActive: false, progress: null, trigger: null },
        rustBackendState: { isCompacting: false, compactionProgress: null },
      };

      const result = validateCompactionStateConsistency(sources);
      expect(result.isValid).toBe(true);
      expect(result.warnings).toHaveLength(0);
    });

    it('should pass validation when local state is properly configured', () => {
      const sources: CompactionStateSources = {
        localProgressState: {
          isActive: true,
          progress: mockProgressAnalyzing,
          trigger: manualTrigger,
        },
        rustBackendState: { isCompacting: false, compactionProgress: null },
      };

      const result = validateCompactionStateConsistency(sources);
      expect(result.isValid).toBe(true);
      expect(result.warnings).toHaveLength(0);
    });

    it('should warn when local state is active but missing progress', () => {
      const sources: CompactionStateSources = {
        localProgressState: {
          isActive: true,
          progress: null,
          trigger: manualTrigger,
        },
        rustBackendState: { isCompacting: false, compactionProgress: null },
      };

      const result = validateCompactionStateConsistency(sources);
      expect(result.isValid).toBe(false);
      expect(result.warnings).toContain(
        'Local compaction state is active but missing progress data'
      );
    });

    it('should warn when Rust state is active but missing progress', () => {
      const sources: CompactionStateSources = {
        localProgressState: { isActive: false, progress: null, trigger: null },
        rustBackendState: { isCompacting: true, compactionProgress: null },
      };

      const result = validateCompactionStateConsistency(sources);
      expect(result.isValid).toBe(false);
      expect(result.warnings).toContain(
        'Rust compaction state is active but missing progress data'
      );
    });

    it('should warn when both states are active with conflicting phases', () => {
      const sources: CompactionStateSources = {
        localProgressState: {
          isActive: true,
          progress: { phase: 'Analyzing context', current: 0, total: 1 },
          trigger: manualTrigger,
        },
        rustBackendState: {
          isCompacting: true,
          compactionProgress: {
            phase: 'Preparing compaction',
            current: 1,
            total: 1,
          },
        },
      };

      const result = validateCompactionStateConsistency(sources);
      expect(result.isValid).toBe(false);
      expect(result.warnings[0]).toContain('State conflict');
      expect(result.warnings[0]).toContain('Analyzing context');
      expect(result.warnings[0]).toContain('Preparing compaction');
    });

    it('should pass validation when both states have matching phases', () => {
      const matchingProgress = {
        phase: 'Analyzing context',
        current: 0,
        total: 1,
      };
      const sources: CompactionStateSources = {
        localProgressState: {
          isActive: true,
          progress: matchingProgress,
          trigger: manualTrigger,
        },
        rustBackendState: {
          isCompacting: true,
          compactionProgress: matchingProgress,
        },
      };

      const result = validateCompactionStateConsistency(sources);
      expect(result.isValid).toBe(true);
      expect(result.warnings).toHaveLength(0);
    });
  });

  describe('createUnifiedCompactionState', () => {
    it('should create inactive state when no compaction is running', () => {
      const sources: CompactionStateSources = {
        localProgressState: { isActive: false, progress: null, trigger: null },
        rustBackendState: { isCompacting: false, compactionProgress: null },
      };

      const state = createUnifiedCompactionState(sources);
      expect(state.isActive).toBe(false);
      expect(state.progress).toBeNull();
      expect(state.trigger).toBeNull();
      expect(state.startTime).toBeNull();
    });

    it('should create active state with local priority', () => {
      const sources: CompactionStateSources = {
        localProgressState: {
          isActive: true,
          progress: mockProgressAnalyzing,
          trigger: manualTrigger,
        },
        rustBackendState: { isCompacting: false, compactionProgress: null },
      };

      const state = createUnifiedCompactionState(sources);
      expect(state.isActive).toBe(true);
      expect(state.progress).toEqual(mockProgressAnalyzing);
      expect(state.trigger).toEqual(manualTrigger);
      expect(state.startTime).toBeTypeOf('number');
      expect(state.startTime).toBeGreaterThan(Date.now() - 1000);
    });

    it('should create active state from Rust backend only', () => {
      const sources: CompactionStateSources = {
        localProgressState: { isActive: false, progress: null, trigger: null },
        rustBackendState: {
          isCompacting: true,
          compactionProgress: mockProgressSummary,
        },
      };

      const state = createUnifiedCompactionState(sources);
      expect(state.isActive).toBe(true);
      expect(state.progress).toEqual(mockProgressSummary);
      expect(state.trigger?.type).toBe('hook-triggered');
      expect(state.startTime).toBeTypeOf('number');
    });
  });

  describe('shouldBlockInput', () => {
    it('should not block input when no compaction is active', () => {
      const sources: CompactionStateSources = {
        localProgressState: { isActive: false, progress: null, trigger: null },
        rustBackendState: { isCompacting: false, compactionProgress: null },
      };

      expect(shouldBlockInput(sources)).toBe(false);
    });

    it('should block input when local compaction is active', () => {
      const sources: CompactionStateSources = {
        localProgressState: {
          isActive: true,
          progress: mockProgressAnalyzing,
          trigger: manualTrigger,
        },
        rustBackendState: { isCompacting: false, compactionProgress: null },
      };

      expect(shouldBlockInput(sources)).toBe(true);
    });

    it('should block input when Rust compaction is active', () => {
      const sources: CompactionStateSources = {
        localProgressState: { isActive: false, progress: null, trigger: null },
        rustBackendState: {
          isCompacting: true,
          compactionProgress: mockProgressSummary,
        },
      };

      expect(shouldBlockInput(sources)).toBe(true);
    });

    it('should block input when both sources are active', () => {
      const sources: CompactionStateSources = {
        localProgressState: {
          isActive: true,
          progress: mockProgressAnalyzing,
          trigger: manualTrigger,
        },
        rustBackendState: {
          isCompacting: true,
          compactionProgress: mockProgressSummary,
        },
      };

      expect(shouldBlockInput(sources)).toBe(true);
    });
  });

  describe('getPlaceholderText', () => {
    it('should return default placeholder when no compaction is active', () => {
      const sources: CompactionStateSources = {
        localProgressState: { isActive: false, progress: null, trigger: null },
        rustBackendState: { isCompacting: false, compactionProgress: null },
      };

      const result = getPlaceholderText(
        sources,
        'Type a message...',
        formatCompactionPlaceholder
      );

      expect(result).toBe('Type a message...');
    });

    it('should return formatted compaction text when local compaction is active', () => {
      const sources: CompactionStateSources = {
        localProgressState: {
          isActive: true,
          progress: mockProgressAnalyzing,
          trigger: manualTrigger,
        },
        rustBackendState: { isCompacting: false, compactionProgress: null },
      };

      const result = getPlaceholderText(
        sources,
        'Type a message...',
        formatCompactionPlaceholder
      );

      expect(result).toBe('Compacting: Analyzing context...');
      expect(result).toContain('Compacting:');
      expect(result).toContain('Analyzing context');
      expect(result).toContain('Analyzing context');
    });

    it('should return formatted compaction text when Rust compaction is active', () => {
      const sources: CompactionStateSources = {
        localProgressState: { isActive: false, progress: null, trigger: null },
        rustBackendState: {
          isCompacting: true,
          compactionProgress: mockProgressSummary,
        },
      };

      const result = getPlaceholderText(
        sources,
        'Type a message...',
        formatCompactionPlaceholder
      );

      expect(result).toBe('Compacting: Preparing compaction...');
      expect(result).toContain('Preparing compaction');
    });

    it('should prioritize local progress for formatting', () => {
      const sources: CompactionStateSources = {
        localProgressState: {
          isActive: true,
          progress: mockProgressAnalyzing,
          trigger: manualTrigger,
        },
        rustBackendState: {
          isCompacting: true,
          compactionProgress: mockProgressSummary,
        },
      };

      const result = getPlaceholderText(
        sources,
        'Type a message...',
        formatCompactionPlaceholder
      );

      // Should use local progress (Analyzing context), not Rust progress (summary)
      expect(result).toContain('Analyzing context');
      expect(result).not.toContain('generating summary');
    });

    it('should work with custom formatting functions', () => {
      const sources: CompactionStateSources = {
        localProgressState: { isActive: false, progress: null, trigger: null },
        rustBackendState: {
          isCompacting: true,
          compactionProgress: mockProgressAnalyzing,
        },
      };

      const customFormat = (progress: CompactionProgress) =>
        `[CUSTOM] ${progress.phase} (${progress.current}/${progress.total})`;

      const result = getPlaceholderText(sources, 'Default text', customFormat);

      expect(result).toBe('[CUSTOM] Analyzing context (0/1)');
    });
  });

  describe('Edge Cases and Error Handling', () => {
    it('should handle null/undefined progress gracefully', () => {
      const sources: CompactionStateSources = {
        localProgressState: {
          isActive: true,
          progress: null,
          trigger: manualTrigger,
        },
        rustBackendState: {
          isCompacting: true,
          compactionProgress: undefined as CompactionProgress | null,
        },
      };

      expect(isCompactionActive(sources)).toBe(true);
      expect(getCurrentCompactionProgress(sources)).toBeNull();
      expect(shouldBlockInput(sources)).toBe(true);

      const placeholder = getPlaceholderText(
        sources,
        'Default',
        formatCompactionPlaceholder
      );
      expect(placeholder).toBe('Default');
    });

    it('should handle malformed trigger objects', () => {
      const malformedTrigger = { type: 'unknown' } as CompactionTrigger;
      const sources: CompactionStateSources = {
        localProgressState: {
          isActive: true,
          progress: mockProgressAnalyzing,
          trigger: malformedTrigger,
        },
        rustBackendState: { isCompacting: false, compactionProgress: null },
      };

      const trigger = getCurrentCompactionTrigger(sources);
      expect(trigger).toEqual(malformedTrigger);

      // Core functions should still work
      expect(isCompactionActive(sources)).toBe(true);
      expect(shouldBlockInput(sources)).toBe(true);
    });

    it('should handle zero progress values correctly', () => {
      const zeroProgress: CompactionProgress = {
        phase: 'Starting',
        current: 0,
        total: 1,
      };

      const sources: CompactionStateSources = {
        localProgressState: {
          isActive: true,
          progress: zeroProgress,
          trigger: manualTrigger,
        },
        rustBackendState: { isCompacting: false, compactionProgress: null },
      };

      expect(getCurrentCompactionProgress(sources)).toEqual(zeroProgress);

      const placeholder = getPlaceholderText(
        sources,
        'Default',
        formatCompactionPlaceholder
      );
      expect(placeholder).toBe('Compacting: Starting...');
    });
  });

  describe('Performance and Consistency', () => {
    it('should handle rapid state changes efficiently', () => {
      const states: CompactionStateSources[] = [];
      const iterations = 1000;

      // Generate many state variations
      for (let i = 0; i < iterations; i++) {
        states.push({
          localProgressState: {
            isActive: i % 2 === 0,
            progress: i % 3 === 0 ? mockProgressAnalyzing : null,
            trigger: i % 4 === 0 ? manualTrigger : null,
          },
          rustBackendState: {
            isCompacting: i % 5 === 0,
            compactionProgress: i % 6 === 0 ? mockProgressSummary : null,
          },
        });
      }

      const startTime = performance.now();

      // Process all states
      states.forEach(sources => {
        isCompactionActive(sources);
        getCurrentCompactionProgress(sources);
        shouldBlockInput(sources);
        getPlaceholderText(sources, 'Default', formatCompactionPlaceholder);
      });

      const endTime = performance.now();
      const totalTime = endTime - startTime;

      // Should process 1000 states quickly
      expect(totalTime).toBeLessThan(100);

      const avgTimePerState = totalTime / iterations;
      expect(avgTimePerState).toBeLessThan(0.1); // < 0.1ms per state
    });

    it('should produce consistent results for identical inputs', () => {
      const sources: CompactionStateSources = {
        localProgressState: {
          isActive: true,
          progress: mockProgressAnalyzing,
          trigger: manualTrigger,
        },
        rustBackendState: { isCompacting: false, compactionProgress: null },
      };

      // Call functions multiple times
      const results = Array.from({ length: 10 }, () => ({
        isActive: isCompactionActive(sources),
        progress: getCurrentCompactionProgress(sources),
        shouldBlock: shouldBlockInput(sources),
        placeholder: getPlaceholderText(
          sources,
          'Default',
          formatCompactionPlaceholder
        ),
      }));

      // All results should be identical
      const firstResult = results[0];
      results.forEach(result => {
        expect(result.isActive).toBe(firstResult.isActive);
        expect(result.progress).toEqual(firstResult.progress);
        expect(result.shouldBlock).toBe(firstResult.shouldBlock);
        expect(result.placeholder).toBe(firstResult.placeholder);
      });
    });
  });
});
