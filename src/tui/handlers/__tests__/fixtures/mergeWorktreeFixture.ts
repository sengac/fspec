/**
 * Shared E2E fixture for merge worktree integration tests.
 *
 * Creates real git repos, real NAPI sessions, and real stores.
 * Used by both GIT-036 (merge-worktree-command.test.ts) and
 * GIT-037 (mergeWorktreeHandler-ux.test.tsx).
 *
 * NO MOCKS — everything goes through TypeScript → Rust NAPI → TypeScript.
 */

import { randomUUID } from 'crypto';
import { mkdir, writeFile, rm } from 'fs/promises';
import { existsSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import * as git from 'isomorphic-git';
import * as fs from 'fs';

import type { MergeWorktreeContext } from '../../mergeWorktreeHandler';
import type { ActionPrompt } from '../../../types/actionPrompt';
import { useFspecStore } from '../../../store/fspecStore';
import { useSessionStore } from '../../../store/sessionStore';

const TEST_MODEL = 'anthropic/claude-sonnet-4-20250514';
const CLEANUP_DELAY_MS = 100;

export interface E2EFixture {
  testDir: string;
  createdSessionIds: string[];
  initGitRepo: () => Promise<void>;
  createIsolatedSession: (name?: string) => Promise<{
    sessionId: string;
    worktreePath: string;
  }>;
  destroyAllSessions: () => Promise<void>;
  resetStores: () => void;
  cleanup: () => Promise<void>;
}

/**
 * Create a full E2E fixture with a real git repo, NAPI persistence, and stores.
 *
 * @param testName - Unique name for the test directory
 * @param initialFiles - Optional map of relative paths to file content for initial commit.
 *   Defaults to README.md + src/main.ts if not provided.
 */
export async function createE2EFixture(
  testName: string,
  initialFiles?: Record<string, string>
): Promise<E2EFixture> {
  const testDir = join(
    tmpdir(),
    `fspec-merge-${testName}-${randomUUID().slice(0, 8)}`
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

  // Set persistence directory for NAPI
  const { persistenceSetDataDirectory } = await import('@sengac/codelet-napi');
  persistenceSetDataDirectory(testDir);

  const resetStores = (): void => {
    useFspecStore.setState({ sessionAttachments: new Map() });
    useSessionStore.getState().setCurrentWorkUnit(null, null);
  };

  resetStores();

  const defaultFiles: Record<string, string> = initialFiles ?? {
    'README.md': '# Test Project\n',
    'src/main.ts': 'export const VERSION = 1;\n',
  };

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

    // Create initial files
    for (const [relPath, content] of Object.entries(defaultFiles)) {
      const fullPath = join(testDir, relPath);
      const dir = fullPath.substring(0, fullPath.lastIndexOf('/'));
      if (dir !== testDir) {
        await mkdir(dir, { recursive: true });
      }
      await writeFile(fullPath, content);
      await git.add({ fs, dir: testDir, filepath: relPath });
    }

    await git.commit({
      fs,
      dir: testDir,
      message: 'Initial commit',
      author: { name: 'Test User', email: 'test@example.com' },
    });
  };

  const createIsolatedSession = async (
    name = 'Merge E2E Session'
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

    for (const id of [...createdSessionIds]) {
      try {
        sessionManagerDestroy(id);
      } catch {
        /* cleanup */
      }
    }
    createdSessionIds.length = 0;

    try {
      const allSessions = sessionManagerList();
      for (const session of allSessions) {
        try {
          sessionManagerDestroy(session.id);
        } catch {
          /* cleanup */
        }
      }
    } catch {
      /* cleanup */
    }

    try {
      const worktrees = listWorktrees(testDir);
      for (const worktree of worktrees) {
        try {
          removeWorktree(testDir, worktree.sessionId);
        } catch {
          /* cleanup */
        }
      }
    } catch {
      /* cleanup */
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
    initGitRepo,
    createIsolatedSession,
    destroyAllSessions,
    resetStores,
    cleanup,
  };
}

/**
 * Tracking object for context callbacks.
 */
export interface ContextCallTracker {
  cleanupCalled: boolean;
  onExitCalled: boolean;
  inputValueSet: string | null;
  actionPromptSet: ActionPrompt | null;
}

/**
 * Create a test context with call tracking.
 */
export function createTestContext(
  fixture: E2EFixture,
  sessionId: string,
  overrides: Partial<MergeWorktreeContext> = {}
): {
  ctx: MergeWorktreeContext;
  conversation: Array<{ type: string; content: string }>;
  calls: ContextCallTracker;
} {
  const conversation: Array<{ type: string; content: string }> = [];
  const calls: ContextCallTracker = {
    cleanupCalled: false,
    onExitCalled: false,
    inputValueSet: null,
    actionPromptSet: null,
  };

  const ctx: MergeWorktreeContext = {
    isIsolated: true,
    currentSessionId: sessionId,
    repoPath: fixture.testDir,
    setConversation: updater => {
      const result = updater(conversation);
      conversation.length = 0;
      conversation.push(...result);
    },
    setInputValue: (value: string) => {
      calls.inputValueSet = value;
    },
    cleanupCurrentSessionHandler: () => {
      calls.cleanupCalled = true;
    },
    onExit: () => {
      calls.onExitCalled = true;
    },
    setActionPrompt: (prompt: ActionPrompt | null) => {
      calls.actionPromptSet = prompt;
    },
    ...overrides,
  };

  return { ctx, conversation, calls };
}

/**
 * Extract status message content strings from a conversation array.
 */
export function getStatusMessages(
  conversation: Array<{ type: string; content: string }>
): string[] {
  return conversation.filter(m => m.type === 'status').map(m => m.content);
}
