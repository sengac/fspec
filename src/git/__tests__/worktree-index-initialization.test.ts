/**
 * Feature: spec/features/isolated-session-worktree-initialization.feature
 *
 * E2E Integration tests for GIT-035: Worktree index initialization fix.
 * Tests TypeScript → NAPI → Rust → NAPI → TypeScript flow.
 *
 * NO MOCKS - Uses real NAPI bindings with real git repositories.
 * These tests verify the worktree index is properly initialized so
 * git status shows clean state (not all files staged for deletion).
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, writeFile, mkdir } from 'fs/promises';
import { execSync } from 'child_process';
import { join } from 'path';
import { tmpdir } from 'os';
import { existsSync } from 'fs';

import {
  createWorktree,
  removeWorktree,
  listWorktrees,
  getSessionDiff,
} from '@sengac/codelet-napi';

describe('Feature: Isolated session worktree initialization', () => {
  let tempDir: string;

  beforeEach(async () => {
    // @step Given I have a git repository with tracked files
    tempDir = await mkdtemp(join(tmpdir(), 'fspec-worktree-index-test-'));

    // Initialize git repo
    execSync('git init', { cwd: tempDir });
    execSync('git config user.email "test@example.com"', { cwd: tempDir });
    execSync('git config user.name "Test User"', { cwd: tempDir });

    // Create initial tracked files
    await mkdir(join(tempDir, 'src'), { recursive: true });
    await writeFile(join(tempDir, 'README.md'), '# Test Repository\n');
    await writeFile(join(tempDir, 'src/main.rs'), 'fn main() {}\n');
    await writeFile(join(tempDir, 'src/config.rs'), '// Config\n');

    // Stage and commit
    execSync('git add .', { cwd: tempDir });
    execSync('git commit -m "Initial commit"', { cwd: tempDir });
  });

  afterEach(async () => {
    // Clean up
    try {
      await rm(tempDir, { recursive: true, force: true });
    } catch {
      // Ignore cleanup errors
    }
  });

  describe('Scenario: Worktree has all tracked files in git index after creation', () => {
    it('should have all tracked files in git ls-files after worktree creation', async () => {
      // @step Given I have a git repository with tracked files (done in beforeEach)
      // Count files in main repo
      const mainLsFiles = execSync('git ls-files', { cwd: tempDir })
        .toString()
        .trim()
        .split('\n')
        .filter(Boolean);
      expect(mainLsFiles.length).toBeGreaterThan(0);

      // @step When I create an isolated session
      const sessionId = 'test-index-session';
      const result = createWorktree(tempDir, sessionId);
      expect(result).toBeDefined();
      expect(result.sessionId).toBe(sessionId);

      // @step Then the worktree should exist at ".fspec/worktrees/<session-id>/"
      const worktreePath = join(tempDir, '.fspec/worktrees', sessionId);
      expect(existsSync(worktreePath)).toBe(true);

      // @step And "git ls-files" in the worktree should return all tracked files
      const worktreeLsFiles = execSync('git ls-files', { cwd: worktreePath })
        .toString()
        .trim()
        .split('\n')
        .filter(Boolean);

      // @step And the file count should match the main repository
      expect(worktreeLsFiles.length).toBe(mainLsFiles.length);
      expect(worktreeLsFiles.length).toBeGreaterThan(0);

      // Verify exact files match
      for (const file of mainLsFiles) {
        expect(worktreeLsFiles).toContain(file);
      }

      // Cleanup
      removeWorktree(tempDir, sessionId);
    });
  });

  describe('Scenario: Worktree has clean git status after creation', () => {
    it('should have clean git status with no staged changes', async () => {
      // @step Given I have a git repository with tracked files (done in beforeEach)

      // @step When I create an isolated session
      const sessionId = 'test-status-session';
      const result = createWorktree(tempDir, sessionId);
      expect(result).toBeDefined();

      const worktreePath = join(tempDir, '.fspec/worktrees', sessionId);

      // @step Then "git status" in the worktree should show clean state
      const statusOutput = execSync('git status --porcelain', {
        cwd: worktreePath,
      })
        .toString()
        .trim();

      // @step And there should be no staged changes
      expect(statusOutput).toBe('');

      // Verify no staged deletions specifically (the bug we're fixing)
      const stagedDeletions = statusOutput
        .split('\n')
        .filter(line => line.startsWith('D '));
      expect(stagedDeletions.length).toBe(0);

      // Cleanup
      removeWorktree(tempDir, sessionId);
    });
  });

  describe('Scenario: Session Management Panel shows accurate file change count', () => {
    it('should show accurate file count when a file is modified', async () => {
      // @step Given I have an isolated session with a worktree
      const sessionId = 'test-diff-session';
      const result = createWorktree(tempDir, sessionId);
      expect(result).toBeDefined();

      const worktreePath = join(tempDir, '.fspec/worktrees', sessionId);

      // @step When I modify a file in the worktree
      await writeFile(
        join(worktreePath, 'src/main.rs'),
        'fn main() { println!("modified"); }\n'
      );

      // @step And I open the Session Management Panel
      // (Session Management Panel uses getSessionDiff under the hood)
      const diffResult = getSessionDiff(tempDir, sessionId);

      // @step Then the session should show "1 files changed"
      expect(diffResult.filesChanged.length).toBe(1);

      // @step And the modified file should appear in the changes list
      expect(diffResult.filesChanged).toContain('src/main.rs');

      // Verify no false positives (all files appearing as changed)
      expect(diffResult.filesAdded.length).toBe(0);
      expect(diffResult.filesDeleted.length).toBe(0);

      // Cleanup
      removeWorktree(tempDir, sessionId);
    });

    it('should show 0 files changed for unmodified worktree', async () => {
      // @step Given I have an isolated session with a worktree
      const sessionId = 'test-clean-session';
      const result = createWorktree(tempDir, sessionId);
      expect(result).toBeDefined();

      // @step And I have not modified any files

      // @step When I get the session diff
      const diffResult = getSessionDiff(tempDir, sessionId);

      // @step Then the session should show 0 files changed
      expect(diffResult.filesChanged.length).toBe(0);
      expect(diffResult.filesAdded.length).toBe(0);
      expect(diffResult.filesDeleted.length).toBe(0);

      // Cleanup
      removeWorktree(tempDir, sessionId);
    });
  });

  describe('Scenario: List worktrees returns correct information', () => {
    it('should list created worktrees with proper metadata', async () => {
      // @step Given I create multiple worktrees
      const session1 = 'test-list-session-1';
      const session2 = 'test-list-session-2';

      createWorktree(tempDir, session1);
      createWorktree(tempDir, session2);

      // @step When I list all worktrees
      const worktrees = listWorktrees(tempDir);

      // @step Then I should see both worktrees
      expect(worktrees.length).toBe(2);

      const sessionIds = worktrees.map(w => w.sessionId);
      expect(sessionIds).toContain(session1);
      expect(sessionIds).toContain(session2);

      // @step And each worktree should have proper paths
      for (const wt of worktrees) {
        expect(wt.path).toContain('.fspec/worktrees');
        expect(wt.headCommit).toBeTruthy();
        expect(wt.isDetached).toBe(true);
      }

      // Cleanup
      removeWorktree(tempDir, session1);
      removeWorktree(tempDir, session2);
    });
  });

  describe('Scenario: Worktree removal cleans up properly', () => {
    it('should remove worktree and its git metadata', async () => {
      // @step Given I create a worktree
      const sessionId = 'test-remove-session';
      createWorktree(tempDir, sessionId);

      const worktreePath = join(tempDir, '.fspec/worktrees', sessionId);
      const gitWorktreeDir = join(tempDir, '.git/worktrees', sessionId);

      expect(existsSync(worktreePath)).toBe(true);
      expect(existsSync(gitWorktreeDir)).toBe(true);

      // @step When I remove the worktree
      removeWorktree(tempDir, sessionId);

      // @step Then the worktree directory should be removed
      expect(existsSync(worktreePath)).toBe(false);

      // @step And the git metadata should be cleaned up
      expect(existsSync(gitWorktreeDir)).toBe(false);

      // @step And the worktree should not appear in list
      const worktrees = listWorktrees(tempDir);
      expect(worktrees.find(w => w.sessionId === sessionId)).toBeUndefined();
    });
  });
});
