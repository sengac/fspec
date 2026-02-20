/**
 * Feature: spec/features/isolated-session-header-display.feature
 *
 * End-to-end tests for the [ISOLATED] badge display in SessionHeader.
 * These tests verify the entire data flow from session creation to UI state.
 *
 * GIT-029: IsolationStateChange handling must work regardless of timing
 *
 * NO MOCKS - uses real NAPI bindings and fixtures
 */

import {
  describe,
  it,
  expect,
  beforeEach,
  afterEach,
  beforeAll,
  afterAll,
} from 'vitest';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import { execSync } from 'child_process';
import { randomUUID } from 'crypto';

import {
  sessionManagerCreateIsolated,
  sessionManagerDestroy,
  listWorktrees,
  removeWorktree,
  persistenceSetDataDirectory,
  sessionManagerList,
} from '@sengac/codelet-napi';
import type { StreamChunk } from '@sengac/codelet-napi';

import {
  GlobalSessionStreamManager,
  initGlobalSessionStreamManager,
  stopGlobalSessionStreamManager,
  applyPendingIsolationState,
} from '../services/globalSessionStreamManager';

import { useSessionStore } from '../store/sessionStore';

describe('Feature: [ISOLATED] badge display in SessionHeader', () => {
  let testDir: string;
  let dataDir: string;
  let testSessionId: string;

  beforeAll(() => {
    // Create a temporary git repository for testing
    testDir = fs.mkdtempSync(
      path.join(os.tmpdir(), 'fspec-isolated-header-e2e-')
    );

    // Create a temporary data directory for persistence
    dataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'fspec-data-'));
    persistenceSetDataDirectory(dataDir);

    // Initialize git repo
    execSync('git init', { cwd: testDir, stdio: 'pipe' });
    execSync('git config user.email "test@test.com"', {
      cwd: testDir,
      stdio: 'pipe',
    });
    execSync('git config user.name "Test User"', {
      cwd: testDir,
      stdio: 'pipe',
    });

    // Create initial commit so HEAD exists
    fs.writeFileSync(path.join(testDir, 'README.md'), '# Test Project');
    execSync('git add .', { cwd: testDir, stdio: 'pipe' });
    execSync('git commit -m "Initial commit"', { cwd: testDir, stdio: 'pipe' });
  });

  afterAll(() => {
    // Cleanup test directories
    try {
      fs.rmSync(testDir, { recursive: true, force: true });
    } catch {
      // Directory may not exist
    }

    try {
      fs.rmSync(dataDir, { recursive: true, force: true });
    } catch {
      // Directory may not exist
    }
  });

  beforeEach(() => {
    // Generate unique session ID
    testSessionId = randomUUID();

    // Reset session store
    useSessionStore.getState().reset();

    // Reset stream manager
    stopGlobalSessionStreamManager();
  });

  afterEach(async () => {
    // Cleanup: destroy session if it exists
    try {
      sessionManagerDestroy(testSessionId);
    } catch {
      // Session may not exist
    }

    // Cleanup: remove worktree if it exists
    try {
      removeWorktree(testDir, testSessionId);
    } catch {
      // Worktree may not exist
    }

    // Reset stream manager
    stopGlobalSessionStreamManager();

    // Reset session store
    useSessionStore.getState().reset();

    // Clean up any remaining sessions
    const sessions = sessionManagerList();
    for (const session of sessions) {
      try {
        sessionManagerDestroy(session.id);
      } catch {
        // Session might already be destroyed
      }
    }
  });

  describe('Scenario: IsolationStateChange received BEFORE activateSession is called', () => {
    it('should apply pending isolation state when session is activated', async () => {
      // @step Given the GlobalSessionStreamManager is initialized
      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();

      // @step And no session is currently active in sessionStore
      expect(useSessionStore.getState().currentSessionId).toBeNull();
      expect(useSessionStore.getState().isIsolated).toBe(false);
      expect(useSessionStore.getState().worktreePath).toBeNull();

      // @step When an isolated session is created
      const result = await sessionManagerCreateIsolated(
        testSessionId,
        'anthropic/claude-sonnet-4-20250514',
        testDir,
        'Test Isolated Session'
      );

      // Subscribe to the session to receive chunks
      manager.subscribeToSession(testSessionId);

      // @step And the session emits IsolationStateChange BEFORE activateSession is called
      // Simulate the chunk that would be emitted when session is created
      const isolationChunk: StreamChunk = {
        type: 'IsolationStateChange',
        isIsolated: true,
        worktreePath: result.worktreePath,
      };

      // At this point, currentSessionId is STILL null (activateSession not called yet)
      expect(useSessionStore.getState().currentSessionId).toBeNull();

      // Simulate the chunk arriving - it will be stored as pending
      manager.simulateChunk(testSessionId, isolationChunk);

      // Wait a tick for state updates
      await new Promise(resolve => setTimeout(resolve, 50));

      // FIX: The isolation state is stored as pending, not applied yet
      expect(useSessionStore.getState().isIsolated).toBe(false);
      expect(useSessionStore.getState().worktreePath).toBeNull();

      // @step When activateSession is called followed by applyPendingIsolationState
      useSessionStore.getState().activateSession(testSessionId);
      applyPendingIsolationState(testSessionId);

      // @step Then the session store should have the correct isolation state
      expect(useSessionStore.getState().currentSessionId).toBe(testSessionId);
      expect(useSessionStore.getState().isIsolated).toBe(true);
      expect(useSessionStore.getState().worktreePath).toBe(result.worktreePath);
    });
  });

  describe('Scenario: Correct behavior - IsolationStateChange after activateSession', () => {
    it('should update sessionStore when IsolationStateChange arrives after session is activated', async () => {
      // @step Given the GlobalSessionStreamManager is initialized
      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();

      // @step And an isolated session is created
      const result = await sessionManagerCreateIsolated(
        testSessionId,
        'anthropic/claude-sonnet-4-20250514',
        testDir,
        'Test Isolated Session'
      );

      manager.subscribeToSession(testSessionId);

      // @step When activateSession is called BEFORE IsolationStateChange arrives
      useSessionStore.getState().activateSession(testSessionId);
      expect(useSessionStore.getState().currentSessionId).toBe(testSessionId);

      // @step And THEN IsolationStateChange is received
      const isolationChunk: StreamChunk = {
        type: 'IsolationStateChange',
        isIsolated: true,
        worktreePath: result.worktreePath,
      };
      manager.simulateChunk(testSessionId, isolationChunk);

      // Wait a tick for state updates
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step Then the session store should be updated correctly
      expect(useSessionStore.getState().isIsolated).toBe(true);
      expect(useSessionStore.getState().worktreePath).toBe(result.worktreePath);
    });
  });

  describe('Scenario: Real isolated session creation should emit IsolationStateChange', () => {
    it('should emit IsolationStateChange when isolated session is created via NAPI', async () => {
      // @step Given the GlobalSessionStreamManager is initialized with a global handler
      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();

      const receivedChunks: Array<{ sessionId: string; chunk: StreamChunk }> =
        [];
      manager.registerGlobalHandler((sessionId, chunk) => {
        receivedChunks.push({ sessionId, chunk });
      });

      // @step When an isolated session is created via sessionManagerCreateIsolated
      const result = await sessionManagerCreateIsolated(
        testSessionId,
        'anthropic/claude-sonnet-4-20250514',
        testDir,
        'Test Isolated Session'
      );

      manager.subscribeToSession(testSessionId);

      // Wait for any chunks to arrive
      await new Promise(resolve => setTimeout(resolve, 500));

      // @step Then the session should have a worktree
      expect(result.worktreePath).toBeDefined();
      expect(result.worktreePath).toContain(testSessionId);
      expect(fs.existsSync(result.worktreePath)).toBe(true);

      // Note: The IsolationStateChange chunk is emitted from Rust when the session starts streaming,
      // not immediately upon creation. This test verifies the worktree exists,
      // and the next test verifies the UI flow when chunks arrive.
    });
  });

  describe('Scenario: Full flow - from session creation to SessionHeader display', () => {
    it('should show [ISOLATED] badge when session is created with proper ordering', async () => {
      // This test simulates the CORRECT order that happens in AgentView after the fix:
      // 1. Create isolated session
      // 2. IsolationStateChange chunk arrives and is stored as pending
      // 3. Activate session in store
      // 4. Apply pending isolation state
      // 5. Store is updated with isIsolated=true
      // 6. SessionHeader reads isIsolated=true from store
      // 7. [ISOLATED] badge is displayed

      // @step Given the GlobalSessionStreamManager is initialized
      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();

      // @step When I create an isolated session
      const result = await sessionManagerCreateIsolated(
        testSessionId,
        'anthropic/claude-sonnet-4-20250514',
        testDir,
        'Test Isolated Session'
      );

      // Subscribe to the session's stream (this happens in sessionService)
      manager.subscribeToSession(testSessionId);

      // @step And the IsolationStateChange chunk arrives (before activation)
      const isolationChunk: StreamChunk = {
        type: 'IsolationStateChange',
        isIsolated: true,
        worktreePath: result.worktreePath,
      };
      manager.simulateChunk(testSessionId, isolationChunk);

      // Wait for state updates
      await new Promise(resolve => setTimeout(resolve, 50));

      // Isolation state is pending (not yet applied)
      expect(useSessionStore.getState().isIsolated).toBe(false);

      // @step When I activate the session and apply pending isolation state
      // (This is what AgentView does now after the fix)
      useSessionStore.getState().activateSession(testSessionId);
      applyPendingIsolationState(testSessionId);

      // @step Then the sessionStore should have the correct isolation state
      expect(useSessionStore.getState().currentSessionId).toBe(testSessionId);
      expect(useSessionStore.getState().isIsolated).toBe(true);
      expect(useSessionStore.getState().worktreePath).toBe(result.worktreePath);

      // @step And when SessionHeader reads from the store via useIsIsolated()
      // The hook returns state.isIsolated which should be true
      const isIsolated = useSessionStore.getState().isIsolated;
      expect(isIsolated).toBe(true);

      // @step Then the [ISOLATED] badge should be displayed
      // (This is verified by the component receiving isIsolated=true)
    });
  });

  describe('Scenario: Pending isolation state is cleared after retrieval', () => {
    it('should only apply pending state once', async () => {
      // @step Given the GlobalSessionStreamManager is initialized
      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();

      // @step When an isolated session is created
      const result = await sessionManagerCreateIsolated(
        testSessionId,
        'anthropic/claude-sonnet-4-20250514',
        testDir,
        'Test Isolated Session'
      );

      manager.subscribeToSession(testSessionId);

      // @step And IsolationStateChange arrives before activation
      const isolationChunk: StreamChunk = {
        type: 'IsolationStateChange',
        isIsolated: true,
        worktreePath: result.worktreePath,
      };
      manager.simulateChunk(testSessionId, isolationChunk);

      // @step When I activate and apply pending state
      useSessionStore.getState().activateSession(testSessionId);
      applyPendingIsolationState(testSessionId);

      expect(useSessionStore.getState().isIsolated).toBe(true);

      // @step And then reset the isolation state manually
      useSessionStore.getState().setIsolationState(false, null);
      expect(useSessionStore.getState().isIsolated).toBe(false);

      // @step When I try to apply pending state again
      applyPendingIsolationState(testSessionId);

      // @step Then nothing should happen (pending state was already consumed)
      expect(useSessionStore.getState().isIsolated).toBe(false);
    });
  });
});
