/**
 * Feature: spec/features/debug-badge-session-awareness.feature
 *
 * Integration tests for BUG-133: [DEBUG] badge session-awareness.
 * Tests DebugStateChange stream event -> GSSM listener -> sessionStore flow.
 *
 * These tests use real NAPI bindings + real GSSM + real Zustand store,
 * mirroring the IsolationStateChange tests in globalSessionStreamManager.test.ts.
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
 * BUG-135: sessionStore now stores debug state as a per-session
 * Map<string, boolean>. Tests previously asserted the flat
 * `state.isDebugEnabled` field; this helper derives the current
 * session's value from the map so existing scenarios still pass.
 */
function currentDebugEnabled(): boolean {
  const state = useSessionStore.getState();
  const sid = state.currentSessionId;
  if (!sid) {
    return false;
  }
  return state.debugStateBySession.get(sid) ?? false;
}

describe('Feature: [DEBUG] badge in SessionHeader is not session-aware like [ISOLATED]', () => {
  let testSetup: WorkUnitTestSetup;

  beforeAll(async () => {
    testSetup = await setupWorkUnitTest('bug133-debug-badge');
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

  // ----------------------------------------
  // Scenario 1: Session switching
  // ----------------------------------------

  describe('Scenario: DEBUG badge reflects only the active session\'s debug state when switching sessions', () => {
    it('should show debug enabled only when session with debug is active', async () => {
      // @step Given session A has debug capture enabled
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

      // Activate session A and set its debug state to enabled via stream event
      useSessionStore.getState().activateSession(sessionA.id);
      const debugOnChunk: StreamChunk = {
        type: 'DebugStateChange',
        enabled: true,
      };
      manager.simulateChunk(sessionA.id, debugOnChunk);

      // @step And session B has debug capture disabled
      // Session B defaults to debug disabled (no DebugStateChange event sent)

      // @step When I switch to session A
      // Session A is already active
      // @step Then the SessionHeader should display the "[DEBUG]" badge
      expect(currentDebugEnabled()).toBe(true);

      // @step When I switch to session B
      useSessionStore.getState().activateSession(sessionB.id);

      // @step Then the SessionHeader should not display the "[DEBUG]" badge
      expect(currentDebugEnabled()).toBe(false);

      // Cleanup
      useSessionStore.getState().reset();
    });
  });

  // ----------------------------------------
  // Scenario 2: Toggle debug on fires stream event
  // ----------------------------------------

  describe('Scenario: Toggling debug fires a Rust stream event that updates Zustand store', () => {
    it('should update Zustand isDebugEnabled to true when DebugStateChange enabled=true is received', async () => {
      // @step Given session A is active with debug capture disabled
      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();

      const sessionA = persistenceCreateSessionWithProvider(
        'Session A',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );

      manager.subscribeToSession(sessionA.id);
      await new Promise(resolve => setTimeout(resolve, 100));

      useSessionStore.getState().activateSession(sessionA.id);
      expect(currentDebugEnabled()).toBe(false);

      // @step When I run the "/debug" command in session A
      // @step Then Rust should emit a DebugStateChange stream event with enabled true for session A
      const debugOnChunk: StreamChunk = {
        type: 'DebugStateChange',
        enabled: true,
      };
      manager.simulateChunk(sessionA.id, debugOnChunk);

      // @step And the Zustand sessionStore should contain isDebugEnabled true for session A
      expect(currentDebugEnabled()).toBe(true);

      // @step And the SessionHeader should display the "[DEBUG]" badge
      // (SessionHeader renders [DEBUG] when isDebugEnabled prop is true)

      // Cleanup
      useSessionStore.getState().reset();
    });
  });

  // ----------------------------------------
  // Scenario 3: Toggle debug off fires stream event
  // ----------------------------------------

  describe('Scenario: Disabling debug fires a stream event that removes the badge', () => {
    it('should update Zustand isDebugEnabled to false when DebugStateChange enabled=false is received', async () => {
      // @step Given session A is active with debug capture enabled
      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();

      const sessionA = persistenceCreateSessionWithProvider(
        'Session A',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );

      manager.subscribeToSession(sessionA.id);
      await new Promise(resolve => setTimeout(resolve, 100));

      useSessionStore.getState().activateSession(sessionA.id);

      // Enable debug first
      manager.simulateChunk(sessionA.id, {
        type: 'DebugStateChange',
        enabled: true,
      } as StreamChunk);
      expect(currentDebugEnabled()).toBe(true);

      // @step When I run the "/debug" command in session A
      // @step Then Rust should emit a DebugStateChange stream event with enabled false for session A
      const debugOffChunk: StreamChunk = {
        type: 'DebugStateChange',
        enabled: false,
      };
      manager.simulateChunk(sessionA.id, debugOffChunk);

      // @step And the Zustand sessionStore should contain isDebugEnabled false for session A
      expect(currentDebugEnabled()).toBe(false);

      // @step And the SessionHeader should not display the "[DEBUG]" badge

      // Cleanup
      useSessionStore.getState().reset();
    });
  });

  // ----------------------------------------
  // Scenario 4: Hydration on session attach
  // ----------------------------------------

  describe('Scenario: Debug state is hydrated from Rust when attaching to an existing session', () => {
    it('should seed Zustand with debug state when DebugStateChange arrives before activation', async () => {
      // @step Given session A previously had debug capture enabled
      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();

      const sessionA = persistenceCreateSessionWithProvider(
        'Session A - Previously Debug',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );

      manager.subscribeToSession(sessionA.id);
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step And session A is not currently attached in the TUI
      expect(useSessionStore.getState().currentSessionId).toBeNull();

      // DebugStateChange arrives before session is activated (pending state)
      const debugOnChunk: StreamChunk = {
        type: 'DebugStateChange',
        enabled: true,
      };
      manager.simulateChunk(sessionA.id, debugOnChunk);

      // Store should NOT have debug enabled yet (session not active)
      expect(currentDebugEnabled()).toBe(false);

      // @step When I attach to session A
      useSessionStore.getState().activateSession(sessionA.id);

      // Apply pending debug state (mirrors applyPendingIsolationState pattern)
      const { applyPendingDebugState } = await import(
        '../../services/globalSessionStreamManager'
      );
      applyPendingDebugState(sessionA.id);

      // @step Then the Zustand sessionStore should be seeded with isDebugEnabled true for session A
      expect(currentDebugEnabled()).toBe(true);

      // @step And the SessionHeader should display the "[DEBUG]" badge

      // Cleanup
      useSessionStore.getState().reset();
    });
  });
});

// =========================================================================
// Scenario 5: Source code structure verification
// =========================================================================

import fs from 'fs';
import path from 'path';

describe('Feature: [DEBUG] badge - Code cleanup verification', () => {
  describe('Scenario: Local React debug state and duplicate handler are removed', () => {
    const agentViewPath = path.join(__dirname, '..', 'AgentView.tsx');
    const agentViewSource = fs.readFileSync(agentViewPath, 'utf-8');

    it('should have no local useState for isDebugEnabled in AgentView', () => {
      // @step Given the AgentView component is rendered
      // @step Then there should be no local useState for isDebugEnabled in AgentView

      // The bug's root cause: const [isDebugEnabled, setIsDebugEnabled] = useState(false)
      expect(agentViewSource).not.toMatch(
        /const\s+\[isDebugEnabled,\s*setIsDebugEnabled\]\s*=\s*useState/
      );

      // Also ensure setIsDebugEnabled is not called anywhere (no local setter)
      expect(agentViewSource).not.toMatch(/setIsDebugEnabled\s*\(/);
    });

    it('should source isDebugEnabled from Zustand useIsDebugEnabled selector', () => {
      // @step And the isDebugEnabled prop to SessionHeader should be sourced from a Zustand useIsDebugEnabled selector

      // Must import useIsDebugEnabled from sessionStore
      expect(agentViewSource).toContain('useIsDebugEnabled');

      // Must NOT have the OR logic: displayIsDebugEnabled = rustSnapshot.isDebugEnabled || isDebugEnabled
      expect(agentViewSource).not.toMatch(
        /displayIsDebugEnabled\s*=\s*rustSnapshot\.isDebugEnabled\s*\|\|\s*isDebugEnabled/
      );

      // Must NOT have a displayIsDebugEnabled variable at all
      expect(agentViewSource).not.toContain('displayIsDebugEnabled');
    });

    it('should have exactly one /debug handler in AgentView', () => {
      // @step And there should be no duplicate "/debug" handler in AgentView

      // Count occurrences of the /debug command handler pattern
      const debugHandlerMatches = agentViewSource.match(
        /if\s*\(\s*userMessage\s*===\s*['"]\/debug['"]\s*\)/g
      );

      // Should have exactly 1 /debug handler (in handleSubmit only, not in handleSubmitWithCommand)
      expect(debugHandlerMatches).not.toBeNull();
      expect(debugHandlerMatches!.length).toBe(1);
    });
  });
});
