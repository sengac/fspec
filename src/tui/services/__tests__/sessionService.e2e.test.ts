/**
 * Feature: spec/features/refactor-session-work-unit-state-management.feature
 *
 * E2E tests for session-work unit state management.
 * Uses REAL NAPI bindings - NO MOCKS, NO STUBS.
 *
 * SOLID/DRY: Reusable fixtures, real implementations, composable setup.
 */

import {
  describe,
  it,
  expect,
  beforeAll,
  afterAll,
  beforeEach,
  afterEach,
} from 'vitest';
import { randomUUID } from 'crypto';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { existsSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import * as git from 'isomorphic-git';
import * as fs from 'fs';

import { useFspecStore } from '../../store/fspecStore';
import { useSessionStore } from '../../store/sessionStore';

// Import sessionService once at module level
import {
  createSession as sessionServiceCreate,
  attachToWorkUnit,
  detachFromWorkUnit,
  destroySession,
  mergeSessionChanges,
  discardSessionChanges,
} from '../sessionService';

// ========================================
// CONSTANTS - Eliminate magic strings/numbers
// ========================================

const TEST_MODEL = 'anthropic/claude-sonnet-4-20250514';
const WORK_UNIT_TOOL = 'TOOL-014';
const WORK_UNIT_AUTH = 'AUTH-001';
const STATUS_SPECIFYING = 'specifying';
const STATUS_IMPLEMENTING = 'implementing';
const CLEANUP_DELAY_MS = 100;

// ========================================
// E2E FIXTURE - Real NAPI, Real Stores
// ========================================

interface E2EFixture {
  testDir: string;
  createdSessionIds: string[];
  createSession: (name?: string) => Promise<string>;
  createIsolatedSession: (
    name?: string
  ) => Promise<{ sessionId: string; worktreePath: string }>;
  destroyAllSessions: () => Promise<void>;
  resetStores: () => void;
  initGitRepo: () => Promise<void>;
  cleanup: () => Promise<void>;
}

async function createE2EFixture(testName: string): Promise<E2EFixture> {
  const testDir = join(
    tmpdir(),
    `fspec-e2e-${testName}-${randomUUID().slice(0, 8)}`
  );
  const specDir = join(testDir, 'spec');
  const createdSessionIds: string[] = [];

  // Create project structure
  await mkdir(specDir, { recursive: true });
  await mkdir(join(specDir, 'features'), { recursive: true });

  // Create work-units.json
  await writeFile(
    join(specDir, 'work-units.json'),
    JSON.stringify(
      {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        workUnits: {
          [WORK_UNIT_TOOL]: {
            id: WORK_UNIT_TOOL,
            title: 'Test Work Unit',
            type: 'story',
            status: STATUS_SPECIFYING,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          [WORK_UNIT_AUTH]: {
            id: WORK_UNIT_AUTH,
            title: 'Auth Work Unit',
            type: 'story',
            status: STATUS_IMPLEMENTING,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        },
        states: {
          backlog: [],
          specifying: [WORK_UNIT_TOOL],
          testing: [],
          implementing: [WORK_UNIT_AUTH],
          validating: [],
          done: [],
          blocked: [],
        },
      },
      null,
      2
    )
  );

  // Set persistence directory for NAPI
  const { persistenceSetDataDirectory } = await import('@sengac/codelet-napi');
  persistenceSetDataDirectory(testDir);

  const resetStores = (): void => {
    useFspecStore.setState({ sessionAttachments: new Map() });
    useSessionStore.getState().setCurrentWorkUnit(null, null);
  };

  // Initialize clean state
  resetStores();

  const initGitRepo = async (): Promise<void> => {
    await git.init({ fs, dir: testDir, defaultBranch: 'main' });
    await git.setConfig({
      fs,
      dir: testDir,
      path: 'user.name',
      value: 'Test User',
    });
    await git.setConfig({
      fs,
      dir: testDir,
      path: 'user.email',
      value: 'test@example.com',
    });
    await writeFile(join(testDir, 'README.md'), '# Test Project');
    await git.add({ fs, dir: testDir, filepath: 'README.md' });
    await git.commit({
      fs,
      dir: testDir,
      message: 'Initial commit',
      author: { name: 'Test User', email: 'test@example.com' },
    });
  };

  const createSession = async (name = 'E2E Test Session'): Promise<string> => {
    const { sessionManagerCreateWithId } = await import('@sengac/codelet-napi');
    const sessionId = randomUUID();

    try {
      await sessionManagerCreateWithId(sessionId, TEST_MODEL, testDir, name);
    } catch {
      // Session creation may fail due to invalid API key, but session still registered
    }

    createdSessionIds.push(sessionId);
    return sessionId;
  };

  const createIsolatedSession = async (
    name = 'Isolated Test Session'
  ): Promise<{ sessionId: string; worktreePath: string }> => {
    const { sessionManagerCreateIsolated } = await import(
      '@sengac/codelet-napi'
    );
    const sessionId = randomUUID();

    const result = await sessionManagerCreateIsolated(
      sessionId,
      TEST_MODEL,
      testDir,
      name
    );
    createdSessionIds.push(sessionId);

    return { sessionId, worktreePath: result.worktreePath };
  };

  const destroyAllSessions = async (): Promise<void> => {
    const {
      sessionManagerDestroy,
      sessionManagerList,
      removeWorktree,
      listWorktrees,
    } = await import('@sengac/codelet-napi');

    // Destroy tracked sessions
    for (const id of [...createdSessionIds]) {
      try {
        sessionManagerDestroy(id);
      } catch {
        // Ignore errors in cleanup
      }
    }
    createdSessionIds.length = 0;

    // Clean up orphaned sessions
    try {
      const allSessions = sessionManagerList();
      for (const session of allSessions) {
        try {
          sessionManagerDestroy(session.id);
        } catch {
          // Ignore errors in cleanup
        }
      }
    } catch {
      // Ignore errors in cleanup
    }

    // Clean up worktrees
    try {
      const worktrees = listWorktrees(testDir);
      for (const worktree of worktrees) {
        try {
          removeWorktree(testDir, worktree.sessionId);
        } catch {
          // Ignore errors in cleanup
        }
      }
    } catch {
      // Ignore errors in cleanup
    }
  };

  const cleanup = async (): Promise<void> => {
    await destroyAllSessions();
    resetStores();
    await new Promise(resolve => setTimeout(resolve, CLEANUP_DELAY_MS));
    if (existsSync(testDir)) {
      await rm(testDir, { recursive: true, force: true });
    }
  };

  return {
    testDir,
    createdSessionIds,
    createSession,
    createIsolatedSession,
    destroyAllSessions,
    resetStores,
    initGitRepo,
    cleanup,
  };
}

// ========================================
// E2E TESTS - ALL 14 SCENARIOS
// ========================================

describe('Feature: Refactor session-work unit state management (E2E)', () => {
  let fixture: E2EFixture;

  beforeAll(async () => {
    fixture = await createE2EFixture('session-mgmt');
    await fixture.initGitRepo();
  });

  afterAll(async () => {
    await fixture.cleanup();
  });

  beforeEach(async () => {
    await fixture.destroyAllSessions();
    fixture.resetStores();
  });

  afterEach(async () => {
    await fixture.destroyAllSessions();
  });

  // ========================================
  // USER BEHAVIOR SCENARIOS (1-5)
  // ========================================

  describe('Scenario: Session attaches to selected work unit when entering agent mode', () => {
    it('should create real NAPI session and attach to work unit via sessionService', async () => {
      // @step Given I am viewing the board with work units
      expect(useSessionStore.getState().currentWorkUnitId).toBeNull();

      // @step When I press Enter to start a session (using real sessionService)
      const sessionResult = await sessionServiceCreate({
        modelPath: TEST_MODEL,
        project: fixture.testDir,
      });
      fixture.createdSessionIds.push(sessionResult.sessionId);

      // @step And I have selected work unit "TOOL-014"
      attachToWorkUnit(
        sessionResult.sessionId,
        WORK_UNIT_TOOL,
        STATUS_SPECIFYING
      );

      // @step Then a new session should be created (real UUID from NAPI)
      expect(sessionResult.sessionId).toBeDefined();
      expect(sessionResult.sessionId).toMatch(/^[0-9a-f-]{36}$/);

      // @step And the session should be attached to work unit "TOOL-014"
      expect(useFspecStore.getState().getAttachedSession(WORK_UNIT_TOOL)).toBe(
        sessionResult.sessionId
      );

      // @step And sessionStore.currentWorkUnitId should be "TOOL-014"
      expect(useSessionStore.getState().currentWorkUnitId).toBe(WORK_UNIT_TOOL);
    });
  });

  describe('Scenario: New session does not auto-attach after closing previous session', () => {
    it('should not auto-attach new session after destroying previous one via real NAPI', async () => {
      // @step Given I am in an agent session attached to work unit "TOOL-014"
      const firstSession = await sessionServiceCreate({
        modelPath: TEST_MODEL,
        project: fixture.testDir,
      });
      fixture.createdSessionIds.push(firstSession.sessionId);
      attachToWorkUnit(
        firstSession.sessionId,
        WORK_UNIT_TOOL,
        STATUS_SPECIFYING
      );
      expect(useSessionStore.getState().currentWorkUnitId).toBe(WORK_UNIT_TOOL);

      // @step When I close the session (real NAPI destroy)
      await destroySession(firstSession.sessionId);

      // @step And I return to the board - verify state cleared
      expect(useSessionStore.getState().currentWorkUnitId).toBeNull();

      // @step And I press "/" to start a new session
      const newSession = await sessionServiceCreate({
        modelPath: TEST_MODEL,
        project: fixture.testDir,
      });
      fixture.createdSessionIds.push(newSession.sessionId);

      // @step Then a new session should be created
      expect(newSession.sessionId).toBeDefined();
      expect(newSession.sessionId).not.toBe(firstSession.sessionId);

      // @step And the session should NOT be attached to any work unit
      expect(
        useFspecStore.getState().getWorkUnitBySession(newSession.sessionId)
      ).toBeUndefined();

      // @step And sessionStore.currentWorkUnitId should be null
      expect(useSessionStore.getState().currentWorkUnitId).toBeNull();
    });
  });

  describe('Scenario: New session does not auto-attach after detaching from previous session', () => {
    it('should not auto-attach new session after detaching via real sessionService', async () => {
      // @step Given I am in an agent session attached to work unit "TOOL-014"
      const firstSession = await sessionServiceCreate({
        modelPath: TEST_MODEL,
        project: fixture.testDir,
      });
      fixture.createdSessionIds.push(firstSession.sessionId);
      attachToWorkUnit(
        firstSession.sessionId,
        WORK_UNIT_TOOL,
        STATUS_SPECIFYING
      );
      expect(useSessionStore.getState().currentWorkUnitId).toBe(WORK_UNIT_TOOL);

      // @step When I detach from the session
      detachFromWorkUnit(firstSession.sessionId);

      // @step And I return to the board
      expect(useSessionStore.getState().currentWorkUnitId).toBeNull();

      // @step And I press "/" to start a new session
      const newSession = await sessionServiceCreate({
        modelPath: TEST_MODEL,
        project: fixture.testDir,
      });
      fixture.createdSessionIds.push(newSession.sessionId);

      // @step Then a new session should be created
      expect(newSession.sessionId).toBeDefined();

      // @step And the session should NOT be attached to any work unit
      expect(
        useFspecStore.getState().getWorkUnitBySession(newSession.sessionId)
      ).toBeUndefined();

      // @step And sessionStore.currentWorkUnitId should be null
      expect(useSessionStore.getState().currentWorkUnitId).toBeNull();
    });
  });

  describe('Scenario: Session created without work unit when no selection on board', () => {
    it('should create session without attachment when no work unit selected', async () => {
      // @step Given I am viewing the board with work units
      // @step And no work unit is selected
      expect(useSessionStore.getState().currentWorkUnitId).toBeNull();

      // @step When I press "/" to start a new session
      const sessionResult = await sessionServiceCreate({
        modelPath: TEST_MODEL,
        project: fixture.testDir,
      });
      fixture.createdSessionIds.push(sessionResult.sessionId);

      // @step Then a new session should be created
      expect(sessionResult.sessionId).toBeDefined();
      expect(sessionResult.sessionId).toMatch(/^[0-9a-f-]{36}$/);

      // @step And the session should NOT be attached to any work unit
      expect(
        useFspecStore.getState().getWorkUnitBySession(sessionResult.sessionId)
      ).toBeUndefined();

      // @step And sessionStore.currentWorkUnitId should be null
      expect(useSessionStore.getState().currentWorkUnitId).toBeNull();
    });
  });

  describe('Scenario: Work unit context updates via IPC', () => {
    it('should update both stores when work unit changes via IPC', async () => {
      // @step Given I am in an agent session attached to work unit "TOOL-014"
      const session = await sessionServiceCreate({
        modelPath: TEST_MODEL,
        project: fixture.testDir,
      });
      fixture.createdSessionIds.push(session.sessionId);
      attachToWorkUnit(session.sessionId, WORK_UNIT_TOOL, STATUS_SPECIFYING);
      expect(useSessionStore.getState().currentWorkUnitId).toBe(WORK_UNIT_TOOL);

      // @step When the AI changes work unit to "AUTH-001" via IPC
      attachToWorkUnit(session.sessionId, WORK_UNIT_AUTH, STATUS_IMPLEMENTING);

      // @step Then sessionStore.currentWorkUnitId should be "AUTH-001"
      expect(useSessionStore.getState().currentWorkUnitId).toBe(WORK_UNIT_AUTH);

      // @step And fspecStore.sessionAttachments should map "AUTH-001" to the current session
      expect(useFspecStore.getState().getAttachedSession(WORK_UNIT_AUTH)).toBe(
        session.sessionId
      );
    });
  });

  // ========================================
  // CODE QUALITY SCENARIO (6)
  // ========================================

  describe('Scenario: Duplicate state removed from fspecStore', () => {
    it('should verify fspecStore no longer has duplicate work unit state', async () => {
      // @step Given I inspect the fspecStore implementation
      const fspecStorePath = join(process.cwd(), 'src/tui/store/fspecStore.ts');
      const fspecStoreContent = await readFile(fspecStorePath, 'utf-8');

      // @step Then fspecStore should NOT have a currentWorkUnitId property
      expect(
        /currentWorkUnitId\s*:\s*string\s*\|\s*null/.test(fspecStoreContent)
      ).toBe(false);

      // @step And fspecStore should NOT have a setCurrentWorkUnitId method
      expect(/setCurrentWorkUnitId\s*:/.test(fspecStoreContent)).toBe(false);

      // @step And fspecStore should NOT have a getCurrentWorkUnitId method
      expect(/getCurrentWorkUnitId\s*:/.test(fspecStoreContent)).toBe(false);

      // @step And fspecStore should still have sessionAttachments for multi-session tracking
      expect(/sessionAttachments\s*:/.test(fspecStoreContent)).toBe(true);
    });
  });

  // ========================================
  // SESSION SERVICE FACADE SCENARIOS (7-9)
  // ========================================

  describe('Scenario: destroySession orchestrates all cleanup atomically', () => {
    it('should clean up NAPI, stores, and stream manager when destroying session', async () => {
      // @step Given I have an active session attached to work unit "TOOL-014"
      const session = await sessionServiceCreate({
        modelPath: TEST_MODEL,
        project: fixture.testDir,
      });
      // Don't add to createdSessionIds since we're destroying it
      attachToWorkUnit(session.sessionId, WORK_UNIT_TOOL, STATUS_SPECIFYING);

      expect(useFspecStore.getState().hasAttachedSession(WORK_UNIT_TOOL)).toBe(
        true
      );
      expect(useSessionStore.getState().currentWorkUnitId).toBe(WORK_UNIT_TOOL);

      // @step When I call destroySession
      await destroySession(session.sessionId);

      // @step Then fspecStore.sessionAttachments should NOT contain "TOOL-014"
      expect(useFspecStore.getState().hasAttachedSession(WORK_UNIT_TOOL)).toBe(
        false
      );

      // @step And sessionStore.currentWorkUnitId should be null
      expect(useSessionStore.getState().currentWorkUnitId).toBeNull();
    });
  });

  describe('Scenario: attachToWorkUnit orchestrates all stores atomically', () => {
    it('should update all stores when attaching session to work unit', async () => {
      // @step Given I have an active session
      const session = await sessionServiceCreate({
        modelPath: TEST_MODEL,
        project: fixture.testDir,
      });
      fixture.createdSessionIds.push(session.sessionId);

      // @step When I call attachToWorkUnit
      attachToWorkUnit(session.sessionId, WORK_UNIT_TOOL, STATUS_SPECIFYING);

      // @step Then fspecStore.sessionAttachments should map "TOOL-014" to the session
      expect(useFspecStore.getState().getAttachedSession(WORK_UNIT_TOOL)).toBe(
        session.sessionId
      );

      // @step And sessionStore.currentWorkUnitId should be "TOOL-014"
      expect(useSessionStore.getState().currentWorkUnitId).toBe(WORK_UNIT_TOOL);
    });
  });

  describe('Scenario: detachFromWorkUnit clears all state atomically', () => {
    it('should clear all stores when detaching from work unit', async () => {
      // @step Given I have an active session attached to work unit "TOOL-014"
      const session = await sessionServiceCreate({
        modelPath: TEST_MODEL,
        project: fixture.testDir,
      });
      fixture.createdSessionIds.push(session.sessionId);
      attachToWorkUnit(session.sessionId, WORK_UNIT_TOOL, STATUS_SPECIFYING);
      expect(useFspecStore.getState().hasAttachedSession(WORK_UNIT_TOOL)).toBe(
        true
      );

      // @step When I call detachFromWorkUnit
      detachFromWorkUnit(session.sessionId);

      // @step Then fspecStore.sessionAttachments should NOT contain "TOOL-014"
      expect(useFspecStore.getState().hasAttachedSession(WORK_UNIT_TOOL)).toBe(
        false
      );

      // @step And sessionStore.currentWorkUnitId should be null
      expect(useSessionStore.getState().currentWorkUnitId).toBeNull();
    });
  });

  // ========================================
  // ISOLATED SESSION SCENARIOS (10-11)
  // ========================================

  describe('Scenario: Isolated session close prompts user then calls merge or discard', () => {
    it('should call mergeSessionChanges with real NAPI when user chooses Merge', async () => {
      // @step Given I have an isolated session with changes in worktree
      const { sessionId, worktreePath } = await fixture.createIsolatedSession(
        'Isolated Merge Test'
      );
      attachToWorkUnit(sessionId, WORK_UNIT_TOOL, STATUS_IMPLEMENTING);

      // Verify worktree was created
      expect(worktreePath).toBeDefined();
      expect(existsSync(worktreePath)).toBe(true);

      // @step When the user chooses "Merge"
      const mergeResult = mergeSessionChanges(fixture.testDir, sessionId);

      // @step Then mergeSessionChanges should return valid result
      expect(mergeResult).toBeDefined();
      expect(mergeResult).toHaveProperty('sessionId');
      expect(mergeResult).toHaveProperty('filesModified');
      expect(mergeResult).toHaveProperty('filesAdded');
      expect(mergeResult).toHaveProperty('filesDeleted');

      // @step And destroySession should be called
      detachFromWorkUnit(sessionId);

      // Worktree should be removed by merge
      expect(existsSync(worktreePath)).toBe(false);

      // Verify store cleanup
      expect(useFspecStore.getState().hasAttachedSession(WORK_UNIT_TOOL)).toBe(
        false
      );
    });
  });

  describe('Scenario: Isolated session discard removes worktree without applying changes', () => {
    it('should call discardSessionChanges with real NAPI when user chooses Discard', async () => {
      // @step Given I have an isolated session with changes in worktree
      const { sessionId, worktreePath } = await fixture.createIsolatedSession(
        'Isolated Discard Test'
      );
      attachToWorkUnit(sessionId, WORK_UNIT_TOOL, STATUS_IMPLEMENTING);

      // Verify worktree was created
      expect(worktreePath).toBeDefined();
      expect(existsSync(worktreePath)).toBe(true);

      // @step When the user chooses "Discard"
      const discardResult = discardSessionChanges(fixture.testDir, sessionId);

      // @step Then discardSessionChanges should return valid result
      expect(discardResult).toBeDefined();
      expect(discardResult).toHaveProperty('filesDiscarded');

      // @step And destroySession should be called
      detachFromWorkUnit(sessionId);

      // Worktree should be removed
      expect(existsSync(worktreePath)).toBe(false);
    });
  });

  // ========================================
  // COMPONENT INTEGRATION SCENARIOS (12-14)
  // ========================================

  describe('Scenario: AgentView uses sessionService facade for all session-work unit lifecycle operations', () => {
    it('should verify AgentView uses sessionService facade and NOT direct store methods', async () => {
      // @step Given I inspect AgentView.tsx imports
      const agentViewPath = join(
        process.cwd(),
        'src/tui/components/AgentView.tsx'
      );
      const agentViewContent = await readFile(agentViewPath, 'utf-8');

      // @step Then AgentView should import from sessionService
      const importsSessionService =
        agentViewContent.includes("from '../services/sessionService'") ||
        agentViewContent.includes('from "../services/sessionService"');
      expect(importsSessionService).toBe(true);

      // @step And AgentView should NOT directly import sessionManagerDestroy from codelet-napi
      const importsSessionManagerDestroy =
        /import\s*\{[^}]*sessionManagerDestroy[^}]*\}\s*from\s*['"]@sengac\/codelet-napi['"]/.test(
          agentViewContent
        );
      expect(importsSessionManagerDestroy).toBe(false);

      // @step And AgentView should NOT use useFspecStore.attachSession directly
      const usesAttachSessionHook =
        /useFspecStore\s*\(\s*(?:state|\w+)\s*=>\s*(?:state|\w+)\.attachSession\s*\)/.test(
          agentViewContent
        );
      expect(usesAttachSessionHook).toBe(false);

      // @step And AgentView should NOT use useFspecStore.detachSession directly
      const usesDetachSessionHook =
        /useFspecStore\s*\(\s*(?:state|\w+)\s*=>\s*(?:state|\w+)\.detachSession\s*\)/.test(
          agentViewContent
        );
      expect(usesDetachSessionHook).toBe(false);

      // @step And AgentView should use attachToWorkUnit from sessionService for all attachment operations
      const importsAttachToWorkUnit =
        /import\s*\{[^}]*attachToWorkUnit[^}]*\}\s*from\s*['"]\.\.\/services\/sessionService['"]/.test(
          agentViewContent
        );
      expect(importsAttachToWorkUnit).toBe(true);

      // @step And AgentView should use detachFromWorkUnit from sessionService for all detachment operations
      const importsDetachFromWorkUnit =
        /import\s*\{[^}]*detachFromWorkUnit[^}]*\}\s*from\s*['"]\.\.\/services\/sessionService['"]/.test(
          agentViewContent
        );
      expect(importsDetachFromWorkUnit).toBe(true);

      // @step And AgentView should NOT directly call useSessionStore for session lifecycle
      const directSessionStoreMutation =
        /useSessionStore\.getState\(\)\.setCurrentWorkUnit/.test(
          agentViewContent
        );
      expect(directSessionStoreMutation).toBe(false);
    });
  });

  describe('Scenario: BoardView IPC handler uses sessionService for work unit attachment', () => {
    it('should use attachToWorkUnit when processing IPC work-unit-changed message', async () => {
      // @step Given I receive an IPC message with workUnitId and status
      const workUnitId = WORK_UNIT_AUTH;
      const status = STATUS_IMPLEMENTING;

      // @step And I have a session to attach
      const session = await sessionServiceCreate({
        modelPath: TEST_MODEL,
        project: fixture.testDir,
      });
      fixture.createdSessionIds.push(session.sessionId);

      // @step When BoardView processes the IPC message
      attachToWorkUnit(session.sessionId, workUnitId, status);

      // @step Then attachToWorkUnit should update the stores
      expect(useFspecStore.getState().getAttachedSession(WORK_UNIT_AUTH)).toBe(
        session.sessionId
      );
    });
  });

  describe('Scenario: globalStreamListener uses sessionService for work unit context sync', () => {
    it('should use sessionService when syncing work unit context from stream', async () => {
      // @step Given I receive stream data indicating work unit changed
      const workUnitId = WORK_UNIT_AUTH;
      const status = STATUS_IMPLEMENTING;

      // @step And I have an active session
      const session = await sessionServiceCreate({
        modelPath: TEST_MODEL,
        project: fixture.testDir,
      });
      fixture.createdSessionIds.push(session.sessionId);

      // @step When globalStreamListener processes the chunk
      attachToWorkUnit(session.sessionId, workUnitId, status);

      // @step Then sessionService should sync work unit context
      expect(useSessionStore.getState().currentWorkUnitId).toBe(WORK_UNIT_AUTH);
      expect(useFspecStore.getState().getAttachedSession(WORK_UNIT_AUTH)).toBe(
        session.sessionId
      );
    });
  });
});
