/**
 * Feature: spec/features/merge-conflict-llm-context.feature
 *
 * E2E integration tests for GIT-038: Send merge conflict context to LLM
 * for AI-assisted resolution.
 *
 * When /merge-worktree detects conflicts, a message with conflict details must
 * be sent to the Rust session (via injectLlmContext → setInputValue + auto-submit
 * → handleSubmit → sessionSendInput) so the LLM knows which files conflict and
 * can resolve the conflict markers.
 *
 * Test strategy:
 * - Full E2E through TypeScript → Rust NAPI → TypeScript
 *   (creates real git repos, real worktrees, real merge/inspect operations)
 * - NO MOCKS — everything goes through the real NAPI layer
 * - Verifies injectLlmContext is called with correct content on conflict
 * - Verifies injectLlmContext is NOT called on success, nothing-to-merge,
 *   or non-conflict errors
 * - Verifies content includes file list, worktree path,
 *   conflict markers mention, and /merge-worktree instruction
 *
 * Work Unit: GIT-038
 */

import { describe, it, expect, beforeAll, afterAll, afterEach } from 'vitest';
import { writeFile, rm } from 'fs/promises';
import { join } from 'path';

import { handleMergeWorktree } from '../mergeWorktreeHandler';
import {
  createE2EFixture,
  createTestContext,
  getStatusMessages,
} from './fixtures/mergeWorktreeFixture';
import type { E2EFixture } from './fixtures/mergeWorktreeFixture';

// ============================================================================
// Initial files for GIT-038 tests
// ============================================================================

const INITIAL_FILES: Record<string, string> = {
  'README.md': '# Test Project\n',
  'src/auth/login.ts': 'export function login() { return true; }\n',
  'src/utils/helpers.ts': 'export function helper() { return 42; }\n',
};

// ============================================================================
// Tests
// ============================================================================

describe('Feature: Send merge conflict context to LLM for AI-assisted resolution', () => {
  let fixture: E2EFixture;

  beforeAll(async () => {
    fixture = await createE2EFixture('llm-context', INITIAL_FILES);
    await fixture.initGitRepo();
  });

  afterAll(async () => {
    await fixture.cleanup();
  });

  afterEach(async () => {
    await fixture.destroyAllSessions();
    fixture.resetStores();

    // Reset main worktree files to committed state
    for (const [relPath, content] of Object.entries(INITIAL_FILES)) {
      await writeFile(join(fixture.testDir, relPath), content);
    }
    // Remove any files added by tests
    try {
      await rm(join(fixture.testDir, 'src', 'new-file.ts'), { force: true });
    } catch {
      /* might not exist */
    }
  });

  // ========================================================================
  // Scenario: Single file conflict sends context to LLM
  // ========================================================================
  describe('Scenario: Single file conflict sends context to LLM', () => {
    it('should send conflict context message with details for single file', async () => {
      // @step Given I am in an isolated session with a worktree
      const { sessionId, worktreePath } = await fixture.createIsolatedSession(
        'Single Conflict LLM'
      );

      // @step And the session has a valid session ID
      expect(sessionId).toBeTruthy();

      // Modify README.md in session worktree
      await writeFile(
        join(worktreePath, 'README.md'),
        '# Modified in session\n'
      );

      // Modify the SAME file in main worktree (creating a conflict)
      await writeFile(
        join(fixture.testDir, 'README.md'),
        '# Modified in main\n'
      );

      const { ctx, conversation, calls } = createTestContext(
        fixture,
        sessionId,
        { worktreePath }
      );

      // @step When I run /merge-worktree and a conflict is detected on "README.md"
      await handleMergeWorktree(ctx);

      // @step Then the TUI status message should contain the conflict summary
      const statusMessages = getStatusMessages(conversation);
      expect(statusMessages.length).toBeGreaterThanOrEqual(1);
      expect(statusMessages[0].toLowerCase()).toContain('conflict');

      // @step And a conflict context message should be sent to the LLM session
      expect(calls.llmContextInjected.length).toBe(1);

      // @step And the message should list "README.md" as a conflicting file
      expect(calls.llmContextInjected[0]).toContain('README.md');

      // @step And the message should mention git conflict markers
      expect(calls.llmContextInjected[0]).toMatch(/<<<<<<</);
      expect(calls.llmContextInjected[0]).toMatch(/=======/);
      expect(calls.llmContextInjected[0]).toMatch(/>>>>>>>/);

      // @step And the message should instruct to run /merge-worktree again after resolving
      expect(calls.llmContextInjected[0]).toContain('/merge-worktree');

      // GIT-038 Rule: Must include worktree path
      expect(calls.llmContextInjected[0]).toContain(worktreePath);
    });
  });

  // ========================================================================
  // Scenario: Multiple file conflicts send context listing all files
  // ========================================================================
  describe('Scenario: Multiple file conflicts send context listing all files', () => {
    it('should send context listing all conflicting files', async () => {
      // @step Given I am in an isolated session with a worktree
      const { sessionId, worktreePath } =
        await fixture.createIsolatedSession('Multi Conflict LLM');

      // @step And the session has a valid session ID
      expect(sessionId).toBeTruthy();

      // Modify two files in session worktree
      await writeFile(
        join(worktreePath, 'src', 'auth', 'login.ts'),
        'export function login() { return false; }\n'
      );
      await writeFile(
        join(worktreePath, 'src', 'utils', 'helpers.ts'),
        'export function helper() { return 99; }\n'
      );

      // Modify the SAME files in main worktree (creating conflicts)
      await writeFile(
        join(fixture.testDir, 'src', 'auth', 'login.ts'),
        'export function login() { return "main"; }\n'
      );
      await writeFile(
        join(fixture.testDir, 'src', 'utils', 'helpers.ts'),
        'export function helper() { return "main"; }\n'
      );

      const { ctx, calls } = createTestContext(fixture, sessionId, {
        worktreePath,
      });

      // @step When I run /merge-worktree and conflicts are detected on "src/auth/login.ts" and "src/utils/helpers.ts"
      await handleMergeWorktree(ctx);

      // @step Then a conflict context message should be sent to the LLM session
      expect(calls.llmContextInjected.length).toBe(1);

      // @step And the message should list "src/auth/login.ts" as a conflicting file
      expect(calls.llmContextInjected[0]).toContain('src/auth/login.ts');

      // @step And the message should list "src/utils/helpers.ts" as a conflicting file
      expect(calls.llmContextInjected[0]).toContain('src/utils/helpers.ts');
    });
  });

  // ========================================================================
  // Scenario: Conflict context triggers auto-submit to Rust session
  // ========================================================================
  describe('Scenario: Conflict context triggers auto-submit to Rust session', () => {
    it('should call injectLlmContext AND keep TUI status message', async () => {
      // @step Given I am in an isolated session with a worktree
      const { sessionId, worktreePath } =
        await fixture.createIsolatedSession('React Conv LLM');

      // @step And the session has a valid session ID
      expect(sessionId).toBeTruthy();

      // Create conflict on README.md
      await writeFile(join(worktreePath, 'README.md'), '# Session README\n');
      await writeFile(join(fixture.testDir, 'README.md'), '# Main README\n');

      const { ctx, conversation, calls } = createTestContext(
        fixture,
        sessionId,
        { worktreePath }
      );

      // @step When I run /merge-worktree and a conflict is detected on "README.md"
      await handleMergeWorktree(ctx);

      // @step Then the injectLlmContext callback should be called with conflict details
      expect(calls.llmContextInjected.length).toBe(1);
      expect(calls.llmContextInjected[0]).toContain('README.md');

      // @step And the TUI status message should still be present unchanged
      const statusMessages = getStatusMessages(conversation);
      expect(statusMessages.length).toBeGreaterThanOrEqual(1);
      expect(statusMessages[0].toLowerCase()).toContain('conflict');
    });
  });

  // ========================================================================
  // Scenario: Successful merge does not send context to LLM
  // ========================================================================
  describe('Scenario: Successful merge does not send context to LLM', () => {
    it('should not send context on successful merge', async () => {
      // @step Given I am in an isolated session with a worktree
      const { sessionId, worktreePath } =
        await fixture.createIsolatedSession('No Inject LLM');

      // @step And the session has a valid session ID
      expect(sessionId).toBeTruthy();

      // Modify a file only in the session (no conflict)
      await writeFile(
        join(worktreePath, 'README.md'),
        '# Updated in session only\n'
      );

      const { ctx, calls } = createTestContext(fixture, sessionId, {
        worktreePath,
      });

      // @step When I run /merge-worktree and the merge succeeds
      await handleMergeWorktree(ctx);

      // @step Then no conflict context message should be sent to the LLM session
      expect(calls.llmContextInjected.length).toBe(0);

      // @step And the action prompt "Press Enter to close session" should be shown
      expect(calls.actionPromptSet).not.toBeNull();
      expect(calls.actionPromptSet?.message).toContain('Press Enter');
    });
  });

  // ========================================================================
  // Scenario: Non-conflict error does not send context to LLM
  // ========================================================================
  describe('Scenario: Non-conflict error does not send context to LLM', () => {
    it('should not send context on non-conflict error', async () => {
      // @step Given I am in an isolated session with a worktree
      const { sessionId, worktreePath } =
        await fixture.createIsolatedSession('Error LLM');

      // @step And the session has a valid session ID
      expect(sessionId).toBeTruthy();

      // Modify a file so inspect finds changes
      await writeFile(
        join(worktreePath, 'README.md'),
        '# Modified for error test\n'
      );

      // Destroy the worktree to simulate a non-conflict error
      const { removeWorktree } = await import('@sengac/codelet-napi');
      try {
        removeWorktree(fixture.testDir, sessionId);
      } catch {
        // May fail if worktree path differs
      }

      const { ctx, conversation, calls } = createTestContext(
        fixture,
        sessionId,
        { worktreePath }
      );

      // @step When I run /merge-worktree and a non-conflict error occurs
      await handleMergeWorktree(ctx);

      // @step Then no conflict context message should be sent to the LLM session
      expect(calls.llmContextInjected.length).toBe(0);

      // @step And the TUI status message should show the generic error
      const statusMessages = getStatusMessages(conversation);
      expect(statusMessages.length).toBeGreaterThanOrEqual(1);
      expect(statusMessages[0]).toContain('Merge failed');
    });
  });

  // ========================================================================
  // Scenario: Context message does not contain system-reminder tags
  // ========================================================================
  describe('Scenario: Context message does not contain system-reminder tags', () => {
    it('should send clean conflict context without system-reminder tags', async () => {
      // @step Given I am in an isolated session with a worktree
      const { sessionId, worktreePath } =
        await fixture.createIsolatedSession('Role Check LLM');

      // @step And the session has a valid session ID
      expect(sessionId).toBeTruthy();

      // Create conflict on README.md
      await writeFile(
        join(worktreePath, 'README.md'),
        '# Session role check\n'
      );
      await writeFile(
        join(fixture.testDir, 'README.md'),
        '# Main role check\n'
      );

      const { ctx, calls } = createTestContext(fixture, sessionId, {
        worktreePath,
      });

      // @step When I run /merge-worktree and a conflict is detected on "README.md"
      await handleMergeWorktree(ctx);

      // @step Then the conflict context message should be sent to the LLM session
      expect(calls.llmContextInjected.length).toBe(1);

      // @step And the message should not contain system-reminder tags
      expect(calls.llmContextInjected[0]).not.toContain('<system-reminder>');
    });
  });

  // ========================================================================
  // Scenario: No context sent when session ID is missing
  // ========================================================================
  describe('Scenario: No context sent when session ID is missing', () => {
    it('should not send context when session ID is null', async () => {
      // @step Given I am in an isolated session with a worktree
      // @step But the session ID is null
      const { ctx, conversation, calls } = createTestContext(
        fixture,
        'dummy-unused',
        { currentSessionId: null }
      );

      // @step When I run /merge-worktree and a conflict is detected
      await handleMergeWorktree(ctx);

      // @step Then no conflict context message should be sent to the LLM session
      expect(calls.llmContextInjected.length).toBe(0);

      // @step And the TUI status message should still show the conflict summary
      // With null session ID, the handler exits early with "No active session"
      const statusMessages = getStatusMessages(conversation);
      expect(statusMessages.length).toBe(1);
      expect(statusMessages[0]).toBe('No active session');
    });
  });
});
