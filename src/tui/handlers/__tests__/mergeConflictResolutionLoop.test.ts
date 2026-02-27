/**
 * Feature: spec/features/merge-conflict-resolution-loop.feature
 *
 * E2E integration tests for BUG-099: Verify that re-merge after conflict
 * resolution does NOT enter an infinite loop. Tests the full TypeScript →
 * Rust NAPI → TypeScript path with real git repos.
 *
 * NO MOCKS — everything goes through the real NAPI layer.
 *
 * Work Unit: BUG-099
 */

import { describe, it, expect, beforeAll, afterAll, afterEach } from 'vitest';
import { writeFile, readFile } from 'fs/promises';
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
// Shared initial files for BUG-099 conflict resolution loop tests
// ============================================================================

const INITIAL_FILES: Record<string, string> = {
  'README.md':
    'line1\nline2\n**The Spec-Driven, Multi-Agent Coding Factory**\nline4\n',
};

// ============================================================================
// Tests
// ============================================================================

describe('Feature: Re-merge after conflict resolution enters infinite loop (BUG-099)', () => {
  let fixture: E2EFixture;

  beforeAll(async () => {
    fixture = await createE2EFixture('bug099-loop', INITIAL_FILES);
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
  });

  // ========================================================================
  // Scenario: Re-merge after conflict resolution does not enter infinite loop
  // (Feature file L52 — exact reproduction from debug session bb90f15f)
  // ========================================================================
  describe('Scenario: Re-merge after conflict resolution does not enter infinite loop', () => {
    it('should succeed on re-merge after LLM resolves conflict markers (TS → Rust → TS)', async () => {
      // @step Given a session worktree with base commit containing "README.md" with "The Spec-Driven"
      const { sessionId, worktreePath } =
        await fixture.createIsolatedSession('BUG-099 Loop Repro');

      // @step And the session has modified "README.md" to "Ma Spec-Driven"
      await writeFile(
        join(worktreePath, 'README.md'),
        'line1\nline2\n**Ma Spec-Driven, Multi-Agent Coding Factory**\nline4\n'
      );

      // @step And the main worktree has modified "README.md" to "Those Spec-Driven"
      await writeFile(
        join(fixture.testDir, 'README.md'),
        'line1\nline2\n**Those Spec-Driven, Multi-Agent Coding Factory**\nline4\n'
      );

      // @step When apply_session_changes is called the first time
      const firstCtx = createTestContext(fixture, sessionId, { worktreePath });
      await handleMergeWorktree(firstCtx.ctx);

      // @step Then a ConflictError should be returned listing "README.md"
      const firstMessages = getStatusMessages(firstCtx.conversation);
      expect(firstMessages.length).toBeGreaterThanOrEqual(1);
      expect(firstMessages[0].toLowerCase()).toContain('conflict');
      expect(firstMessages[0]).toContain('README.md');

      // @step And the worktree "README.md" should contain "<<<<<<< session (your changes)"
      const markersContent = readFileSync(
        join(worktreePath, 'README.md'),
        'utf-8'
      );
      expect(markersContent).toContain('<<<<<<< session (your changes)');

      // @step And a ".fspec-pending-conflicts" file should exist in the worktree listing "README.md"
      const stateFile = join(worktreePath, '.fspec-pending-conflicts');
      expect(existsSync(stateFile)).toBe(true);
      const stateContent = readFileSync(stateFile, 'utf-8');
      expect(stateContent).toContain('README.md');

      // @step When the user resolves "README.md" by removing conflict markers and keeping "Ma Spec-Driven"
      await writeFile(
        join(worktreePath, 'README.md'),
        'line1\nline2\n**Ma Spec-Driven, Multi-Agent Coding Factory**\nline4\n'
      );

      // @step And apply_session_changes is called again
      const secondCtx = createTestContext(fixture, sessionId, {
        worktreePath,
      });
      await handleMergeWorktree(secondCtx.ctx);

      // @step Then the merge should succeed without returning a ConflictError
      expect(secondCtx.calls.actionPromptSet).not.toBeNull();
      expect(secondCtx.calls.llmContextInjected.length).toBe(0);

      // @step And the main worktree "README.md" should contain "Ma Spec-Driven"
      const mainContent = readFileSync(
        join(fixture.testDir, 'README.md'),
        'utf-8'
      );
      expect(mainContent).toContain(
        '**Ma Spec-Driven, Multi-Agent Coding Factory**'
      );

      // @step And the ".fspec-pending-conflicts" file should be deleted
      // (worktree is removed on successful merge, so the file is gone too)
      expect(existsSync(worktreePath)).toBe(false);
    });
  });

  // ========================================================================
  // Scenario: First merge creates pending conflict state file alongside markers
  // (Feature file L67)
  // ========================================================================
  describe('Scenario: First merge creates pending conflict state file alongside markers', () => {
    it('should create .fspec-pending-conflicts on first conflict (TS → Rust → TS)', async () => {
      // @step Given a session worktree with base commit containing "README.md" with "original"
      const { sessionId, worktreePath } =
        await fixture.createIsolatedSession('BUG-099 State File');

      // @step And the session has modified "README.md" to "session version"
      await writeFile(
        join(worktreePath, 'README.md'),
        'session version of README\n'
      );

      // @step And the main worktree has modified "README.md" to "main version"
      await writeFile(
        join(fixture.testDir, 'README.md'),
        'main version of README\n'
      );

      // @step And no ".fspec-pending-conflicts" file exists in the worktree
      expect(existsSync(join(worktreePath, '.fspec-pending-conflicts'))).toBe(
        false
      );

      // @step When apply_session_changes is called
      const { ctx, conversation } = createTestContext(fixture, sessionId, {
        worktreePath,
      });
      await handleMergeWorktree(ctx);

      // @step Then a ConflictError should be returned listing "README.md"
      const messages = getStatusMessages(conversation);
      expect(messages[0].toLowerCase()).toContain('conflict');

      // @step And a ".fspec-pending-conflicts" file should exist in the worktree listing "README.md"
      const stateFile = join(worktreePath, '.fspec-pending-conflicts');
      expect(existsSync(stateFile)).toBe(true);
      const stateContent = readFileSync(stateFile, 'utf-8');
      expect(stateContent).toContain('README.md');
    });
  });

  // ========================================================================
  // Scenario: Resolved conflict file is accepted without re-running three-way merge
  // (Feature file L77)
  // ========================================================================
  describe('Scenario: Resolved conflict file is accepted without re-running three-way merge', () => {
    it('should accept resolved file and apply to main (TS → Rust → TS)', async () => {
      // @step Given a session worktree with pending conflicts listing "README.md"
      const { sessionId, worktreePath } = await fixture.createIsolatedSession(
        'BUG-099 Accept Resolved'
      );

      // Create conflict
      await writeFile(join(worktreePath, 'README.md'), 'session edit\n');
      await writeFile(join(fixture.testDir, 'README.md'), 'main edit\n');

      const firstCtx = createTestContext(fixture, sessionId, { worktreePath });
      await handleMergeWorktree(firstCtx.ctx);

      // Verify conflict occurred
      expect(existsSync(join(worktreePath, '.fspec-pending-conflicts'))).toBe(
        true
      );

      // @step And the worktree "README.md" does NOT contain "<<<<<<< " markers
      const resolved = 'manual-merge-result from LLM\n';
      await writeFile(join(worktreePath, 'README.md'), resolved);

      // @step When apply_session_changes is called
      const secondCtx = createTestContext(fixture, sessionId, {
        worktreePath,
      });
      await handleMergeWorktree(secondCtx.ctx);

      // @step Then the merge should succeed
      expect(secondCtx.calls.actionPromptSet).not.toBeNull();

      // @step And the worktree "README.md" content should be copied to main as the final resolution
      const mainContent = readFileSync(
        join(fixture.testDir, 'README.md'),
        'utf-8'
      );
      expect(mainContent).toContain('manual-merge-result from LLM');
    });
  });

  // ========================================================================
  // Scenario: Unresolved file with markers still present is reported without
  //           regenerating markers (Feature file L88)
  // ========================================================================
  describe('Scenario: Unresolved file with markers still present is reported without regenerating markers', () => {
    it('should return ConflictError without overwriting markers (TS → Rust → TS)', async () => {
      // @step Given a session worktree with pending conflicts listing "README.md"
      const { sessionId, worktreePath } =
        await fixture.createIsolatedSession('BUG-099 Unresolved');

      await writeFile(join(worktreePath, 'README.md'), 'session-v\n');
      await writeFile(join(fixture.testDir, 'README.md'), 'main-v\n');

      const firstCtx = createTestContext(fixture, sessionId, { worktreePath });
      await handleMergeWorktree(firstCtx.ctx);

      // @step And the worktree "README.md" still contains "<<<<<<< session (your changes)" markers
      const markersAfterFirst = readFileSync(
        join(worktreePath, 'README.md'),
        'utf-8'
      );
      expect(markersAfterFirst).toContain('<<<<<<< session (your changes)');

      // @step When apply_session_changes is called
      const secondCtx = createTestContext(fixture, sessionId, {
        worktreePath,
      });
      await handleMergeWorktree(secondCtx.ctx);

      // @step Then a ConflictError should be returned listing "README.md"
      const messages = getStatusMessages(secondCtx.conversation);
      expect(messages[0].toLowerCase()).toContain('conflict');

      // @step And the worktree "README.md" content should be byte-identical to before the re-merge call
      const markersAfterSecond = readFileSync(
        join(worktreePath, 'README.md'),
        'utf-8'
      );
      expect(markersAfterSecond).toBe(markersAfterFirst);

      // @step And the ".fspec-pending-conflicts" file should still exist
      expect(existsSync(join(worktreePath, '.fspec-pending-conflicts'))).toBe(
        true
      );
    });
  });

  // ========================================================================
  // Scenario: Resolution matching main exactly succeeds on re-merge
  // (Feature file L107)
  // ========================================================================
  describe('Scenario: Resolution matching main exactly succeeds on re-merge', () => {
    it('should succeed when resolution matches main exactly (TS → Rust → TS)', async () => {
      // @step Given a session worktree with pending conflicts listing "README.md"
      const { sessionId, worktreePath } =
        await fixture.createIsolatedSession('BUG-099 Match Main');

      await writeFile(join(worktreePath, 'README.md'), 'session-edit\n');
      const mainContent = 'main-edit\n';
      await writeFile(join(fixture.testDir, 'README.md'), mainContent);

      const firstCtx = createTestContext(fixture, sessionId, { worktreePath });
      await handleMergeWorktree(firstCtx.ctx);

      // @step And the worktree "README.md" has been resolved to match main exactly
      await writeFile(join(worktreePath, 'README.md'), mainContent);

      // @step When apply_session_changes is called
      const secondCtx = createTestContext(fixture, sessionId, {
        worktreePath,
      });
      await handleMergeWorktree(secondCtx.ctx);

      // @step Then the merge should succeed
      expect(secondCtx.calls.actionPromptSet).not.toBeNull();
      expect(secondCtx.calls.llmContextInjected.length).toBe(0);
    });
  });

  // ========================================================================
  // Scenario: Pending conflicts state file is not collected as a worktree file
  // (Feature file L115)
  // ========================================================================
  describe('Scenario: Pending conflicts state file is not collected as a worktree file', () => {
    it('should exclude .fspec-pending-conflicts from session diff (TS → Rust → TS)', async () => {
      // @step Given a session worktree with a ".fspec-pending-conflicts" file present
      const { sessionId, worktreePath } = await fixture.createIsolatedSession(
        'BUG-099 Exclude State'
      );

      // Write both a real change and the state file
      await writeFile(
        join(worktreePath, 'README.md'),
        '# Modified by session\n'
      );
      await writeFile(
        join(worktreePath, '.fspec-pending-conflicts'),
        '{"files":["README.md"]}'
      );

      // @step When worktree files are collected for diff or apply
      const { inspectSessionChanges } = await import(
        '../../services/sessionService'
      );
      const diff = inspectSessionChanges(fixture.testDir, sessionId);

      // @step Then ".fspec-pending-conflicts" should NOT appear in the collected file list
      const allFiles = [
        ...diff.filesChanged,
        ...diff.filesAdded,
        ...diff.filesDeleted,
      ];
      expect(
        allFiles.some((f: string) => f.includes('fspec-pending-conflicts'))
      ).toBe(false);

      // The diff SHOULD contain the README.md change
      expect(diff.filesChanged.length + diff.filesAdded.length).toBeGreaterThan(
        0
      );
    });
  });
});
