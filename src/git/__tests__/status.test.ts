/**
 * Feature: spec/features/replace-git-cli-usage-with-isomorphic-git-library.feature
 *
 * This test file validates the git status abstraction layer using isomorphic-git.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 *
 * Uses memfs for in-memory filesystem testing (fast, isolated, deterministic).
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { vol } from 'memfs';
import git from 'isomorphic-git';
import {
  getStagedFiles,
  getUnstagedFiles,
  getUntrackedFiles,
  getFileStatus,
} from '../status';

describe('Feature: Replace git CLI usage with isomorphic-git library', () => {
  const fs = vol as any;

  beforeEach(() => {
    // Reset in-memory filesystem before each test
    vol.reset();
  });

  describe('Scenario: Get staged and unstaged files for virtual hooks', () => {
    it('should return staged files when files are added to index', async () => {
      // Given a git repository with src/git/status.ts module
      vol.fromJSON({
        '/repo/file1.txt': 'content 1',
        '/repo/file2.txt': 'content 2',
        '/repo/file3.txt': 'content 3',
      });

      await git.init({ fs, dir: '/repo', defaultBranch: 'main' });

      // And the repository has staged files
      await git.add({ fs, dir: '/repo', filepath: 'file1.txt' });
      await git.add({ fs, dir: '/repo', filepath: 'file2.txt' });

      // When git-context.ts calls getStagedFiles()
      const stagedFiles = await getStagedFiles('/repo', { fs });

      // Then it receives arrays of filenames from isomorphic-git status operations
      expect(stagedFiles).toContain('file1.txt');
      expect(stagedFiles).toContain('file2.txt');
      expect(stagedFiles).not.toContain('file3.txt'); // Not staged
      expect(stagedFiles.length).toBe(2);
    });

    it('should return unstaged files when files are modified but not added', async () => {
      // Given a git repository with committed files
      vol.fromJSON({
        '/repo/committed.txt': 'initial content',
        '/repo/modified.txt': 'initial content',
      });

      await git.init({ fs, dir: '/repo', defaultBranch: 'main' });
      await git.add({ fs, dir: '/repo', filepath: 'committed.txt' });
      await git.add({ fs, dir: '/repo', filepath: 'modified.txt' });
      await git.commit({
        fs,
        dir: '/repo',
        message: 'Initial commit',
        author: { name: 'Test', email: 'test@test.com' },
      });

      // And the repository has unstaged files (modified after commit)
      fs.writeFileSync('/repo/modified.txt', 'changed content');

      // When git-context.ts calls getUnstagedFiles()
      const unstagedFiles = await getUnstagedFiles('/repo', { fs });

      // Then it receives arrays of filenames
      expect(unstagedFiles).toContain('modified.txt');
      expect(unstagedFiles).not.toContain('committed.txt'); // Not modified
      expect(unstagedFiles.length).toBe(1);

      // And the files are passed to virtual hook scripts (e.g., eslint)
      // And no git CLI commands are executed via execa
      // (This is verified by the fact that we're using memfs - git CLI can't access it)
    });
  });

  describe('Scenario: Detect all file changes for checkpoint system', () => {
    it('should detect all types of file changes for checkpointing', async () => {
      // Given a git repository with modified, staged, and untracked files
      vol.fromJSON({
        '/repo/committed.txt': 'initial',
        '/repo/will-modify.txt': 'initial',
        '/repo/will-stage.txt': 'initial',
      });

      await git.init({ fs, dir: '/repo', defaultBranch: 'main' });

      // Create initial commit
      await git.add({ fs, dir: '/repo', filepath: 'committed.txt' });
      await git.add({ fs, dir: '/repo', filepath: 'will-modify.txt' });
      await git.add({ fs, dir: '/repo', filepath: 'will-stage.txt' });
      await git.commit({
        fs,
        dir: '/repo',
        message: 'Initial commit',
        author: { name: 'Test', email: 'test@test.com' },
      });

      // Create different types of changes
      fs.writeFileSync('/repo/will-modify.txt', 'modified but not staged');
      fs.writeFileSync('/repo/will-stage.txt', 'modified and staged');
      await git.add({ fs, dir: '/repo', filepath: 'will-stage.txt' });
      fs.writeFileSync('/repo/untracked.txt', 'new file');

      // When checkpoint system (GIT-002) calls git status operations
      const staged = await getStagedFiles('/repo', { fs });
      const unstaged = await getUnstagedFiles('/repo', { fs });
      const untracked = await getUntrackedFiles('/repo', { fs });

      // Then getStagedFiles() returns list of staged files
      expect(staged).toContain('will-stage.txt');
      expect(staged.length).toBe(1);

      // And getUnstagedFiles() returns list of modified but unstaged files
      expect(unstaged).toContain('will-modify.txt');
      expect(unstaged.length).toBe(1);

      // And getUntrackedFiles() returns list of untracked files
      expect(untracked).toContain('untracked.txt');
      expect(untracked.length).toBe(1);

      // And all operations use isomorphic-git instead of git CLI
      // (Verified by memfs usage - CLI can't access in-memory fs)
    });
  });

  describe('Scenario: Handle empty repository without crashing', () => {
    it('should handle repository with no commits gracefully', async () => {
      // Given a git repository created with git init
      await git.init({ fs, dir: '/repo', defaultBranch: 'main' });

      // And the repository has no commits (no HEAD)
      // (Just initialized, no commits made)

      // And the repository contains untracked files
      vol.fromJSON({
        '/repo/README.md': '# New Project',
        '/repo/src/index.ts': 'export {}',
      });

      // When git status operations are called
      const getStatusPromise = async () => {
        const staged = await getStagedFiles('/repo', { fs });
        const unstaged = await getUnstagedFiles('/repo', { fs });
        const untracked = await getUntrackedFiles('/repo', { fs });
        return { staged, unstaged, untracked };
      };

      // Then operations complete successfully without errors
      await expect(getStatusPromise()).resolves.toBeDefined();

      const result = await getStatusPromise();

      // And all files are correctly identified as untracked
      expect(result.untracked).toContain('README.md');
      expect(result.untracked).toContain('src/index.ts');
      expect(result.staged).toEqual([]);
      expect(result.unstaged).toEqual([]);

      // And no crashes occur from missing HEAD reference
      // (Test passes = no crash)
    });

    it('should allow staging files in empty repository', async () => {
      // Given empty repo
      await git.init({ fs, dir: '/repo', defaultBranch: 'main' });
      vol.fromJSON({ '/repo/file.txt': 'content' });

      // When we add files before any commits
      await git.add({ fs, dir: '/repo', filepath: 'file.txt' });

      // Then operations work correctly
      const staged = await getStagedFiles('/repo', { fs });
      expect(staged).toContain('file.txt');
    });
  });

  describe('Scenario: Respect .gitignore when listing files', () => {
    it('should exclude files matching .gitignore patterns', async () => {
      // Given a git repository with .gitignore file
      // And the .gitignore excludes node_modules/ and *.log files
      vol.fromJSON({
        '/repo/.gitignore': 'node_modules/\n*.log\ndist/',
        '/repo/src/index.ts': 'code',
        '/repo/README.md': 'docs',
        '/repo/debug.log': 'logs',
        '/repo/error.log': 'errors',
        '/repo/node_modules/package.json': '{}',
        '/repo/dist/output.js': 'compiled',
      });

      await git.init({ fs, dir: '/repo', defaultBranch: 'main' });

      // And the repository contains both tracked and ignored files
      // (Files created above)

      // When git status operations are called
      const untracked = await getUntrackedFiles('/repo', { fs });

      // Then results exclude files matching .gitignore patterns
      // And node_modules/ files are not listed
      expect(untracked).not.toContain('node_modules/package.json');

      // And *.log files are not listed
      expect(untracked).not.toContain('debug.log');
      expect(untracked).not.toContain('error.log');

      // And dist/ files are not listed
      expect(untracked).not.toContain('dist/output.js');

      // And only non-ignored files appear in status results
      expect(untracked).toContain('src/index.ts');
      expect(untracked).toContain('README.md');
      expect(untracked).toContain('.gitignore');
    });
  });

  describe('Scenario: Return empty arrays for clean repository', () => {
    it('should return empty arrays when repository is clean', async () => {
      // Given a git repository with committed files
      vol.fromJSON({
        '/repo/file1.txt': 'content 1',
        '/repo/file2.txt': 'content 2',
      });

      await git.init({ fs, dir: '/repo', defaultBranch: 'main' });
      await git.add({ fs, dir: '/repo', filepath: 'file1.txt' });
      await git.add({ fs, dir: '/repo', filepath: 'file2.txt' });
      await git.commit({
        fs,
        dir: '/repo',
        message: 'Initial commit',
        author: { name: 'Test', email: 'test@test.com' },
      });

      // And the working directory matches HEAD (no changes)
      // (No modifications made after commit)

      // When getStagedFiles(), getUnstagedFiles(), and getUntrackedFiles() are called
      const staged = await getStagedFiles('/repo', { fs });
      const unstaged = await getUnstagedFiles('/repo', { fs });
      const untracked = await getUntrackedFiles('/repo', { fs });

      // Then all functions return empty arrays
      expect(staged).toEqual([]);
      expect(unstaged).toEqual([]);
      expect(untracked).toEqual([]);

      // And operations complete efficiently without errors
      // (Test passes = no errors)

      // And no unnecessary processing occurs for clean state
      // (Verified by test completing quickly)
    });
  });

  describe('Additional tests for FileStatus wrapper type', () => {
    it('should return semantic FileStatus object with boolean flags', async () => {
      // Test that we use semantic wrapper types instead of raw StatusRow
      vol.fromJSON({
        '/repo/file.txt': 'initial',
      });

      await git.init({ fs, dir: '/repo', defaultBranch: 'main' });
      await git.add({ fs, dir: '/repo', filepath: 'file.txt' });
      await git.commit({
        fs,
        dir: '/repo',
        message: 'Initial',
        author: { name: 'Test', email: 'test@test.com' },
      });

      fs.writeFileSync('/repo/file.txt', 'modified');

      // Get status for specific file
      const status = await getFileStatus('/repo', 'file.txt', { fs });

      // Should return semantic type, not raw [filepath, HEAD, WORKDIR, STAGE] array
      expect(status).toHaveProperty('filepath');
      expect(status).toHaveProperty('staged');
      expect(status).toHaveProperty('modified');
      expect(status).toHaveProperty('untracked');

      // Values should be semantic booleans
      expect(status?.filepath).toBe('file.txt');
      expect(status?.staged).toBe(false);
      expect(status?.modified).toBe(true);
      expect(status?.untracked).toBe(false);
    });
  });

  describe('Error handling with configurable strict mode', () => {
    it('should throw error in strict mode when directory is not a git repo', async () => {
      // Given a directory that is not a git repository
      vol.fromJSON({
        '/not-a-repo/file.txt': 'content',
      });

      // When operations are called in strict mode
      // Then should throw error (strict mode enabled)
      await expect(getStagedFiles('/not-a-repo', { fs, strict: true }))
        .rejects
        .toThrow();
    });

    it('should return empty array in non-strict mode when directory is not a git repo', async () => {
      // Given a directory that is not a git repository
      vol.fromJSON({
        '/not-a-repo/file.txt': 'content',
      });

      // When operations are called in non-strict mode (default)
      const staged = await getStagedFiles('/not-a-repo', { fs });

      // Then should return empty array (silent failure)
      expect(staged).toEqual([]);
    });
  });
});
