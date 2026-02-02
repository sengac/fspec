/**
 * Test fixtures for compaction-related testing
 * Provides reusable mock data for consistent testing across components
 */

import type { CompactionProgress } from '../tui/hooks/useRustSessionState';

export const compactionProgressFixtures: Record<string, CompactionProgress> = {
  analyzingAnchors: {
    phase: 'analyzing anchors',
    current: 15,
    total: 32,
  },

  analyzingAnchorsStart: {
    phase: 'analyzing anchors',
    current: 1,
    total: 32,
  },

  analyzingAnchorsEnd: {
    phase: 'analyzing anchors',
    current: 32,
    total: 32,
  },

  generatingSummary: {
    phase: 'generating summary',
    current: 1,
    total: 1,
  },

  generatingSummaryProgress: {
    phase: 'generating summary',
    current: 3,
    total: 5,
  },

  optimizingContext: {
    phase: 'optimizing context',
    current: 8,
    total: 45,
  },

  // Edge cases
  singleItem: {
    phase: 'finalizing',
    current: 1,
    total: 1,
  },

  largeNumbers: {
    phase: 'processing chunks',
    current: 157,
    total: 892,
  },

  // Realistic emergency scenarios
  emergencyLarge: {
    phase: 'analyzing anchors',
    current: 8,
    total: 45,
  },

  // Hook-triggered scenarios
  hookTriggered: {
    phase: 'analyzing anchors',
    current: 10,
    total: 25,
  },
};

/**
 * Common test scenarios for different compaction triggers
 */
export const compactionScenarios = {
  manual: {
    trigger: 'manual',
    description: 'User types /compact command',
    progress: compactionProgressFixtures.analyzingAnchors,
  },

  hookTriggered: {
    trigger: 'hook-triggered',
    description: 'Token threshold exceeded, hook triggers compaction',
    progress: compactionProgressFixtures.hookTriggered,
  },

  emergency: {
    trigger: 'emergency',
    description:
      'API rejects prompt as too long, emergency compaction triggered',
    progress: compactionProgressFixtures.emergencyLarge,
  },
};

/**
 * Expected formatted output for each fixture
 * Used to verify compaction text formatting consistency
 */
export const expectedCompactionTexts = {
  analyzingAnchors: 'Compacting: analyzing anchors... 15/32 turns',
  analyzingAnchorsStart: 'Compacting: analyzing anchors... 1/32 turns',
  analyzingAnchorsEnd: 'Compacting: analyzing anchors... 32/32 turns',
  generatingSummary: 'Compacting: generating summary... 1/1 turns',
  generatingSummaryProgress: 'Compacting: generating summary... 3/5 turns',
  optimizingContext: 'Compacting: optimizing context... 8/45 turns',
  singleItem: 'Compacting: finalizing... 1/1 turns',
  largeNumbers: 'Compacting: processing chunks... 157/892 turns',
  emergencyLarge: 'Compacting: analyzing anchors... 8/45 turns',
  hookTriggered: 'Compacting: analyzing anchors... 10/25 turns',
};

/**
 * Mock input scenarios for testing keyboard handling during compaction
 */
export const inputTestScenarios = {
  typing: {
    input: 'hello world',
    description: 'User types text during compaction',
  },

  backspace: {
    input: '\x7f',
    description: 'User presses backspace during compaction',
  },

  delete: {
    input: '\x1b[3~',
    description: 'User presses delete key during compaction',
  },

  enter: {
    input: '\r',
    description: 'User presses enter during compaction',
  },

  wordDelete: {
    input: '\x1b\x7f', // Alt+Backspace
    description: 'User presses Alt+Backspace during compaction',
  },
};
