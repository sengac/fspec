/**
 * Test fixtures for compaction-related testing
 * Provides reusable mock data for consistent testing across components
 *
 * Compaction is a brief in-memory setup (~5ms) followed by agent-driven
 * DAG construction. Progress data describes the Rust setup phase.
 */

import type { CompactionProgress } from '../tui/hooks/useRustSessionState';

export const compactionProgressFixtures: Record<string, CompactionProgress> = {
  analyzingContext: {
    phase: 'Analyzing context',
    current: 0,
    total: 1,
  },

  analyzingContextStart: {
    phase: 'Analyzing context',
    current: 0,
    total: 1,
  },

  analyzingContextEnd: {
    phase: 'Analyzing context',
    current: 1,
    total: 1,
  },

  generatingSummary: {
    phase: 'Preparing compaction',
    current: 1,
    total: 1,
  },

  generatingSummaryProgress: {
    phase: 'Preparing compaction',
    current: 0,
    total: 1,
  },

  optimizingContext: {
    phase: 'Optimizing context',
    current: 0,
    total: 1,
  },

  // Edge cases
  singleItem: {
    phase: 'Finalizing',
    current: 1,
    total: 1,
  },

  largeNumbers: {
    phase: 'Processing',
    current: 0,
    total: 1,
  },

  // Realistic emergency scenarios
  emergencyLarge: {
    phase: 'Emergency compaction',
    current: 0,
    total: 1,
  },

  // Hook-triggered scenarios
  hookTriggered: {
    phase: 'Context limit reached',
    current: 0,
    total: 1,
  },
};

/**
 * Common test scenarios for different compaction triggers
 */
export const compactionScenarios = {
  manual: {
    trigger: 'manual',
    description: 'User types /compact command',
    progress: compactionProgressFixtures.analyzingContext,
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
 *
 * No more "X/Y turns" — compaction is brief setup, not turn processing
 */
export const expectedCompactionTexts = {
  analyzingContext: 'Compacting: Analyzing context...',
  analyzingContextStart: 'Compacting: Analyzing context...',
  analyzingContextEnd: 'Compacting: Analyzing context...',
  generatingSummary: 'Compacting: Preparing compaction...',
  generatingSummaryProgress: 'Compacting: Preparing compaction...',
  optimizingContext: 'Compacting: Optimizing context...',
  singleItem: 'Compacting: Finalizing...',
  largeNumbers: 'Compacting: Processing...',
  emergencyLarge: 'Compacting: Emergency compaction...',
  hookTriggered: 'Compacting: Context limit reached...',
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
