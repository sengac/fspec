/**
 * Feature: spec/features/merge-conflict-markers.feature
 *
 * E2E integration tests for BUG-098: Verify that when apply_session_changes()
 * detects conflicts, the Rust layer writes standard git conflict markers into
 * the worktree files BEFORE returning ConflictError.
 *
 * Test strategy:
 * - Full E2E through TypeScript → Rust NAPI → TypeScript
 *   (creates real git repos, real worktrees, real file I/O)
 * - NO MOCKS — everything goes through the real NAPI layer
 * - Verifies conflict markers (<<<<<<< / ======= / >>>>>>>) are present
 *   in worktree files after a conflict is reported
 * - Verifies clean auto-merge for non-overlapping changes
 * - Verifies binary files don't get conflict markers
 * - Verifies identical changes don't produce conflicts
 *
 * Work Unit: BUG-098
 */

import { describe, it, expect, beforeAll, afterAll, afterEach } from 'vitest';
import { writeFile, readFile, mkdir, rm } from 'fs/promises';
import { existsSync, readFileSync } from 'fs';
import { join } from 'path';
import {
  createE2EFixture,
  createTestContext,
  getStatusMessages,
} from './fixtures/mergeWorktreeFixture';
import type { E2EFixture } from './fixtures/mergeWorktreeFixture';
import { handleMergeWorktree } from '../mergeWorktreeHandler';

// ============================================================================
// Shared initial files for BUG-098 conflict marker tests
// ============================================================================

const INITIAL_FILES: Record<string, string> = {
  'README.md':
    'line1\nline2\nline3\nline4\nline5\nline6\nThe Spec-Driven\nline8\n',
  'src/app.ts':
    'line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nline10\n' +
    'line11\nline12\nline13\nline14\nline15\nline16\nline17\nline18\nline19\nline20\n',
};

// ============================================================================
// Tests
// ============================================================================

describe('Feature: Merge conflict markers written to worktree files (BUG-098)', () => {
  let fixture: E2EFixture;

  beforeAll(async () => {
    fixture = await createE2EFixture('bug098-markers', INITIAL_FILES);
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
    // Remove files that tests may have added
    try {
      await rm(join(fixture.testDir, 'utils.ts'), { force: true });
    } catch {
      /* might not exist */
    }
  });

  // ========================================================================
  // Scenario: Conflicting text file gets standard git conflict markers
  //           written to worktree (TS → Rust NAPI → TS)
  // ========================================================================
  describe('Scenario: Conflicting text file gets standard git conflict markers written to worktree', () => {
    it('should write <<<<<<< / ======= / >>>>>>> markers into the worktree file', async () => {
      // @step Given a session worktree with base commit containing "README.md"
      const { sessionId, worktreePath } = await fixture.createIsolatedSession(
        'Conflict Markers E2E'
      );

      // @step And the session has modified line 7 of "README.md" from "The Spec-Driven" to "Da Spec-Driven"
      await writeFile(
        join(worktreePath, 'README.md'),
        'line1\nline2\nline3\nline4\nline5\nline6\nDa Spec-Driven\nline8\n'
      );

      // @step And the main worktree has modified line 7 of "README.md" from "The Spec-Driven" to "The Spec-Driven (v2.0)"
      await writeFile(
        join(fixture.testDir, 'README.md'),
        'line1\nline2\nline3\nline4\nline5\nline6\nThe Spec-Driven (v2.0)\nline8\n'
      );

      const { ctx, conversation, calls } = createTestContext(
        fixture,
        sessionId,
        { worktreePath }
      );

      // @step When apply_session_changes is called
      await handleMergeWorktree(ctx);

      // @step And a ConflictError should be returned listing "README.md"
      const statusMessages = getStatusMessages(conversation);
      expect(statusMessages.length).toBeGreaterThanOrEqual(1);
      expect(statusMessages[0].toLowerCase()).toContain('conflict');
      expect(statusMessages[0]).toContain('README.md');

      // @step Then the worktree "README.md" should contain "<<<<<<< session (your changes)"
      // @step And the worktree "README.md" should contain "======="
      // @step And the worktree "README.md" should contain ">>>>>>> main"
      // @step And the worktree "README.md" should contain "Da Spec-Driven"
      // @step And the worktree "README.md" should contain "The Spec-Driven (v2.0)"
      const worktreeContent = readFileSync(
        join(worktreePath, 'README.md'),
        'utf-8'
      );
      expect(worktreeContent).toContain('<<<<<<< session (your changes)');
      expect(worktreeContent).toContain('=======');
      expect(worktreeContent).toContain('>>>>>>> main');
      expect(worktreeContent).toContain('Da Spec-Driven');
      expect(worktreeContent).toContain('The Spec-Driven (v2.0)');

      // The LLM context injection should reference the conflict
      expect(calls.llmContextInjected.length).toBe(1);
      expect(calls.llmContextInjected[0]).toContain('README.md');
    });
  });

  // ========================================================================
  // Scenario: Non-overlapping changes in same file merge cleanly without
  //           conflict markers (TS → Rust NAPI → TS)
  // ========================================================================
  describe('Scenario: Non-overlapping changes in same file merge cleanly without conflict markers', () => {
    it('should auto-merge non-overlapping changes and apply to main', async () => {
      // @step Given a session worktree with base commit containing "src/app.ts"
      const { sessionId, worktreePath } =
        await fixture.createIsolatedSession('Auto Merge E2E');

      // @step And the session has modified lines 2-3 of "src/app.ts"
      const sessionContent =
        'line1\nSESSION_EDIT\nSESSION_EDIT\nline4\nline5\nline6\nline7\nline8\nline9\nline10\n' +
        'line11\nline12\nline13\nline14\nline15\nline16\nline17\nline18\nline19\nline20\n';
      await writeFile(join(worktreePath, 'src', 'app.ts'), sessionContent);

      // @step And the main worktree has modified lines 18-19 of "src/app.ts" with no overlap
      const mainContent =
        'line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nline10\n' +
        'line11\nline12\nline13\nline14\nline15\nline16\nline17\nMAIN_EDIT\nMAIN_EDIT\nline20\n';
      await writeFile(join(fixture.testDir, 'src', 'app.ts'), mainContent);

      const { ctx, calls } = createTestContext(fixture, sessionId, {
        worktreePath,
      });

      // @step When apply_session_changes is called
      await handleMergeWorktree(ctx);

      // @step And no ConflictError should be returned
      // On success, the handler sets an action prompt (not a conflict message)
      expect(calls.actionPromptSet).not.toBeNull();
      expect(calls.llmContextInjected.length).toBe(0);

      // @step Then "src/app.ts" should be copied to the main worktree with both changes merged
      const mergedContent = readFileSync(
        join(fixture.testDir, 'src', 'app.ts'),
        'utf-8'
      );
      expect(mergedContent).toContain('SESSION_EDIT');
      expect(mergedContent).toContain('MAIN_EDIT');

      // @step And "src/app.ts" should NOT contain conflict markers
      expect(mergedContent).not.toContain('<<<<<<<');
    });
  });

  // ========================================================================
  // Scenario: File added in both session and main with different content
  //           gets conflict markers (TS → Rust NAPI → TS)
  // ========================================================================
  describe('Scenario: File added in both session and main with different content gets conflict markers', () => {
    it('should write conflict markers when both sides add the same file', async () => {
      // @step Given a session worktree with base commit that does NOT contain "utils.ts"
      const { sessionId, worktreePath } = await fixture.createIsolatedSession(
        'New File Conflict E2E'
      );

      // @step And the session has added "utils.ts" with content "export const x = 1;"
      await writeFile(join(worktreePath, 'utils.ts'), 'export const x = 1;\n');

      // @step And the main worktree has also added "utils.ts" with content "export const x = 2;"
      await writeFile(
        join(fixture.testDir, 'utils.ts'),
        'export const x = 2;\n'
      );

      const { ctx, conversation, calls } = createTestContext(
        fixture,
        sessionId,
        { worktreePath }
      );

      // @step When apply_session_changes is called
      await handleMergeWorktree(ctx);

      // @step Then a ConflictError should be returned listing "utils.ts"
      const statusMessages = getStatusMessages(conversation);
      expect(statusMessages.length).toBeGreaterThanOrEqual(1);
      expect(statusMessages[0].toLowerCase()).toContain('conflict');

      // @step And the worktree "utils.ts" should contain "<<<<<<< session (your changes)"
      // @step And the worktree "utils.ts" should contain ">>>>>>> main"
      const worktreeContent = readFileSync(
        join(worktreePath, 'utils.ts'),
        'utf-8'
      );
      expect(worktreeContent).toContain('<<<<<<< session (your changes)');
      expect(worktreeContent).toContain('>>>>>>> main');

      // LLM context should mention the file
      expect(calls.llmContextInjected.length).toBe(1);
    });
  });

  // ========================================================================
  // Scenario: Binary file conflict is reported without writing conflict
  //           markers (TS → Rust NAPI → TS)
  // ========================================================================
  describe('Scenario: Binary file conflict is reported without writing conflict markers', () => {
    it('should list binary file as conflicting but NOT write markers', async () => {
      // Create a binary file first and commit it
      const binaryBase = Buffer.from([
        0x89, 0x50, 0x4e, 0x47, 0x00, 0x01, 0x02, 0x03,
      ]);
      await writeFile(join(fixture.testDir, 'logo.png'), binaryBase);

      // Commit the binary file
      const { execSync } = await import('child_process');
      execSync('git add . && git commit -m "add binary"', {
        cwd: fixture.testDir,
      });

      // @step Given a session worktree with base commit containing binary file "logo.png"
      const { sessionId, worktreePath } = await fixture.createIsolatedSession(
        'Binary Conflict E2E'
      );

      // @step And the session has modified "logo.png" with new binary content
      const sessionBinary = Buffer.from([
        0x89, 0x50, 0x4e, 0x47, 0x00, 0xaa, 0xbb, 0xcc,
      ]);
      await writeFile(join(worktreePath, 'logo.png'), sessionBinary);

      // @step And the main worktree has also modified "logo.png" with different binary content
      const mainBinary = Buffer.from([
        0x89, 0x50, 0x4e, 0x47, 0x00, 0xdd, 0xee, 0xff,
      ]);
      await writeFile(join(fixture.testDir, 'logo.png'), mainBinary);

      const { ctx, conversation } = createTestContext(fixture, sessionId, {
        worktreePath,
      });

      // @step When apply_session_changes is called
      await handleMergeWorktree(ctx);

      // @step Then a ConflictError should be returned listing "logo.png"
      const statusMessages = getStatusMessages(conversation);
      expect(statusMessages.length).toBeGreaterThanOrEqual(1);
      expect(statusMessages[0].toLowerCase()).toContain('conflict');

      // @step And the worktree "logo.png" should NOT contain conflict markers
      const fileContent = await readFile(join(worktreePath, 'logo.png'));
      const asStr = fileContent.toString('utf-8');
      expect(asStr).not.toContain('<<<<<<<');

      // @step And the worktree "logo.png" should retain the session version
      expect(Buffer.compare(fileContent, sessionBinary)).toBe(0);
    });
  });

  // ========================================================================
  // Scenario: Identical changes from session and main do not produce a
  //           conflict (TS → Rust NAPI → TS)
  // ========================================================================
  describe('Scenario: Identical changes from session and main do not produce a conflict', () => {
    it('should succeed when both sides make the same change', async () => {
      // @step Given a session worktree with base commit containing "README.md"
      const { sessionId, worktreePath } = await fixture.createIsolatedSession(
        'Identical Changes E2E'
      );

      // @step And the session has modified line 7 of "README.md" from "The Spec-Driven" to "Da Spec-Driven"
      const changed =
        'line1\nline2\nline3\nline4\nline5\nline6\nDa Spec-Driven\nline8\n';
      await writeFile(join(worktreePath, 'README.md'), changed);

      // @step And the main worktree has also modified line 7 identically
      await writeFile(join(fixture.testDir, 'README.md'), changed);

      const { ctx, calls } = createTestContext(fixture, sessionId, {
        worktreePath,
      });

      // @step When apply_session_changes is called
      await handleMergeWorktree(ctx);

      // @step Then no ConflictError should be returned
      expect(calls.actionPromptSet).not.toBeNull();
      expect(calls.llmContextInjected.length).toBe(0);

      // @step And "README.md" should be applied to the main worktree without conflict markers
      const mainContent = readFileSync(
        join(fixture.testDir, 'README.md'),
        'utf-8'
      );
      expect(mainContent).toContain('Da Spec-Driven');
      expect(mainContent).not.toContain('<<<<<<<');
    });
  });
});
