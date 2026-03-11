/**
 * Feature: spec/features/git-repository-operations.feature
 *
 * This test file validates the gitoxide-backed git operations via NAPI-RS bindings.
 * Tests map directly to Gherkin scenarios defined in the feature file.
 *
 * NOTE: Unlike previous isomorphic-git tests, these use real temporary directories
 * because gitoxide is a native Rust library accessed via NAPI bindings.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, writeFile, mkdir } from 'fs/promises';
import { execSync } from 'child_process';
import { join } from 'path';
import { tmpdir } from 'os';

// Import the gitoxide-backed operations
// NOTE: These imports will FAIL until the native bindings are implemented
import {
  getStagedFiles,
  getUnstagedFiles,
  getUntrackedFiles,
  getFileDiff,
  getCurrentBranch,
} from '@sengac/codelet-napi';

describe('Feature: Git Repository Operations', () => {
  let tempDir: string;

  beforeEach(async () => {
    // Create a unique temporary directory for each test
    tempDir = await mkdtemp(join(tmpdir(), 'fspec-git-test-'));

    // Initialize a git repository
    execSync('git init', { cwd: tempDir });
    execSync('git config user.email "test@example.com"', { cwd: tempDir });
    execSync('git config user.name "Test User"', { cwd: tempDir });
  });

  afterEach(async () => {
    // Clean up temporary directory
    await rm(tempDir, { recursive: true, force: true });
  });

  describe('Scenario: Query staged files from repository', () => {
    it('should return staged files when files are added to the index', async () => {
      // @step Given a git repository with files staged using git add
      await writeFile(join(tempDir, 'staged-file.ts'), 'export const x = 1;');
      await writeFile(join(tempDir, 'unstaged-file.ts'), 'export const y = 2;');
      execSync('git add staged-file.ts', { cwd: tempDir });

      // @step When I call getStagedFiles()
      const stagedFiles = await getStagedFiles(tempDir);

      // @step Then I receive a list of file paths that are staged for commit
      expect(stagedFiles).toContain('staged-file.ts');
      expect(stagedFiles).not.toContain('unstaged-file.ts');
      expect(stagedFiles.length).toBe(1);
    });
  });

  describe('Scenario: Query unstaged files from repository', () => {
    it('should return modified files not yet staged', async () => {
      // @step Given a git repository with modified files not yet staged
      await writeFile(join(tempDir, 'file.ts'), 'initial content');
      execSync('git add file.ts', { cwd: tempDir });
      execSync('git commit -m "initial"', { cwd: tempDir });

      // Modify the file without staging
      await writeFile(join(tempDir, 'file.ts'), 'modified content');

      // @step When I call getUnstagedFiles()
      const unstagedFiles = await getUnstagedFiles(tempDir);

      // @step Then I receive a list of modified files not yet added to the index
      expect(unstagedFiles).toContain('file.ts');
      expect(unstagedFiles.length).toBe(1);
    });
  });

  describe('Scenario: Query untracked files from repository', () => {
    it('should return new files not tracked by git', async () => {
      // @step Given a git repository with new files not yet added to git
      await writeFile(join(tempDir, 'tracked.ts'), 'tracked');
      execSync('git add tracked.ts', { cwd: tempDir });
      execSync('git commit -m "initial"', { cwd: tempDir });

      // Create untracked file
      await writeFile(join(tempDir, 'untracked.ts'), 'new file');

      // @step When I call getUntrackedFiles()
      const untrackedFiles = await getUntrackedFiles(tempDir);

      // @step Then I receive a list of new files not tracked by git
      expect(untrackedFiles).toContain('untracked.ts');
      expect(untrackedFiles).not.toContain('tracked.ts');
    });
  });

  describe('Scenario: Request unified diff for changed file', () => {
    it('should return unified diff showing added and removed lines', async () => {
      // @step Given a git repository with a modified file
      await writeFile(
        join(tempDir, 'diff-test.ts'),
        'line 1\nline 2\nline 3\n'
      );
      execSync('git add diff-test.ts', { cwd: tempDir });
      execSync('git commit -m "initial"', { cwd: tempDir });

      // Modify the file
      await writeFile(
        join(tempDir, 'diff-test.ts'),
        'line 1\nmodified line 2\nline 3\nnew line 4\n'
      );

      // @step When I call getFileDiff() for that file
      const diff = await getFileDiff(tempDir, 'diff-test.ts');

      // @step Then I receive unified diff format output showing added and removed lines
      expect(diff).not.toBeNull();
      expect(diff).toContain('-line 2');
      expect(diff).toContain('+modified line 2');
      expect(diff).toContain('+new line 4');
    });
  });

  describe('Scenario: Detect and handle binary files in diff', () => {
    it('should identify binary files and exclude from text diff', async () => {
      // @step Given a git repository with a modified binary file
      // Create a binary file (PNG header bytes)
      const binaryContent = Buffer.from([
        0x89,
        0x50,
        0x4e,
        0x47,
        0x0d,
        0x0a,
        0x1a,
        0x0a, // PNG signature
        0x00,
        0x00,
        0x00,
        0x0d, // IHDR chunk length
      ]);
      await writeFile(join(tempDir, 'image.png'), binaryContent);
      execSync('git add image.png', { cwd: tempDir });
      execSync('git commit -m "add binary"', { cwd: tempDir });

      // Modify the binary file
      const modifiedBinary = Buffer.concat([
        binaryContent,
        Buffer.from([0x00, 0x01, 0x02]),
      ]);
      await writeFile(join(tempDir, 'image.png'), modifiedBinary);

      // @step When I call getFileDiff() for the binary file
      const diff = await getFileDiff(tempDir, 'image.png');

      // @step Then the file is identified as binary and excluded from text diff output
      expect(diff).toContain('Binary');
    });
  });

  describe('Scenario: Query current branch name', () => {
    it('should return the active branch name', async () => {
      // @step Given a git repository checked out to a branch
      await writeFile(join(tempDir, 'file.ts'), 'content');
      execSync('git add file.ts', { cwd: tempDir });
      execSync('git commit -m "initial"', { cwd: tempDir });
      execSync('git checkout -b feature-branch', { cwd: tempDir });

      // @step When I call getCurrentBranch()
      const branch = await getCurrentBranch(tempDir);

      // @step Then I receive the active branch name or detached HEAD state
      expect(branch).toBe('feature-branch');
    });

    it('should handle detached HEAD state', async () => {
      // @step Given a git repository checked out to a branch
      await writeFile(join(tempDir, 'file.ts'), 'content');
      execSync('git add file.ts', { cwd: tempDir });
      execSync('git commit -m "initial"', { cwd: tempDir });
      const commitHash = execSync('git rev-parse HEAD', { cwd: tempDir })
        .toString()
        .trim();
      execSync(`git checkout ${commitHash}`, { cwd: tempDir });

      // @step When I call getCurrentBranch()
      const branch = await getCurrentBranch(tempDir);

      // @step Then I receive the active branch name or detached HEAD state
      // NAPI returns null for Rust None values
      expect(branch).toBeNull(); // Detached HEAD returns null
    });
  });

  describe('Scenario: Maintain TypeScript API compatibility', () => {
    it('should maintain same function signatures as previous TypeScript API', async () => {
      // @step Given the existing TypeScript API
      // The old API signatures:
      // - getStagedFiles(dir: string, options?: GitStatusOptions): Promise<string[]>
      // - getUnstagedFiles(dir: string, options?: GitStatusOptions): Promise<string[]>
      // - getUntrackedFiles(dir: string, options?: GitStatusOptions): Promise<string[]>
      // - getFileDiff(cwd: string, filepath: string): Promise<string | null>
      // - getCurrentBranch(dir: string, options?: GitStatusOptions): Promise<string | undefined>

      // @step When the gitoxide implementation is substituted
      // Functions are imported from '@sengac/codelet-napi' instead of '../status'

      // @step Then all existing function signatures remain unchanged
      expect(typeof getStagedFiles).toBe('function');
      expect(typeof getUnstagedFiles).toBe('function');
      expect(typeof getUntrackedFiles).toBe('function');
      expect(typeof getFileDiff).toBe('function');
      expect(typeof getCurrentBranch).toBe('function');

      // @step And all existing consumers continue to work without modification
      // Verify functions accept the same parameters
      await writeFile(join(tempDir, 'test.ts'), 'content');
      execSync('git add test.ts', { cwd: tempDir });

      // Call with same signature as before
      const staged = await getStagedFiles(tempDir);
      const unstaged = await getUnstagedFiles(tempDir);
      const untracked = await getUntrackedFiles(tempDir);
      const branch = await getCurrentBranch(tempDir);

      // Results should be arrays/strings as expected
      expect(Array.isArray(staged)).toBe(true);
      expect(Array.isArray(unstaged)).toBe(true);
      expect(Array.isArray(untracked)).toBe(true);
      // Branch can be string, null, or undefined
      expect(
        typeof branch === 'string' || branch === null || branch === undefined
      ).toBe(true);
    });
  });
});
