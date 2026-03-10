/**
 * Feature: spec/features/persistent-handler-state-refresh.feature
 *
 * Tests that the extracted handlePersistentSessionStateChange function
 * calls refreshRustState for ALL SessionStateChange events, and does NOT
 * call endCompaction for non-CompactionComplete states.
 *
 * BUG-101: The persistentChunkHandler returned early for SessionStateChange
 * chunks without calling refreshRustState, causing isLoading to stay true
 * after the agent finished.
 *
 * These tests use dependency injection to verify actual call behavior —
 * unlike the previous version which only tested the subscription layer
 * and passed even with the fix reverted.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  handlePersistentSessionStateChange,
  type SessionStateChangeDeps,
} from '../../handlers/persistentSessionStateHandler';

// =============================================================================
// Shared mock factory (DRY — single source for all tests)
// =============================================================================

function createMockDeps(
  overrides: Partial<SessionStateChangeDeps> = {}
): SessionStateChangeDeps {
  return {
    resetConversation: vi.fn(),
    startCompaction: vi.fn(),
    getCompactionProgress: vi.fn().mockReturnValue(null),
    refreshRustState: vi.fn(),
    getCurrentSessionId: vi.fn().mockReturnValue('test-session-123'),
    ...overrides,
  };
}

// =============================================================================
// Tests
// =============================================================================

describe('Feature: Persistent chunk handler refreshes React state on session state changes', () => {
  describe('Scenario: SessionStateChange(Idle) via persistent handler transitions isLoading to false', () => {
    it('should call refreshRustState when state is Idle', () => {
      // @step Given the streaming handler has been cleaned up after a Done chunk
      // (The persistent handler is the only active handler at this point)
      const deps = createMockDeps();

      // @step And the Rust session status transitions to Idle after apply_pending_dag
      // (The state string 'Idle' is what arrives in the chunk)

      // @step When the persistentChunkHandler receives a SessionStateChange with state Idle
      handlePersistentSessionStateChange('Idle', deps);

      // @step Then refreshRustState should be called for the current session
      expect(deps.refreshRustState).toHaveBeenCalledOnce();
      expect(deps.refreshRustState).toHaveBeenCalledWith('test-session-123');

      // @step And isLoading should transition to false
      // (Verified: refreshRustState was called, which triggers useSyncExternalStore
      // to fetch fresh state from Rust. When Rust status is Idle, isLoading=false.)
      // Also verify no other side effects were triggered:
      expect(deps.resetConversation).not.toHaveBeenCalled();
      expect(deps.startCompaction).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: SessionStateChange(Running) via persistent handler keeps isLoading true', () => {
    it('should call refreshRustState when state is Running', () => {
      // @step Given the streaming handler has been cleaned up
      const deps = createMockDeps();

      // @step And the Rust session status is Running during a compact flow

      // @step When the persistentChunkHandler receives a SessionStateChange with state Running
      handlePersistentSessionStateChange('Running', deps);

      // @step Then refreshRustState should be called for the current session
      expect(deps.refreshRustState).toHaveBeenCalledOnce();
      expect(deps.refreshRustState).toHaveBeenCalledWith('test-session-123');

      // @step And isLoading should remain true
      // (Verified: refreshRustState was called, which triggers useSyncExternalStore
      // to fetch fresh state from Rust. When Rust status is Running, isLoading=true.)
      expect(deps.resetConversation).not.toHaveBeenCalled();
      expect(deps.startCompaction).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: endCompaction guard preserved for non-CompactionComplete state changes', () => {
    it('should NOT call endCompaction when SessionStateChange(Idle) arrives during compaction', () => {
      // @step Given a compaction is in progress with isCompacting true
      // endCompaction is NOT in deps — it's intentionally excluded from the handler.
      // The handler has no way to call endCompaction because it's not a dependency.
      // Only CompactionComplete (handled separately in persistentChunkHandler) calls endCompaction.
      const deps = createMockDeps();

      // @step When the persistentChunkHandler receives a SessionStateChange with state Idle
      handlePersistentSessionStateChange('Idle', deps);

      // @step Then endCompaction should NOT be called
      // Verified structurally: endCompaction is NOT in SessionStateChangeDeps.
      // The handler cannot call it even if it wanted to.
      // The ONLY functions called are those in deps — verify exhaustively:
      expect(deps.refreshRustState).toHaveBeenCalledOnce();
      expect(deps.resetConversation).not.toHaveBeenCalled();
      expect(deps.startCompaction).not.toHaveBeenCalled();
      expect(deps.getCompactionProgress).not.toHaveBeenCalled();

      // @step And the compaction indicator should remain visible until CompactionComplete arrives
      // Verified: the handler only calls refreshRustState (which updates isLoading).
      // The compaction indicator is managed by compactionRef.current.endCompaction()
      // which is ONLY called from the CompactionComplete handler — a completely separate
      // code path in persistentChunkHandler.
    });
  });

  // ==========================================================================
  // Additional edge case tests (guard against regressions)
  // ==========================================================================

  describe('Edge cases', () => {
    it('should call resetConversation AND refreshRustState for Cleared state', () => {
      const deps = createMockDeps();

      handlePersistentSessionStateChange('Cleared', deps);

      expect(deps.resetConversation).toHaveBeenCalledOnce();
      expect(deps.refreshRustState).toHaveBeenCalledOnce();
      expect(deps.startCompaction).not.toHaveBeenCalled();
    });

    it('should start compaction tracking AND refreshRustState for Compacting state', () => {
      const progress = { phase: 'Building DAG', current: 1, total: 3 };
      const deps = createMockDeps({
        getCompactionProgress: vi.fn().mockReturnValue(progress),
      });

      handlePersistentSessionStateChange('Compacting', deps);

      expect(deps.startCompaction).toHaveBeenCalledOnce();
      expect(deps.startCompaction).toHaveBeenCalledWith(
        'hook-triggered',
        'test-session-123',
        progress
      );
      expect(deps.refreshRustState).toHaveBeenCalledOnce();
      expect(deps.resetConversation).not.toHaveBeenCalled();
    });

    it('should handle Compacting state with null session ID gracefully', () => {
      const deps = createMockDeps({
        getCurrentSessionId: vi.fn().mockReturnValue(null),
      });

      handlePersistentSessionStateChange('Compacting', deps);

      // startCompaction should NOT be called when session ID is null
      expect(deps.startCompaction).not.toHaveBeenCalled();
      // refreshRustState should still be called (with null)
      expect(deps.refreshRustState).toHaveBeenCalledOnce();
      expect(deps.refreshRustState).toHaveBeenCalledWith(null);
    });

    it('should call refreshRustState for any unknown state string', () => {
      const deps = createMockDeps();

      handlePersistentSessionStateChange('SomeUnknownState', deps);

      expect(deps.refreshRustState).toHaveBeenCalledOnce();
      expect(deps.resetConversation).not.toHaveBeenCalled();
      expect(deps.startCompaction).not.toHaveBeenCalled();
    });
  });
});
