/**
 * Tests for useRustSessionState hook internals
 *
 * Feature: Rust session state subscription with proper caching
 *
 * These tests verify that the snapshot caching logic works correctly to avoid
 * React's "infinite loop" warning from useSyncExternalStore, while still providing
 * reactive updates when Rust state actually changes.
 *
 * Note: We test the internal functions directly rather than rendering React hooks,
 * as this provides more direct verification of the caching logic and avoids
 * React version compatibility issues with testing-library.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import {
  refreshSessionState,
  clearAllSubscriptions,
  getSubscriptionForTesting,
  getEmptySnapshotForTesting,
  setRustStateSource,
  resetRustStateSource,
  type RustStateSource,
} from '../useRustSessionState';

// =============================================================================
// We need to export the internal snapshot function for testing
// Add this to useRustSessionState.ts for testability
// =============================================================================

// For testing, we'll create a test helper that simulates what getSnapshot does
// This tests the caching logic without needing React

interface MockState {
  status: string;
  model: { providerId: string; modelId: string } | null;
  tokens: { inputTokens: number; outputTokens: number };
  debugEnabled: boolean;
}

function createMockStateSource(initialState: MockState): {
  source: RustStateSource;
  setState: (state: Partial<MockState>) => void;
  getCallCounts: () => {
    status: number;
    model: number;
    tokens: number;
    debug: number;
  };
  resetCallCounts: () => void;
} {
  let state = { ...initialState };
  let callCounts = { status: 0, model: 0, tokens: 0, debug: 0 };

  const source: RustStateSource = {
    getStatus: () => {
      callCounts.status++;
      return state.status;
    },
    getModel: () => {
      callCounts.model++;
      return state.model;
    },
    getTokens: () => {
      callCounts.tokens++;
      return state.tokens;
    },
    getDebugEnabled: () => {
      callCounts.debug++;
      return state.debugEnabled;
    },
  };

  return {
    source,
    setState: (newState: Partial<MockState>) => {
      state = { ...state, ...newState };
    },
    getCallCounts: () => ({ ...callCounts }),
    resetCallCounts: () => {
      callCounts = { status: 0, model: 0, tokens: 0, debug: 0 };
    },
  };
}

// Import the internal function that we expose for testing
// We need to add this export to the main file
import { getSessionSnapshotForTesting } from '../useRustSessionState';

// =============================================================================
// Test Setup
// =============================================================================

describe('Feature: Rust session state subscription with proper caching', () => {
  beforeEach(() => {
    clearAllSubscriptions();
  });

  afterEach(() => {
    resetRustStateSource();
    clearAllSubscriptions();
  });

  // ===========================================================================
  // Scenario: Snapshot caching prevents unnecessary Rust calls
  // ===========================================================================

  describe('Scenario: Snapshot caching prevents unnecessary Rust calls', () => {
    it('should return cached snapshot without Rust calls when version unchanged', () => {
      // Given a mock state source that tracks call counts
      const mock = createMockStateSource({
        status: 'idle',
        model: null,
        tokens: { inputTokens: 0, outputTokens: 0 },
        debugEnabled: false,
      });
      setRustStateSource(mock.source);

      // When I get the snapshot for the first time
      const snapshot1 = getSessionSnapshotForTesting('test-session-1');

      // Then it should fetch from Rust once
      expect(mock.getCallCounts().status).toBe(1);
      expect(mock.getCallCounts().model).toBe(1);
      expect(mock.getCallCounts().tokens).toBe(1);
      expect(mock.getCallCounts().debug).toBe(1);

      // When I get the snapshot again without refresh
      const snapshot2 = getSessionSnapshotForTesting('test-session-1');

      // Then it should NOT make additional Rust calls
      expect(mock.getCallCounts().status).toBe(1); // Same as before
      expect(mock.getCallCounts().model).toBe(1);
      expect(mock.getCallCounts().tokens).toBe(1);
      expect(mock.getCallCounts().debug).toBe(1);

      // And it should return the same object reference
      expect(snapshot2).toBe(snapshot1);
    });

    it('should return same reference on multiple rapid calls (simulating React behavior)', () => {
      // Given a mock state source
      const mock = createMockStateSource({
        status: 'running',
        model: { providerId: 'anthropic', modelId: 'claude-3' },
        tokens: { inputTokens: 100, outputTokens: 50 },
        debugEnabled: true,
      });
      setRustStateSource(mock.source);

      // When I get the snapshot multiple times (simulating React's multiple getSnapshot calls)
      const snapshot1 = getSessionSnapshotForTesting('test-session-2');
      const snapshot2 = getSessionSnapshotForTesting('test-session-2');
      const snapshot3 = getSessionSnapshotForTesting('test-session-2');

      // Then all references should be the same object
      expect(snapshot1).toBe(snapshot2);
      expect(snapshot2).toBe(snapshot3);

      // And only one set of Rust calls should have been made
      expect(mock.getCallCounts().status).toBe(1);
      expect(mock.getCallCounts().model).toBe(1);
      expect(mock.getCallCounts().tokens).toBe(1);
      expect(mock.getCallCounts().debug).toBe(1);
    });
  });

  // ===========================================================================
  // Scenario: Snapshot updates when refresh is called
  // ===========================================================================

  describe('Scenario: Snapshot updates when refresh is called', () => {
    it('should fetch fresh data and return new snapshot after refresh', () => {
      // Given a mock state source with initial state
      const mock = createMockStateSource({
        status: 'idle',
        model: null,
        tokens: { inputTokens: 0, outputTokens: 0 },
        debugEnabled: false,
      });
      setRustStateSource(mock.source);

      // When I get the initial snapshot
      const initialSnapshot = getSessionSnapshotForTesting('test-session-3');
      expect(initialSnapshot.status).toBe('idle');
      expect(initialSnapshot.isLoading).toBe(false);

      // And I update the mock state
      mock.setState({
        status: 'running',
        tokens: { inputTokens: 100, outputTokens: 50 },
      });

      // And I call refresh
      refreshSessionState('test-session-3');

      // And I get the snapshot again
      const newSnapshot = getSessionSnapshotForTesting('test-session-3');

      // Then the snapshot should have updated values
      expect(newSnapshot.status).toBe('running');
      expect(newSnapshot.isLoading).toBe(true);
      expect(newSnapshot.tokens.inputTokens).toBe(100);

      // And it should be a different object reference
      expect(newSnapshot).not.toBe(initialSnapshot);
    });

    it('should return same reference if data unchanged after refresh', () => {
      // Given a mock state source
      const mock = createMockStateSource({
        status: 'idle',
        model: null,
        tokens: { inputTokens: 0, outputTokens: 0 },
        debugEnabled: false,
      });
      setRustStateSource(mock.source);

      // When I get the initial snapshot
      const initialSnapshot = getSessionSnapshotForTesting('test-session-4');

      // And I call refresh WITHOUT changing the mock state
      refreshSessionState('test-session-4');

      // And I get the snapshot again
      const newSnapshot = getSessionSnapshotForTesting('test-session-4');

      // Then the snapshot should be the same object reference (optimization)
      expect(newSnapshot).toBe(initialSnapshot);
    });
  });

  // ===========================================================================
  // Scenario: Empty snapshot for null session
  // ===========================================================================

  describe('Scenario: Empty snapshot constant', () => {
    it('should provide a frozen empty snapshot', () => {
      // When I get the empty snapshot
      const emptySnapshot = getEmptySnapshotForTesting();

      // Then it should have default values
      expect(emptySnapshot.status).toBe('idle');
      expect(emptySnapshot.isLoading).toBe(false);
      expect(emptySnapshot.model).toBeNull();
      expect(emptySnapshot.tokens.inputTokens).toBe(0);
      expect(emptySnapshot.tokens.outputTokens).toBe(0);
      expect(emptySnapshot.isDebugEnabled).toBe(false);
      expect(emptySnapshot.version).toBe(0);

      // And it should be the same reference every time
      const emptySnapshot2 = getEmptySnapshotForTesting();
      expect(emptySnapshot).toBe(emptySnapshot2);
    });
  });

  // ===========================================================================
  // Scenario: Subscription management
  // ===========================================================================

  describe('Scenario: Subscription management', () => {
    it('should create subscription when session is first accessed', () => {
      const mock = createMockStateSource({
        status: 'idle',
        model: null,
        tokens: { inputTokens: 0, outputTokens: 0 },
        debugEnabled: false,
      });
      setRustStateSource(mock.source);

      // Before accessing, subscription should not exist
      expect(getSubscriptionForTesting('test-session-5')).toBeUndefined();

      // When I get the snapshot
      getSessionSnapshotForTesting('test-session-5');

      // Then a subscription should exist
      const sub = getSubscriptionForTesting('test-session-5');
      expect(sub).toBeDefined();
      expect(sub!.version).toBe(0);
      expect(sub!.cachedSnapshot).not.toBeNull();
    });

    it('should notify subscribers when refreshSessionState is called', () => {
      const mock = createMockStateSource({
        status: 'idle',
        model: null,
        tokens: { inputTokens: 0, outputTokens: 0 },
        debugEnabled: false,
      });
      setRustStateSource(mock.source);

      // Setup: create subscription and add a subscriber
      getSessionSnapshotForTesting('shared-session');
      const sub = getSubscriptionForTesting('shared-session');

      let notified = false;
      sub!.subscribers.add(() => {
        notified = true;
      });

      // When I call refreshSessionState
      refreshSessionState('shared-session');

      // Then the subscriber should have been notified
      expect(notified).toBe(true);

      // And the version should have incremented
      expect(sub!.version).toBe(1);
    });
  });

  // ===========================================================================
  // Scenario: Version tracking works correctly
  // ===========================================================================

  describe('Scenario: Version tracking for cache invalidation', () => {
    it('should increment version on each refresh call', () => {
      const mock = createMockStateSource({
        status: 'idle',
        model: null,
        tokens: { inputTokens: 0, outputTokens: 0 },
        debugEnabled: false,
      });
      setRustStateSource(mock.source);

      // Create subscription
      getSessionSnapshotForTesting('version-session');

      const sub = getSubscriptionForTesting('version-session');
      expect(sub!.version).toBe(0);

      // Call refresh multiple times
      refreshSessionState('version-session');
      expect(sub!.version).toBe(1);

      refreshSessionState('version-session');
      expect(sub!.version).toBe(2);

      refreshSessionState('version-session');
      expect(sub!.version).toBe(3);
    });

    it('should fetch fresh data only when version changes', () => {
      const mock = createMockStateSource({
        status: 'idle',
        model: null,
        tokens: { inputTokens: 0, outputTokens: 0 },
        debugEnabled: false,
      });
      setRustStateSource(mock.source);

      // Initial fetch: 1 set of calls
      getSessionSnapshotForTesting('fetch-tracking-session');
      expect(mock.getCallCounts().status).toBe(1);

      // Multiple gets without refresh: no additional calls
      getSessionSnapshotForTesting('fetch-tracking-session');
      getSessionSnapshotForTesting('fetch-tracking-session');
      getSessionSnapshotForTesting('fetch-tracking-session');
      expect(mock.getCallCounts().status).toBe(1);

      // Refresh: should trigger new fetch on next get
      refreshSessionState('fetch-tracking-session');
      getSessionSnapshotForTesting('fetch-tracking-session');
      expect(mock.getCallCounts().status).toBe(2);

      // More gets without refresh: no additional calls
      getSessionSnapshotForTesting('fetch-tracking-session');
      getSessionSnapshotForTesting('fetch-tracking-session');
      expect(mock.getCallCounts().status).toBe(2);
    });
  });

  // ===========================================================================
  // Scenario: Different sessions are independent
  // ===========================================================================

  describe('Scenario: Session independence', () => {
    it('should maintain separate caches for different sessions', () => {
      const mock = createMockStateSource({
        status: 'idle',
        model: null,
        tokens: { inputTokens: 0, outputTokens: 0 },
        debugEnabled: false,
      });
      setRustStateSource(mock.source);

      // Get snapshots for two different sessions
      const snapshotA = getSessionSnapshotForTesting('session-A');
      const snapshotB = getSessionSnapshotForTesting('session-B');

      // They should be different objects (different sessions)
      expect(snapshotA).not.toBe(snapshotB);

      // Refresh only session A
      mock.setState({ status: 'running' });
      refreshSessionState('session-A');

      const newSnapshotA = getSessionSnapshotForTesting('session-A');
      const newSnapshotB = getSessionSnapshotForTesting('session-B');

      // Session A should have new data
      expect(newSnapshotA.status).toBe('running');
      expect(newSnapshotA).not.toBe(snapshotA);

      // Session B should still return cached (old data, but same reference)
      expect(newSnapshotB).toBe(snapshotB);
    });
  });

  // ===========================================================================
  // Scenario: Snapshot data correctness
  // ===========================================================================

  describe('Scenario: Snapshot data correctness', () => {
    it('should correctly derive isLoading from status', () => {
      const mock = createMockStateSource({
        status: 'idle',
        model: null,
        tokens: { inputTokens: 0, outputTokens: 0 },
        debugEnabled: false,
      });
      setRustStateSource(mock.source);

      // Idle -> not loading
      const idleSnapshot = getSessionSnapshotForTesting('loading-test');
      expect(idleSnapshot.isLoading).toBe(false);

      // Running -> loading
      mock.setState({ status: 'running' });
      refreshSessionState('loading-test');
      const runningSnapshot = getSessionSnapshotForTesting('loading-test');
      expect(runningSnapshot.isLoading).toBe(true);

      // Back to idle -> not loading
      mock.setState({ status: 'idle' });
      refreshSessionState('loading-test');
      const backToIdleSnapshot = getSessionSnapshotForTesting('loading-test');
      expect(backToIdleSnapshot.isLoading).toBe(false);
    });

    it('should include all state fields in snapshot', () => {
      const mock = createMockStateSource({
        status: 'running',
        model: { providerId: 'anthropic', modelId: 'claude-sonnet-4' },
        tokens: { inputTokens: 1500, outputTokens: 750 },
        debugEnabled: true,
      });
      setRustStateSource(mock.source);

      const snapshot = getSessionSnapshotForTesting('full-state-test');

      expect(snapshot.status).toBe('running');
      expect(snapshot.isLoading).toBe(true);
      expect(snapshot.model).toEqual({
        providerId: 'anthropic',
        modelId: 'claude-sonnet-4',
      });
      expect(snapshot.tokens).toEqual({
        inputTokens: 1500,
        outputTokens: 750,
      });
      expect(snapshot.isDebugEnabled).toBe(true);
      expect(typeof snapshot.version).toBe('number');
    });
  });

  // ===========================================================================
  // Scenario: Edge cases and error handling
  // ===========================================================================

  describe('Scenario: Edge cases and error handling', () => {
    it('should handle refresh on non-existent session gracefully', () => {
      // Calling refresh on a session that was never accessed should not throw
      expect(() => {
        refreshSessionState('non-existent-session');
      }).not.toThrow();
    });

    it('should handle clearAllSubscriptions correctly', () => {
      const mock = createMockStateSource({
        status: 'idle',
        model: null,
        tokens: { inputTokens: 0, outputTokens: 0 },
        debugEnabled: false,
      });
      setRustStateSource(mock.source);

      // Create multiple subscriptions
      getSessionSnapshotForTesting('session-1');
      getSessionSnapshotForTesting('session-2');

      expect(getSubscriptionForTesting('session-1')).toBeDefined();
      expect(getSubscriptionForTesting('session-2')).toBeDefined();

      // Clear all
      clearAllSubscriptions();

      expect(getSubscriptionForTesting('session-1')).toBeUndefined();
      expect(getSubscriptionForTesting('session-2')).toBeUndefined();
    });

    it('should handle null model correctly in equality check', () => {
      const mock = createMockStateSource({
        status: 'idle',
        model: null,
        tokens: { inputTokens: 0, outputTokens: 0 },
        debugEnabled: false,
      });
      setRustStateSource(mock.source);

      // Get initial snapshot with null model
      const snapshot1 = getSessionSnapshotForTesting('null-model-test');
      expect(snapshot1.model).toBeNull();

      // Refresh without changing model
      refreshSessionState('null-model-test');
      const snapshot2 = getSessionSnapshotForTesting('null-model-test');

      // Should return same reference (data unchanged)
      expect(snapshot2).toBe(snapshot1);

      // Now set a model
      mock.setState({ model: { providerId: 'test', modelId: 'test-model' } });
      refreshSessionState('null-model-test');
      const snapshot3 = getSessionSnapshotForTesting('null-model-test');

      // Should be new reference
      expect(snapshot3).not.toBe(snapshot1);
      expect(snapshot3.model).toEqual({
        providerId: 'test',
        modelId: 'test-model',
      });
    });
  });

  // ===========================================================================
  // Scenario: Critical fix verification - cache check before Rust calls
  // ===========================================================================

  describe('Scenario: CRITICAL - Cache check happens before Rust calls', () => {
    it('should NOT call Rust when cache is valid (version match)', () => {
      // This is the critical bug fix - verify cache is checked FIRST
      const mock = createMockStateSource({
        status: 'idle',
        model: null,
        tokens: { inputTokens: 0, outputTokens: 0 },
        debugEnabled: false,
      });
      setRustStateSource(mock.source);

      // First call populates cache
      getSessionSnapshotForTesting('critical-test');
      expect(mock.getCallCounts().status).toBe(1);

      // Reset call counts to clearly see subsequent behavior
      mock.resetCallCounts();
      expect(mock.getCallCounts().status).toBe(0);

      // Multiple subsequent calls should NOT call Rust at all
      for (let i = 0; i < 10; i++) {
        getSessionSnapshotForTesting('critical-test');
      }

      // Should still be 0 - cache was used every time
      expect(mock.getCallCounts().status).toBe(0);
      expect(mock.getCallCounts().model).toBe(0);
      expect(mock.getCallCounts().tokens).toBe(0);
      expect(mock.getCallCounts().debug).toBe(0);
    });

    it('should call Rust exactly once per refresh cycle', () => {
      const mock = createMockStateSource({
        status: 'idle',
        model: null,
        tokens: { inputTokens: 0, outputTokens: 0 },
        debugEnabled: false,
      });
      setRustStateSource(mock.source);

      // Initial call
      getSessionSnapshotForTesting('refresh-cycle-test');
      mock.resetCallCounts();

      // Refresh increments version
      refreshSessionState('refresh-cycle-test');

      // First get after refresh should call Rust
      getSessionSnapshotForTesting('refresh-cycle-test');
      expect(mock.getCallCounts().status).toBe(1);

      // Subsequent gets should NOT call Rust
      getSessionSnapshotForTesting('refresh-cycle-test');
      getSessionSnapshotForTesting('refresh-cycle-test');
      expect(mock.getCallCounts().status).toBe(1); // Still just 1

      // Another refresh
      mock.resetCallCounts();
      refreshSessionState('refresh-cycle-test');
      getSessionSnapshotForTesting('refresh-cycle-test');
      expect(mock.getCallCounts().status).toBe(1); // Just 1 for this cycle
    });
  });
});
