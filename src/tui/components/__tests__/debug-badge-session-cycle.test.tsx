/**
 * Feature: spec/features/debug-badge-session-cycle.feature
 *
 * Regression tests for BUG-135: [DEBUG] badge disappears when cycling
 * back to a previously-visited session via Shift+Left/Right.
 *
 * These tests assert the fix design from spec/attachments/BUG-135/design-analysis.md:
 *   - Zustand stores debug state as per-session Map<sessionId, boolean>
 *   - activateSession does NOT reset debug state
 *   - setDebugState(sessionId, enabled) writes into the per-session map
 *   - useIsDebugEnabled selector reads current session's entry
 *   - Each session retains its state across arbitrary numbers of switches
 */

import {
  describe,
  it,
  expect,
  beforeEach,
  beforeAll,
  afterAll,
  afterEach,
} from 'vitest';

import {
  persistenceSetDataDirectory,
  persistenceCreateSessionWithProvider,
  sessionManagerDestroy,
  sessionManagerList,
} from '@sengac/codelet-napi';
import type { StreamChunk } from '@sengac/codelet-napi';

import {
  setupWorkUnitTest,
  type WorkUnitTestSetup,
} from '../../../test-helpers/universal-test-setup';

import {
  GlobalSessionStreamManager,
  initGlobalSessionStreamManager,
  stopGlobalSessionStreamManager,
} from '../../services/globalSessionStreamManager';

import { useSessionStore } from '../../store/sessionStore';

/**
 * Read the current session's debug state from Zustand for assertions.
 * After BUG-135 fix this reads `debugStateBySession.get(currentSessionId)`.
 */
function currentDebugState(): boolean {
  const state = useSessionStore.getState();
  const sid = state.currentSessionId;
  if (!sid) {
    return false;
  }
  return state.debugStateBySession.get(sid) ?? false;
}

/**
 * Read a specific session's stored debug state (regardless of active session).
 */
function debugStateFor(sessionId: string): boolean {
  return (
    useSessionStore.getState().debugStateBySession.get(sessionId) ?? false
  );
}

describe('Feature: [DEBUG] badge disappears when cycling back to a previously-visited session via Shift+Left/Right', () => {
  let testSetup: WorkUnitTestSetup;

  beforeAll(async () => {
    testSetup = await setupWorkUnitTest('bug135-debug-badge-cycle');
    persistenceSetDataDirectory(testSetup.testDir);
  });

  afterAll(async () => {
    await testSetup.cleanup();
  });

  beforeEach(() => {
    stopGlobalSessionStreamManager();
    useSessionStore.getState().reset();
  });

  afterEach(() => {
    stopGlobalSessionStreamManager();

    const sessions = sessionManagerList();
    for (const session of sessions) {
      try {
        sessionManagerDestroy(session.id);
      } catch {
        // Session might already be destroyed
      }
    }
  });

  // --------------------------------------------------------------
  // Scenario 1: The headline regression — A → B → A
  // --------------------------------------------------------------
  describe('Scenario: DEBUG badge reappears when cycling back to a session that had debug enabled', () => {
    it('should restore the badge for session A after A→B→A via activateSession', async () => {
      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();

      const sessionA = persistenceCreateSessionWithProvider(
        'Session A - Debug ON',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );
      const sessionB = persistenceCreateSessionWithProvider(
        'Session B - Debug OFF',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );

      manager.subscribeToSession(sessionA.id);
      manager.subscribeToSession(sessionB.id);
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Given session A has debug capture enabled
      useSessionStore.getState().activateSession(sessionA.id);
      manager.simulateChunk(sessionA.id, {
        type: 'DebugStateChange',
        enabled: true,
      } as StreamChunk);

      // @step And session B has debug capture disabled
      // (no event for B — defaults to false)

      // @step And I am currently viewing session A with the [DEBUG] badge visible
      expect(currentDebugState()).toBe(true);

      // @step When I press Shift+Right to switch to session B
      useSessionStore.getState().activateSession(sessionB.id);

      // @step Then the [DEBUG] badge should not be visible
      expect(currentDebugState()).toBe(false);

      // @step When I press Shift+Left to switch back to session A
      useSessionStore.getState().activateSession(sessionA.id);

      // @step Then the [DEBUG] badge should be visible again
      expect(currentDebugState()).toBe(true);

      // And the per-session map still retains A's true value
      expect(debugStateFor(sessionA.id)).toBe(true);
      expect(debugStateFor(sessionB.id)).toBe(false);

      useSessionStore.getState().reset();
    });
  });

  // --------------------------------------------------------------
  // Scenario 2: Longer cycle with three sessions
  // --------------------------------------------------------------
  describe('Scenario: Each session retains its own debug state across multiple switches', () => {
    it('should preserve independent debug flags across A→B→C→B→A', async () => {
      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();

      const sessionA = persistenceCreateSessionWithProvider(
        'Session A',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );
      const sessionB = persistenceCreateSessionWithProvider(
        'Session B',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );
      const sessionC = persistenceCreateSessionWithProvider(
        'Session C',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );

      for (const s of [sessionA, sessionB, sessionC]) {
        manager.subscribeToSession(s.id);
      }
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Given session A has debug capture enabled
      useSessionStore.getState().activateSession(sessionA.id);
      manager.simulateChunk(sessionA.id, {
        type: 'DebugStateChange',
        enabled: true,
      } as StreamChunk);

      // @step And session B has debug capture disabled
      useSessionStore.getState().activateSession(sessionB.id);
      // leave B false

      // @step And session C has debug capture enabled
      useSessionStore.getState().activateSession(sessionC.id);
      manager.simulateChunk(sessionC.id, {
        type: 'DebugStateChange',
        enabled: true,
      } as StreamChunk);

      // @step When I cycle through sessions A, B, C, B, A using Shift+Right and Shift+Left
      const cycle = [sessionA.id, sessionB.id, sessionC.id, sessionB.id, sessionA.id];
      const observed: Record<string, boolean[]> = {
        [sessionA.id]: [],
        [sessionB.id]: [],
        [sessionC.id]: [],
      };
      for (const id of cycle) {
        useSessionStore.getState().activateSession(id);
        observed[id].push(currentDebugState());
      }

      // @step Then session A should always show the [DEBUG] badge when active
      expect(observed[sessionA.id].every(v => v === true)).toBe(true);
      // @step And session B should never show the [DEBUG] badge when active
      expect(observed[sessionB.id].every(v => v === false)).toBe(true);
      // @step And session C should always show the [DEBUG] badge when active
      expect(observed[sessionC.id].every(v => v === true)).toBe(true);

      useSessionStore.getState().reset();
    });
  });

  // --------------------------------------------------------------
  // Scenario 3: Independent toggles — B unaffected by A
  // --------------------------------------------------------------
  describe('Scenario: Toggling debug on one session does not affect other sessions state', () => {
    it('should leave session B unchanged when session A debug is toggled off', async () => {
      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();

      const sessionA = persistenceCreateSessionWithProvider(
        'Session A',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );
      const sessionB = persistenceCreateSessionWithProvider(
        'Session B',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );

      manager.subscribeToSession(sessionA.id);
      manager.subscribeToSession(sessionB.id);
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Given session A has debug capture enabled
      useSessionStore.getState().activateSession(sessionA.id);
      manager.simulateChunk(sessionA.id, {
        type: 'DebugStateChange',
        enabled: true,
      } as StreamChunk);
      expect(debugStateFor(sessionA.id)).toBe(true);

      // @step And session B has debug capture enabled
      useSessionStore.getState().activateSession(sessionB.id);
      manager.simulateChunk(sessionB.id, {
        type: 'DebugStateChange',
        enabled: true,
      } as StreamChunk);
      expect(debugStateFor(sessionB.id)).toBe(true);

      // @step When I run the "/debug" command in session A to disable debug capture
      // (Rust emits DebugStateChange enabled=false for A)
      useSessionStore.getState().activateSession(sessionA.id);
      manager.simulateChunk(sessionA.id, {
        type: 'DebugStateChange',
        enabled: false,
      } as StreamChunk);

      // @step Then session A should not show the [DEBUG] badge
      expect(currentDebugState()).toBe(false);
      expect(debugStateFor(sessionA.id)).toBe(false);

      // @step When I press Shift+Right to switch to session B
      useSessionStore.getState().activateSession(sessionB.id);

      // @step Then session B should still show the [DEBUG] badge
      expect(currentDebugState()).toBe(true);
      expect(debugStateFor(sessionB.id)).toBe(true);

      useSessionStore.getState().reset();
    });
  });

  // --------------------------------------------------------------
  // Scenario 4: Hydration via applyPendingDebugState + Rust ground-truth fallback
  // --------------------------------------------------------------
  describe('Scenario: DEBUG badge appears when attaching to a session that already has debug enabled in Rust', () => {
    it('should seed the per-session map from a pending DebugStateChange on attach', async () => {
      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();

      const sessionA = persistenceCreateSessionWithProvider(
        'Session A - Pre-enabled',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );

      manager.subscribeToSession(sessionA.id);
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Given session A exists in Rust with is_debug_enabled true
      // Event arrives before activation — should either buffer via pending map
      // or be directly written into debugStateBySession for session A.
      manager.simulateChunk(sessionA.id, {
        type: 'DebugStateChange',
        enabled: true,
      } as StreamChunk);

      // @step And the TUI has no pending debug state for session A
      // (After the event is handled the per-session map entry for A must exist.)

      // @step When I attach to session A in the TUI
      useSessionStore.getState().activateSession(sessionA.id);
      const { applyPendingDebugState } = await import(
        '../../services/globalSessionStreamManager'
      );
      applyPendingDebugState(sessionA.id);

      // @step Then the Zustand store should be seeded with debug enabled for session A
      expect(debugStateFor(sessionA.id)).toBe(true);

      // @step And the [DEBUG] badge should be visible
      expect(currentDebugState()).toBe(true);

      useSessionStore.getState().reset();
    });
  });

  // --------------------------------------------------------------
  // Scenario 5: Stream event for non-active session populates map
  // --------------------------------------------------------------
  describe('Scenario: Debug stream event for an inactive session updates its stored state', () => {
    it('should record debug state for an inactive session without affecting the current view', async () => {
      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();

      const sessionA = persistenceCreateSessionWithProvider(
        'Session A - Background',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );
      const sessionB = persistenceCreateSessionWithProvider(
        'Session B - Foreground',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );

      manager.subscribeToSession(sessionA.id);
      manager.subscribeToSession(sessionB.id);
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Given I am currently viewing session B
      useSessionStore.getState().activateSession(sessionB.id);
      expect(currentDebugState()).toBe(false);

      // @step And session A has debug capture disabled
      expect(debugStateFor(sessionA.id)).toBe(false);

      // @step When Rust emits a DebugStateChange event with enabled true for session A
      manager.simulateChunk(sessionA.id, {
        type: 'DebugStateChange',
        enabled: true,
      } as StreamChunk);

      // @step Then the Zustand store should record debug enabled true for session A
      expect(debugStateFor(sessionA.id)).toBe(true);

      // @step And the [DEBUG] badge should not be visible on session B
      expect(currentDebugState()).toBe(false);

      // @step When I press Shift+Left to switch to session A
      useSessionStore.getState().activateSession(sessionA.id);

      // @step Then the [DEBUG] badge should be visible without any additional refresh
      expect(currentDebugState()).toBe(true);

      useSessionStore.getState().reset();
    });
  });
});
