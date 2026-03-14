/**
 * Feature: spec/features/hitl-handler-wiring.feature
 *
 * Part 2: Snapshot polling integration.
 * Uses mock RustStateSource to test useRustSessionState snapshot
 * includes hitlRequest correctly when session is paused.
 *
 * BUG-118: HITL TUI integration
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';

import {
  refreshSessionState,
  clearAllSubscriptions,
  getSessionSnapshotForTesting,
  setRustStateSource,
  resetRustStateSource,
  type RustStateSource,
} from '../hooks/useRustSessionState';

import {
  parseHitlRequestInfo,
  hitlRequestInfoEqual,
  type HitlRequestInfo,
} from '../types/hitlRequest';

// ============================================================================
// Helpers + Fixtures
// ============================================================================

function createHitlMockSource(overrides: {
  status?: string;
  hitlRequest?: HitlRequestInfo | null;
}): RustStateSource {
  return {
    getStatus: () => overrides.status ?? 'idle',
    getModel: () => null,
    getTokens: () => ({ inputTokens: 0, outputTokens: 0 }),
    getDebugEnabled: () => false,
    getPauseState: () => null,
    getBaseThinkingLevel: () => 0,
    setBaseThinkingLevel: () => {},
    getCompactionProgress: () => null,
    getHitlRequest: () => overrides.hitlRequest ?? null,
  };
}

const FIXTURE_OPTIONS: HitlRequestInfo = {
  questions: [
    {
      id: 'approach',
      header: 'Approach',
      question: 'Which approach do you prefer?',
      options: [
        { label: 'Option A', description: 'First approach' },
        { label: 'Option B', description: 'Second approach' },
      ],
    },
  ],
};

const FIXTURE_MULTI: HitlRequestInfo = {
  questions: [
    {
      id: 'priority',
      header: 'Priority',
      question: 'What priority?',
      options: [
        { label: 'High', description: 'Now' },
        { label: 'Low', description: 'Later' },
      ],
    },
    {
      id: 'scope',
      header: 'Scope',
      question: 'What scope?',
      options: [
        { label: 'Minimal', description: 'Essentials' },
        { label: 'Full', description: 'Everything' },
        { label: 'Custom', description: 'Specify' },
      ],
    },
  ],
};

const FIXTURE_FREEFORM: HitlRequestInfo = {
  questions: [
    {
      id: 'feedback',
      header: 'Feedback',
      question: 'Any feedback?',
    },
  ],
};

// ============================================================================
// Tests
// ============================================================================

describe('HITL Snapshot Polling Integration', () => {
  beforeEach(() => {
    clearAllSubscriptions();
  });

  afterEach(() => {
    resetRustStateSource();
    clearAllSubscriptions();
  });

  // ==========================================================================
  // Scenario: useRustSessionState includes hitlRequest when paused
  // ==========================================================================

  describe('Scenario: Snapshot includes hitlRequest when paused', () => {
    it('should have hitlRequest with options question', () => {
      // @step Given a session is paused with HITL request
      setRustStateSource(
        createHitlMockSource({
          status: 'paused',
          hitlRequest: FIXTURE_OPTIONS,
        })
      );

      // @step When snapshot is fetched
      refreshSessionState('snap-options-test');
      const snap = getSessionSnapshotForTesting('snap-options-test');

      // @step Then isPaused should be true
      expect(snap.isPaused).toBe(true);

      // @step And hitlRequest should contain the question
      expect(snap.hitlRequest).not.toBeNull();
      expect(snap.hitlRequest?.questions).toHaveLength(1);
      expect(snap.hitlRequest?.questions[0].id).toBe('approach');
      expect(snap.hitlRequest?.questions[0].options).toHaveLength(2);
    });

    it('should have hitlRequest with multi-step questions', () => {
      // @step Given a session is paused with 2 questions
      setRustStateSource(
        createHitlMockSource({
          status: 'paused',
          hitlRequest: FIXTURE_MULTI,
        })
      );

      // @step When snapshot is fetched
      refreshSessionState('snap-multi-test');
      const snap = getSessionSnapshotForTesting('snap-multi-test');

      // @step Then hitlRequest should contain 2 questions
      expect(snap.hitlRequest?.questions).toHaveLength(2);
      expect(snap.hitlRequest?.questions[0].id).toBe('priority');
      expect(snap.hitlRequest?.questions[1].id).toBe('scope');
      expect(snap.hitlRequest?.questions[1].options).toHaveLength(3);
    });

    it('should have hitlRequest for freeform question', () => {
      // @step Given a session is paused with freeform question
      setRustStateSource(
        createHitlMockSource({
          status: 'paused',
          hitlRequest: FIXTURE_FREEFORM,
        })
      );

      // @step When snapshot is fetched
      refreshSessionState('snap-freeform-test');
      const snap = getSessionSnapshotForTesting('snap-freeform-test');

      // @step Then hitlRequest should have a question without options
      expect(snap.hitlRequest?.questions[0].id).toBe('feedback');
      expect(snap.hitlRequest?.questions[0].options).toBeUndefined();
    });
  });

  // ==========================================================================
  // Scenario: Null hitlRequest when not paused
  // ==========================================================================

  describe('Scenario: Null hitlRequest when not paused', () => {
    it('should return null when running', () => {
      // @step Given a running session
      setRustStateSource(createHitlMockSource({ status: 'running' }));
      refreshSessionState('snap-running');
      const snap = getSessionSnapshotForTesting('snap-running');

      // @step Then hitlRequest should be null
      expect(snap.hitlRequest).toBeNull();
      expect(snap.isPaused).toBe(false);
    });

    it('should return null when idle', () => {
      // @step Given an idle session
      setRustStateSource(createHitlMockSource({ status: 'idle' }));
      refreshSessionState('snap-idle');
      const snap = getSessionSnapshotForTesting('snap-idle');

      // @step Then hitlRequest should be null
      expect(snap.hitlRequest).toBeNull();
    });
  });

  // ==========================================================================
  // Snapshot caching
  // ==========================================================================

  describe('Snapshot caching with hitlRequest', () => {
    it('should return same ref when data unchanged', () => {
      // @step Given a paused session with HITL request
      setRustStateSource(
        createHitlMockSource({
          status: 'paused',
          hitlRequest: FIXTURE_OPTIONS,
        })
      );
      refreshSessionState('snap-cache');

      // @step When fetched twice without version change
      const s1 = getSessionSnapshotForTesting('snap-cache');
      const s2 = getSessionSnapshotForTesting('snap-cache');

      // @step Then same reference (cache hit)
      expect(s1).toBe(s2);
    });
  });

  // ==========================================================================
  // Type utilities
  // ==========================================================================

  describe('parseHitlRequestInfo', () => {
    it('should parse valid NAPI response', () => {
      const result = parseHitlRequestInfo({
        questions: [
          {
            id: 'test_q',
            header: 'Test',
            question: 'Works?',
            options: [{ label: 'Yes', description: 'It works' }],
          },
        ],
      });
      expect(result).not.toBeNull();
      expect(result?.questions[0].id).toBe('test_q');
    });

    it('should return null for empty questions', () => {
      expect(parseHitlRequestInfo({ questions: [] })).toBeNull();
    });

    it('should return null for null/undefined', () => {
      expect(parseHitlRequestInfo(null)).toBeNull();
      expect(parseHitlRequestInfo(undefined)).toBeNull();
    });

    it('should handle missing optional fields', () => {
      const result = parseHitlRequestInfo({ questions: [{ id: 'q1' }] });
      expect(result?.questions[0].header).toBe('');
      expect(result?.questions[0].question).toBe('');
      expect(result?.questions[0].options).toBeUndefined();
    });
  });

  describe('hitlRequestInfoEqual', () => {
    it('should return true for identical requests', () => {
      const clone: HitlRequestInfo = {
        questions: [
          {
            id: 'approach',
            header: 'Approach',
            question: 'Which approach do you prefer?',
            options: [
              { label: 'Option A', description: 'First approach' },
              { label: 'Option B', description: 'Second approach' },
            ],
          },
        ],
      };
      expect(hitlRequestInfoEqual(FIXTURE_OPTIONS, clone)).toBe(true);
    });

    it('should return false for different ids', () => {
      const altered: HitlRequestInfo = {
        questions: [
          {
            id: 'different',
            header: 'Approach',
            question: 'Which approach do you prefer?',
            options: [
              { label: 'Option A', description: 'First approach' },
              { label: 'Option B', description: 'Second approach' },
            ],
          },
        ],
      };
      expect(hitlRequestInfoEqual(FIXTURE_OPTIONS, altered)).toBe(false);
    });

    it('should handle null comparisons', () => {
      expect(hitlRequestInfoEqual(null, null)).toBe(true);
      expect(hitlRequestInfoEqual(FIXTURE_OPTIONS, null)).toBe(false);
      expect(hitlRequestInfoEqual(null, FIXTURE_OPTIONS)).toBe(false);
    });
  });
});
