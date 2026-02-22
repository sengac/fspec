/**
 * Session Service Integration Test Fixture
 *
 * TUI-068: E2E test fixture for session service integration testing.
 * Uses REAL NAPI bindings - no mocks except for external dependencies.
 *
 * SOLID: Single Responsibility - Only handles session service test setup
 * DRY: Reusable across multiple test files
 * COMPOSABLE: Can be combined with other fixtures
 */

import { randomUUID } from 'crypto';
import { mkdir, writeFile, rm } from 'fs/promises';
import { existsSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';

import { useFspecStore } from '../../../store/fspecStore';
import { useSessionStore } from '../../../store/sessionStore';
import { GlobalSessionStreamManager } from '../../globalSessionStreamManager';

/**
 * Test project structure for session tests
 */
interface TestProjectStructure {
  testDir: string;
  specDir: string;
  workUnitsFile: string;
  prefixesFile: string;
}

/**
 * Store state snapshot for assertions
 */
interface StoreStateSnapshot {
  sessionAttachments: Map<string, string>;
  currentWorkUnitId: string | null;
  currentWorkUnitStatus: string | null;
}

/**
 * Session service fixture state
 */
export interface SessionServiceFixture {
  project: TestProjectStructure;
  getStoreState: () => StoreStateSnapshot;
  resetStores: () => void;
  attachSession: (workUnitId: string, sessionId: string) => void;
  detachSession: (workUnitId: string) => void;
  setCurrentWorkUnit: (
    workUnitId: string | null,
    status: string | null
  ) => void;
  getAttachedSession: (workUnitId: string) => string | undefined;
  getWorkUnitBySession: (sessionId: string) => string | undefined;
  createWorkUnit: (id: string, title: string, status: string) => Promise<void>;
  cleanup: () => Promise<void>;
}

/**
 * Creates an fspec project structure for testing
 */
async function createTestProjectStructure(
  testName: string
): Promise<TestProjectStructure> {
  const testDir = join(
    tmpdir(),
    `fspec-session-${testName}-${randomUUID().slice(0, 8)}`
  );
  const specDir = join(testDir, 'spec');

  await mkdir(specDir, { recursive: true });
  await mkdir(join(specDir, 'features'), { recursive: true });

  const workUnitsFile = join(specDir, 'work-units.json');
  const prefixesFile = join(specDir, 'prefixes.json');

  // Create minimal work units structure
  await writeFile(
    workUnitsFile,
    JSON.stringify(
      {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        workUnits: {},
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      },
      null,
      2
    )
  );

  // Create prefixes file
  await writeFile(
    prefixesFile,
    JSON.stringify(
      {
        prefixes: {
          TEST: { description: 'Test prefix' },
          AUTH: { description: 'Authentication features' },
        },
      },
      null,
      2
    )
  );

  return {
    testDir,
    specDir,
    workUnitsFile,
    prefixesFile,
  };
}

/**
 * Creates a session service fixture for integration testing.
 *
 * This fixture provides:
 * - Real fspec project structure
 * - Real Zustand store state management
 * - Clean state reset between tests
 * - Proper cleanup on teardown
 *
 * @example
 * ```typescript
 * describe('Session Service Integration', () => {
 *   let fixture: SessionServiceFixture;
 *
 *   beforeEach(async () => {
 *     fixture = await createSessionServiceFixture('my-test');
 *   });
 *
 *   afterEach(async () => {
 *     await fixture.cleanup();
 *   });
 *
 *   it('should attach session to work unit', () => {
 *     fixture.attachSession('TOOL-014', 'session-123');
 *     expect(fixture.getAttachedSession('TOOL-014')).toBe('session-123');
 *   });
 * });
 * ```
 */
export async function createSessionServiceFixture(
  testName: string
): Promise<SessionServiceFixture> {
  const project = await createTestProjectStructure(testName);

  // Reset stores to clean state
  const resetStores = () => {
    useFspecStore.setState({
      sessionAttachments: new Map(),
    });
    useSessionStore.getState().setCurrentWorkUnit(null, null);
  };

  // Initialize with clean state
  resetStores();

  const getStoreState = (): StoreStateSnapshot => {
    const fspecState = useFspecStore.getState();
    const sessionState = useSessionStore.getState();

    return {
      sessionAttachments: new Map(fspecState.sessionAttachments),
      currentWorkUnitId: sessionState.currentWorkUnitId,
      currentWorkUnitStatus: sessionState.currentWorkUnitStatus,
    };
  };

  const attachSession = (workUnitId: string, sessionId: string) => {
    useFspecStore.getState().attachSession(workUnitId, sessionId);
  };

  const detachSession = (workUnitId: string) => {
    useFspecStore.getState().detachSession(workUnitId);
  };

  const setCurrentWorkUnit = (
    workUnitId: string | null,
    status: string | null
  ) => {
    useSessionStore.getState().setCurrentWorkUnit(workUnitId, status);
  };

  const getAttachedSession = (workUnitId: string): string | undefined => {
    return useFspecStore.getState().getAttachedSession(workUnitId);
  };

  const getWorkUnitBySession = (sessionId: string): string | undefined => {
    return useFspecStore.getState().getWorkUnitBySession(sessionId);
  };

  const createWorkUnit = async (
    id: string,
    title: string,
    status: string
  ): Promise<void> => {
    const workUnitsData = JSON.parse(
      await import('fs/promises').then(fs =>
        fs.readFile(project.workUnitsFile, 'utf-8')
      )
    );

    workUnitsData.workUnits[id] = {
      id,
      title,
      status,
      type: 'story',
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };

    workUnitsData.states[status].push(id);

    await writeFile(
      project.workUnitsFile,
      JSON.stringify(workUnitsData, null, 2)
    );
  };

  const cleanup = async () => {
    // Reset stores
    resetStores();

    // Remove temp directory
    if (existsSync(project.testDir)) {
      await rm(project.testDir, { recursive: true, force: true });
    }
  };

  return {
    project,
    getStoreState,
    resetStores,
    attachSession,
    detachSession,
    setCurrentWorkUnit,
    getAttachedSession,
    getWorkUnitBySession,
    createWorkUnit,
    cleanup,
  };
}

/**
 * Extended fixture with NAPI session support
 *
 * Use this when you need actual NAPI session creation/destruction.
 * Requires valid credentials to be set up.
 */
export interface SessionServiceNapiFixture extends SessionServiceFixture {
  createNapiSession: (name?: string) => Promise<string>;
  destroyNapiSession: (sessionId: string) => void;
  destroyAllSessions: () => void;
  getCreatedSessionIds: () => string[];
}

/**
 * Creates a session service fixture with real NAPI support.
 *
 * WARNING: This creates real sessions in the NAPI layer.
 * Ensure proper cleanup in afterEach/afterAll.
 */
export async function createSessionServiceNapiFixture(
  testName: string
): Promise<SessionServiceNapiFixture> {
  const baseFixture = await createSessionServiceFixture(testName);
  const createdSessionIds: string[] = [];

  // Set up persistence directory
  const { persistenceSetDataDirectory } = await import('@sengac/codelet-napi');
  persistenceSetDataDirectory(baseFixture.project.testDir);

  const createNapiSession = async (name = 'Test Session'): Promise<string> => {
    const { sessionManagerCreateWithId } = await import('@sengac/codelet-napi');
    const sessionId = randomUUID();

    try {
      await sessionManagerCreateWithId(
        sessionId,
        'anthropic/claude-sonnet-4',
        baseFixture.project.testDir,
        name
      );
    } catch {
      // Session creation may fail due to invalid API key, but session ID still exists
    }

    createdSessionIds.push(sessionId);
    return sessionId;
  };

  const destroyNapiSession = (sessionId: string) => {
    import('@sengac/codelet-napi')
      .then(({ sessionManagerDestroy }) => {
        try {
          sessionManagerDestroy(sessionId);
          const idx = createdSessionIds.indexOf(sessionId);
          if (idx !== -1) {
            createdSessionIds.splice(idx, 1);
          }
        } catch {
          // Ignore destroy errors
        }
      })
      .catch(() => {});
  };

  const destroyAllSessions = () => {
    import('@sengac/codelet-napi')
      .then(({ sessionManagerDestroy }) => {
        for (const id of [...createdSessionIds]) {
          try {
            sessionManagerDestroy(id);
          } catch {
            // Ignore
          }
        }
        createdSessionIds.length = 0;
      })
      .catch(() => {});
  };

  const cleanup = async () => {
    // Destroy all created sessions
    destroyAllSessions();

    // Wait for async cleanup
    await new Promise(resolve => setTimeout(resolve, 100));

    // Call base cleanup
    await baseFixture.cleanup();
  };

  return {
    ...baseFixture,
    createNapiSession,
    destroyNapiSession,
    destroyAllSessions,
    getCreatedSessionIds: () => [...createdSessionIds],
    cleanup,
  };
}
