/**
 * Global Chunk Callback Fixture
 *
 * BRIDGE-012: E2E test fixture for global chunk callback architecture.
 * Uses GlobalSessionStreamManager to receive chunks (not a separate callback).
 *
 * SOLID: Single Responsibility - Only handles test setup and cleanup
 * DRY: Reusable across multiple test files
 */

import { randomUUID } from 'crypto';
import { mkdir, writeFile, rm } from 'fs/promises';
import { existsSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import type { StreamChunk } from '@sengac/codelet-napi';

import { GlobalSessionStreamManager } from '../../globalSessionStreamManager';

/**
 * Received chunk with session context
 */
export interface ReceivedChunk {
  sessionId: string;
  chunk: StreamChunk;
  timestamp: number;
}

/**
 * Test session info
 */
export interface TestSession {
  id: string;
  name: string;
  destroy: () => void;
}

/**
 * Session factory for creating test sessions
 */
export interface SessionFactory {
  createSession: (name?: string) => Promise<TestSession>;
  createSessionWithId: (id: string, name?: string) => Promise<TestSession>;
  destroySession: (sessionId: string) => void;
  destroyAllSessions: () => void;
  getCreatedSessionIds: () => string[];
}

/**
 * Global chunk callback fixture state
 */
export interface GlobalChunkCallbackFixture {
  testDir: string;
  receivedChunks: ReceivedChunk[];
  getChunksForSession: (sessionId: string) => ReceivedChunk[];
  getChunksByType: (type: string) => ReceivedChunk[];
  clearChunks: () => void;
  waitForChunk: (
    predicate: (chunk: ReceivedChunk) => boolean,
    timeoutMs?: number
  ) => Promise<ReceivedChunk>;
  waitForChunks: (
    count: number,
    timeoutMs?: number
  ) => Promise<ReceivedChunk[]>;
  waitForChunksMatching: (
    predicate: (chunks: ReceivedChunk[]) => boolean,
    timeoutMs?: number
  ) => Promise<void>;
  cleanup: () => Promise<void>;
  createCredentials: (provider: string, apiKey: string) => Promise<void>;
  sessionFactory: SessionFactory;
}

/**
 * Creates a session factory for E2E testing
 */
export function createSessionFactory(testDir: string): SessionFactory {
  const createdSessionIds: string[] = [];

  const createSessionWithId = async (
    id: string,
    name = 'Test Session'
  ): Promise<TestSession> => {
    const { sessionManagerCreateWithId, sessionManagerDestroy } = await import(
      '@sengac/codelet-napi'
    );

    try {
      await sessionManagerCreateWithId(
        id,
        'anthropic/claude-sonnet-4',
        testDir,
        name
      );
    } catch {
      // Session creation may fail due to invalid API key, but session still exists
    }

    createdSessionIds.push(id);

    return {
      id,
      name,
      destroy: () => {
        try {
          sessionManagerDestroy(id);
          const idx = createdSessionIds.indexOf(id);
          if (idx !== -1) {
            createdSessionIds.splice(idx, 1);
          }
        } catch {
          // Ignore destroy errors
        }
      },
    };
  };

  const createSession = async (name = 'Test Session'): Promise<TestSession> => {
    return createSessionWithId(randomUUID(), name);
  };

  const destroySession = (sessionId: string) => {
    import('@sengac/codelet-napi')
      .then(({ sessionManagerDestroy }) => {
        try {
          sessionManagerDestroy(sessionId);
          const idx = createdSessionIds.indexOf(sessionId);
          if (idx !== -1) {
            createdSessionIds.splice(idx, 1);
          }
        } catch {
          // Ignore
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

  return {
    createSession,
    createSessionWithId,
    destroySession,
    destroyAllSessions,
    getCreatedSessionIds: () => [...createdSessionIds],
  };
}

/**
 * Creates a global chunk callback fixture for E2E testing.
 *
 * Uses GlobalSessionStreamManager's global handler to receive chunks.
 * This works because OnceCell only allows one callback registration.
 */
export async function createGlobalChunkCallbackFixture(
  testName: string
): Promise<GlobalChunkCallbackFixture> {
  // Create unique temp directory
  const testDir = join(
    tmpdir(),
    `fspec-bridge012-${testName}-${randomUUID().slice(0, 8)}`
  );
  await mkdir(testDir, { recursive: true });

  // Create spec directory structure
  const specDir = join(testDir, 'spec');
  await mkdir(specDir, { recursive: true });

  await writeFile(
    join(specDir, 'work-units.json'),
    JSON.stringify({
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
    })
  );

  // Set up persistence directory
  const { persistenceSetDataDirectory } = await import('@sengac/codelet-napi');
  persistenceSetDataDirectory(testDir);

  // Track received chunks via GlobalSessionStreamManager's global handler
  const receivedChunks: ReceivedChunk[] = [];

  // Initialize the global stream manager and WAIT for callback registration
  const manager = GlobalSessionStreamManager.getInstance();
  await manager.registerGlobalCallback();

  // Register global handler to capture ALL chunks
  const unregisterGlobalHandler = manager.registerGlobalHandler(
    (sessionId: string, chunk: StreamChunk) => {
      receivedChunks.push({
        sessionId,
        chunk,
        timestamp: Date.now(),
      });
    }
  );

  const getChunksForSession = (sessionId: string): ReceivedChunk[] => {
    return receivedChunks.filter(c => c.sessionId === sessionId);
  };

  const getChunksByType = (type: string): ReceivedChunk[] => {
    return receivedChunks.filter(c => c.chunk.type === type);
  };

  const clearChunks = () => {
    receivedChunks.length = 0;
  };

  const waitForChunk = async (
    predicate: (chunk: ReceivedChunk) => boolean,
    timeoutMs = 5000
  ): Promise<ReceivedChunk> => {
    const startTime = Date.now();
    while (Date.now() - startTime < timeoutMs) {
      const found = receivedChunks.find(predicate);
      if (found) {
        return found;
      }
      await new Promise(resolve => setTimeout(resolve, 50));
    }
    throw new Error(`Timeout waiting for chunk after ${timeoutMs}ms`);
  };

  const waitForChunks = async (
    count: number,
    timeoutMs = 5000
  ): Promise<ReceivedChunk[]> => {
    const startTime = Date.now();
    while (Date.now() - startTime < timeoutMs) {
      if (receivedChunks.length >= count) {
        return receivedChunks.slice(0, count);
      }
      await new Promise(resolve => setTimeout(resolve, 50));
    }
    throw new Error(
      `Timeout waiting for ${count} chunks, only received ${receivedChunks.length} after ${timeoutMs}ms`
    );
  };

  const waitForChunksMatching = async (
    predicate: (chunks: ReceivedChunk[]) => boolean,
    timeoutMs = 5000
  ): Promise<void> => {
    const startTime = Date.now();
    while (Date.now() - startTime < timeoutMs) {
      if (predicate(receivedChunks)) {
        return;
      }
      await new Promise(resolve => setTimeout(resolve, 50));
    }
    throw new Error(
      `Timeout waiting for chunks. Received: ${JSON.stringify(
        receivedChunks.map(c => ({
          sessionId: c.sessionId,
          type: c.chunk.type,
        }))
      )}`
    );
  };

  const createCredentials = async (
    provider: string,
    apiKey: string
  ): Promise<void> => {
    const credentialsDir = join(testDir, 'credentials');
    await mkdir(credentialsDir, { recursive: true });

    await writeFile(
      join(credentialsDir, 'credentials.json'),
      JSON.stringify({
        version: 1,
        providers: {
          [provider]: {
            apiKey,
            lastUpdated: new Date().toISOString(),
          },
        },
      }),
      { mode: 0o600 }
    );
  };

  const cleanup = async () => {
    // Unregister our global handler
    unregisterGlobalHandler();

    // Destroy sessions
    try {
      const { sessionManagerList, sessionManagerDestroy } = await import(
        '@sengac/codelet-napi'
      );
      const sessions = sessionManagerList();
      for (const session of sessions) {
        try {
          sessionManagerDestroy(session.id);
        } catch {
          // Ignore
        }
      }
    } catch {
      // Ignore
    }

    // Remove temp directory
    if (existsSync(testDir)) {
      await rm(testDir, { recursive: true, force: true });
    }
  };

  const sessionFactory = createSessionFactory(testDir);

  return {
    testDir,
    receivedChunks,
    getChunksForSession,
    getChunksByType,
    clearChunks,
    waitForChunk,
    waitForChunks,
    waitForChunksMatching,
    cleanup,
    createCredentials,
    sessionFactory,
  };
}
