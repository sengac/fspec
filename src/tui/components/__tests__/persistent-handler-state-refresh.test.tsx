/**
 * Feature: spec/features/persistent-handler-state-refresh.feature
 *
 * Tests that the extracted handlePersistentSessionStateChange function
 * calls refreshRustState for ALL SessionStateChange events.
 *
 * BUG-101: The persistentChunkHandler returned early for SessionStateChange
 * chunks without calling refreshRustState, causing isLoading to stay true
 * after the agent finished.
 *
 * CMPCT-034: The handler no longer has a Compacting branch — Rust is the
 * source of truth for compaction state. startCompaction and getCompactionProgress
 * have been removed from SessionStateChangeDeps.
 *
 * These tests use dependency injection to verify actual call behavior —
 * unlike the previous version which only tested the subscription layer
 * and passed even with the fix reverted.
 */

import { describe, it, expect, vi } from 'vitest';
import {
  handlePersistentSessionStateChange,
  type SessionStateChangeDeps,
} from '../../handlers/persistentSessionStateHandler';

// =============================================================================
// Shared mock factory (DRY — single source for all tests)
// CMPCT-034: Removed startCompaction and getCompactionProgress
// =============================================================================

function createMockDeps(
  overrides: Partial<SessionStateChangeDeps> = {}
): SessionStateChangeDeps {
  return {
    resetConversation: vi.fn(),
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
      const deps = createMockDeps();

      // @step And the Rust session status transitions to Idle after apply_pending_dag

      // @step When the persistentChunkHandler receives a SessionStateChange with state Idle
      handlePersistentSessionStateChange('test-session-123', 'Idle', deps);

      // @step Then refreshRustState should be called for the current session
      expect(deps.refreshRustState).toHaveBeenCalledOnce();
      expect(deps.refreshRustState).toHaveBeenCalledWith('test-session-123');

      // @step And isLoading should transition to false
      expect(deps.resetConversation).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: SessionStateChange(Running) via persistent handler keeps isLoading true', () => {
    it('should call refreshRustState when state is Running', () => {
      // @step Given the streaming handler has been cleaned up
      const deps = createMockDeps();

      // @step And the Rust session status is Running during a compact flow

      // @step When the persistentChunkHandler receives a SessionStateChange with state Running
      handlePersistentSessionStateChange('test-session-123', 'Running', deps);

      // @step Then refreshRustState should be called for the current session
      expect(deps.refreshRustState).toHaveBeenCalledOnce();
      expect(deps.refreshRustState).toHaveBeenCalledWith('test-session-123');

      // @step And isLoading should remain true
      expect(deps.resetConversation).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: endCompaction guard preserved for non-CompactionComplete state changes', () => {
    it('should NOT call endCompaction when SessionStateChange(Idle) arrives during compaction', () => {
      // @step Given a compaction is in progress with isCompacting true
      // CMPCT-034: endCompaction is not even in deps anymore — Rust manages compaction state.
      const deps = createMockDeps();

      // @step When the persistentChunkHandler receives a SessionStateChange with state Idle
      handlePersistentSessionStateChange('test-session-123', 'Idle', deps);

      // @step Then endCompaction should NOT be called
      // Verified structurally: endCompaction is NOT in SessionStateChangeDeps.
      expect(deps.refreshRustState).toHaveBeenCalledOnce();
      expect(deps.resetConversation).not.toHaveBeenCalled();

      // @step And the compaction indicator should remain visible until CompactionComplete arrives
      // CMPCT-034: Compaction indicator is driven by rustSnapshot.isCompacting from Rust.
      // When Rust sets SessionStatus::Idle on CompactionComplete, rustSnapshot updates.
    });
  });

  // ==========================================================================
  // Additional edge case tests (guard against regressions)
  // ==========================================================================

  describe('Edge cases', () => {
    it('should call resetConversation AND refreshRustState for Cleared state', () => {
      const deps = createMockDeps();

      handlePersistentSessionStateChange('test-session-123', 'Cleared', deps);

      expect(deps.resetConversation).toHaveBeenCalledOnce();
      expect(deps.refreshRustState).toHaveBeenCalledOnce();
    });

    it('CMPCT-034: Compacting state only triggers refreshRustState (no local state update)', () => {
      const deps = createMockDeps();

      handlePersistentSessionStateChange(
        'test-session-123',
        'Compacting',
        deps
      );

      // CMPCT-034: No startCompaction call — Rust is the source of truth.
      // refreshRustState triggers useRustSessionState to re-read isCompacting from Rust.
      expect(deps.refreshRustState).toHaveBeenCalledOnce();
      expect(deps.resetConversation).not.toHaveBeenCalled();
    });

    it('should handle Compacting state with empty routed sessionId gracefully', () => {
      const deps = createMockDeps({
        getCurrentSessionId: vi.fn().mockReturnValue(null),
      });

      handlePersistentSessionStateChange('', 'Compacting', deps);

      // CMPCT-034: No startCompaction — just refreshRustState with null (current-viewed id)
      expect(deps.refreshRustState).toHaveBeenCalledOnce();
      expect(deps.refreshRustState).toHaveBeenCalledWith(null);
    });

    it('should call refreshRustState for any unknown state string', () => {
      const deps = createMockDeps();

      handlePersistentSessionStateChange(
        'test-session-123',
        'SomeUnknownState',
        deps
      );

      expect(deps.refreshRustState).toHaveBeenCalledOnce();
      expect(deps.resetConversation).not.toHaveBeenCalled();
    });
  });
});
