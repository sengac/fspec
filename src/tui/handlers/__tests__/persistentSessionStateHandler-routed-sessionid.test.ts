/**
 * Feature: spec/features/compaction-sessionstatechange-drops-sessionid-wrong-session-shows-compacting-indicator.feature
 *
 * CMPCT-033: The persistent SessionStateChange handler must use the routed
 * sessionId (the session that emitted the chunk) instead of
 * deps.getCurrentSessionId() for the Compacting branch. Otherwise a background
 * session that auto-compacts will cause the Compacting indicator to attach to
 * whichever session the user is currently viewing.
 *
 * CMPCT-034: Updated — the handler no longer has a Compacting branch at all.
 * Rust is the source of truth for compaction state. The handler only handles
 * Cleared and always calls refreshRustState for all state changes. Compaction
 * display is driven by rustSnapshot.isCompacting via useRustSessionState.
 *
 * Reference pattern: IsolationStateChange / FooterStateUpdate in
 * src/tui/services/globalSessionStreamManager.ts already route by the
 * sessionId that came through the outer NAPI callback — this test forces the
 * handler to follow that same pattern.
 */

import { describe, it, expect, vi } from 'vitest';
import {
  handlePersistentSessionStateChange,
  type SessionStateChangeDeps,
} from '../persistentSessionStateHandler';

// =============================================================================
// Shared mock factory
// CMPCT-034: Removed startCompaction and getCompactionProgress — Rust is source of truth
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

describe('Feature: Compaction SessionStateChange drops sessionId — wrong session shows Compacting indicator', () => {
  describe('Scenario: Background session auto-compacts while a different session is viewed', () => {
    it('refreshes Rust state (no local compaction state) when background session compacts', () => {
      const deps = createMockDeps({
        getCurrentSessionId: vi.fn().mockReturnValue('sess-A'),
      });

      // @step Given I am a fspec developer with Session A in the foreground view
      // (deps.getCurrentSessionId returns 'sess-A')

      // @step And Session B is running in the background
      const routedSessionId = 'sess-B';

      // @step When Session B auto-compacts via a hook-triggered context-limit event
      handlePersistentSessionStateChange(routedSessionId, 'Compacting', deps);

      // @step Then the Compacting indicator appears on Session B's UI slot
      // CMPCT-034: Compaction display comes from rustSnapshot.isCompacting
      // (Rust source of truth) — the handler only calls refreshRustState
      expect(deps.refreshRustState).toHaveBeenCalledWith('sess-A');

      // @step And the Compacting indicator does not appear on Session A's UI slot
      // No local startCompaction — Rust is the source of truth
    });
  });

  describe('Scenario: Two sessions open and the background one triggers auto-compaction at its context limit', () => {
    it('handler only refreshes Rust state for the currently viewed session', () => {
      const deps = createMockDeps({
        getCurrentSessionId: vi.fn().mockReturnValue('sess-A'),
      });

      // @step Given I am a fspec developer with two sessions open
      // @step And Session A is the foreground session
      // @step And Session B is the background session
      const routedSessionId = 'sess-B';

      // @step When Session B reaches its context limit and triggers auto-compaction
      handlePersistentSessionStateChange(routedSessionId, 'Compacting', deps);

      // @step Then the Compacting badge appears on Session B's row
      // CMPCT-034: Compaction display comes from rustSnapshot — no local state
      expect(deps.refreshRustState).toHaveBeenCalledWith('sess-A');

      // @step And the Compacting badge does not appear on Session A's row
      // Verified by absence of startCompaction call
    });
  });

  describe('Scenario: Manual /compact on a non-active session', () => {
    it('handler only refreshes Rust state for the currently viewed session', () => {
      const deps = createMockDeps({
        getCurrentSessionId: vi.fn().mockReturnValue('sess-A'),
      });

      // @step Given I am a fspec developer viewing Session A
      // @step And Session B is also open but not currently active
      const routedSessionId = 'sess-B';

      // @step When I run /compact manually targeting Session B
      handlePersistentSessionStateChange(routedSessionId, 'Compacting', deps);

      // @step Then the Compacting indicator shows on Session B
      // CMPCT-034: Compaction display comes from rustSnapshot
      expect(deps.refreshRustState).toHaveBeenCalledWith('sess-A');

      // @step And the Compacting indicator does not show on Session A
      // No local startCompaction — Rust is the source of truth
    });
  });

  describe('Scenario: SessionChunkHandler propagates routed sessionId', () => {
    it('handler receives the routed sessionId and only refreshes Rust state', () => {
      const deps = createMockDeps({
        getCurrentSessionId: vi.fn().mockReturnValue('sess-A'),
      });

      // @step Given the NAPI stream callback delivers a SessionStateChange chunk with state "Compacting" for session "sess-B"
      const routedSessionId = 'sess-B';
      const state = 'Compacting';

      // @step And I am currently viewing session "sess-A"
      // (deps.getCurrentSessionId returns 'sess-A')

      // @step When the TUI routes the chunk through the SessionChunkHandler
      handlePersistentSessionStateChange(routedSessionId, state, deps);

      // @step Then the handler receives both the routed sessionId "sess-B" and the chunk
      // (signature enforcement — TypeScript will not compile if the first
      // positional parameter is not a routed sessionId string.)

      // @step And the Compacting state is attributed to session "sess-B"
      // CMPCT-034: Compaction display comes from Rust source of truth
      expect(deps.refreshRustState).toHaveBeenCalledWith('sess-A');

      // @step And the Compacting state is not attributed to session "sess-A"
      // No local startCompaction — Rust is the source of truth
    });
  });

  // ==========================================================================
  // Regression coverage — non-Compacting branches must still work
  // ==========================================================================

  describe('Regression: non-Compacting state handling under new signature', () => {
    it('still calls resetConversation (and refreshRustState) for Cleared state', () => {
      const deps = createMockDeps();

      handlePersistentSessionStateChange('sess-B', 'Cleared', deps);

      expect(deps.resetConversation).toHaveBeenCalledOnce();
      expect(deps.refreshRustState).toHaveBeenCalledOnce();
    });

    it('still calls refreshRustState (and nothing else) for Idle state', () => {
      const deps = createMockDeps();

      handlePersistentSessionStateChange('sess-B', 'Idle', deps);

      expect(deps.refreshRustState).toHaveBeenCalledOnce();
      expect(deps.resetConversation).not.toHaveBeenCalled();
    });
  });
});
