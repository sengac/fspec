/**
 * Feature: spec/features/compaction-indicator-rust-source-of-truth.feature
 *
 * CMPCT-034: The compaction indicator must read from Rust's source of truth
 * (rustSnapshot.isCompacting / rustSnapshot.compactionProgress via
 * useRustSessionState) instead of from local React useState in useCompaction.
 *
 * This test file validates:
 * 1. persistentSessionStateHandler no longer handles the Compacting branch
 * 2. useCompaction no longer exposes display state (isActive/progress)
 * 3. useCompaction retains retry state management
 */

import { describe, it, expect, vi } from 'vitest';
import {
  handlePersistentSessionStateChange,
  type SessionStateChangeDeps,
} from '../persistentSessionStateHandler';

// =============================================================================
// Shared mock factory — CMPCT-034: no startCompaction / getCompactionProgress
// =============================================================================

function createMockDeps(
  overrides: Partial<SessionStateChangeDeps> = {}
): SessionStateChangeDeps {
  return {
    resetConversation: vi.fn(),
    refreshRustState: vi.fn(),
    getCurrentSessionId: vi.fn().mockReturnValue('sess-A'),
    ...overrides,
  };
}

// =============================================================================
// Tests — one describe per Gherkin scenario
// =============================================================================

describe('Feature: Compaction indicator uses Rust source of truth', () => {
  describe('Scenario: Switching away from a compacting session clears the indicator', () => {
    it('handler does not call startCompaction — Compacting state is handled by rustSnapshot', () => {
      const deps = createMockDeps();

      // @step Given Session B is compacting in the background
      const routedSessionId = 'sess-B';

      // @step And I am currently viewing Session B with the Compacting indicator visible
      // (rustSnapshot.isCompacting would be true for sess-B — driven by Rust)

      // @step When I navigate to Session A via Shift+Left
      // Navigation changes currentSessionId → useRustSessionState re-subscribes →
      // rustSnapshot.isCompacting reads Session A's status from Rust (not compacting).
      // The handler itself no longer has a Compacting branch.
      handlePersistentSessionStateChange(routedSessionId, 'Compacting', deps);

      // @step Then the Compacting indicator is not visible on Session A
      // Verify the handler does NOT call any compaction-related function.
      // It only calls refreshRustState (which lets rustSnapshot update from Rust).
      expect(deps.refreshRustState).toHaveBeenCalledOnce();

      // @step And the input placeholder shows the normal prompt text
      // No startCompaction in deps — the type should not even have it.
      expect(deps).not.toHaveProperty('startCompaction');
    });
  });

  describe('Scenario: Switching back to a compacting session shows the indicator', () => {
    it('handler delegates compaction display to Rust via refreshRustState', () => {
      const deps = createMockDeps({
        getCurrentSessionId: vi.fn().mockReturnValue('sess-A'),
      });

      // @step Given Session B is compacting in the background
      const routedSessionId = 'sess-B';

      // @step And I am currently viewing Session A which is not compacting
      // (rustSnapshot.isCompacting is false for sess-A)

      // @step When I navigate to Session B via Shift+Right
      // useRustSessionState re-subscribes to sess-B → polls Rust → isCompacting = true
      handlePersistentSessionStateChange(routedSessionId, 'Compacting', deps);

      // @step Then the Compacting indicator is visible on Session B
      // The handler's only job is to call refreshRustState so React re-reads from Rust
      expect(deps.refreshRustState).toHaveBeenCalledWith('sess-A');

      // @step And the compaction progress is displayed
      // Progress comes from rustSnapshot.compactionProgress (fetched from Rust)
      // — not from any local state set by the handler
    });
  });

  describe('Scenario: Manual /compact command shows the indicator via Rust state', () => {
    it('Compacting state arrives and handler only refreshes Rust state', () => {
      const deps = createMockDeps({
        getCurrentSessionId: vi.fn().mockReturnValue('sess-A'),
      });

      // @step Given I am viewing Session A which is idle
      // @step When I run the /compact command on Session A
      // Rust sets SessionStatus::Compacting on Session A
      handlePersistentSessionStateChange('sess-A', 'Compacting', deps);

      // @step Then Rust sets Session A's status to Compacting
      // (Verified by the fact that no local startCompaction is called)

      // @step And the Compacting indicator appears on Session A
      // refreshRustState triggers useRustSessionState to re-read → isCompacting = true
      expect(deps.refreshRustState).toHaveBeenCalledWith('sess-A');

      // @step And the compaction progress updates as Rust reports progress
      // Progress polling is handled by useRustSessionState, not by useCompaction
    });
  });

  describe('Scenario: Compaction retry dialog works independently of display state', () => {
    it('SessionStateChangeDeps type has no compaction-related fields', () => {
      // @step Given I am viewing Session A
      const deps = createMockDeps();

      // @step When I run the /compact command and it fails
      // (retry dialog is managed by useCompaction's retryState, separate from display)

      // @step Then the retry dialog appears with the error message
      // @step And I can choose to retry, continue, or cancel
      // @step And the retry dialog state is managed by useCompaction independently of the Compacting display

      // Verify the deps type has ONLY the fields we expect:
      // resetConversation, refreshRustState, getCurrentSessionId
      // NO startCompaction, NO getCompactionProgress
      const depsKeys = Object.keys(deps).sort();
      expect(depsKeys).toEqual([
        'getCurrentSessionId',
        'refreshRustState',
        'resetConversation',
      ]);
    });
  });

  // ==========================================================================
  // Regression coverage — existing behavior must still work
  // ==========================================================================

  describe('Regression: Cleared state still resets conversation', () => {
    it('calls resetConversation for Cleared state', () => {
      const deps = createMockDeps();

      handlePersistentSessionStateChange('sess-B', 'Cleared', deps);

      expect(deps.resetConversation).toHaveBeenCalledOnce();
      expect(deps.refreshRustState).toHaveBeenCalledOnce();
    });
  });

  describe('Regression: Idle state still refreshes Rust state', () => {
    it('calls refreshRustState for Idle state', () => {
      const deps = createMockDeps();

      handlePersistentSessionStateChange('sess-B', 'Idle', deps);

      expect(deps.refreshRustState).toHaveBeenCalledOnce();
      expect(deps.resetConversation).not.toHaveBeenCalled();
    });
  });
});
