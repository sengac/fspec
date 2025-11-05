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
  getStagedFilesWithChangeType,
  getUnstagedFilesWithChangeType,
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
        '/repo/committed.txt': 'initial',
        '/repo/modified.txt': 'initial',
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
      // NOTE: File size must change for isomorphic-git to detect modification in memfs
      fs.writeFileSync(
        '/repo/modified.txt',
        'changed content with different size'
      );

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
      expect(status).toHaveProperty('hasUnstagedChanges');
      expect(status).toHaveProperty('untracked');

      // Values should be semantic booleans
      expect(status?.filepath).toBe('file.txt');
      expect(status?.staged).toBe(false);
      expect(status?.hasUnstagedChanges).toBe(true);
      expect(status?.untracked).toBe(false);
    });

    it('should use clear field name hasUnstagedChanges instead of ambiguous modified (S2 Issue #3 - FIXED)', async () => {
      // This test verifies the fix for S2 Issue #3 semantic ambiguity
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

      // Modify file and stage it
      fs.writeFileSync('/repo/file.txt', 'modified and staged');
      await git.add({ fs, dir: '/repo', filepath: 'file.txt' });

      const status = await getFileStatus('/repo', 'file.txt', { fs });

      // File is staged (v2 differs from HEAD v1)
      expect(status?.staged).toBe(true);

      // Field name hasUnstagedChanges is clear: file has NO unstaged changes (all changes are staged)
      expect(status?.hasUnstagedChanges).toBe(false); // ✅ CLEAR: No unstaged changes
    });
  });

  describe('S2 Critical: Partial staging scenario (file modified after staging)', () => {
    it('should detect files in BOTH staged and unstaged when file is modified after staging', async () => {
      // Given a git repository with a committed file
      vol.fromJSON({
        '/repo/partial.txt': 'v1 initial',
      });

      await git.init({ fs, dir: '/repo', defaultBranch: 'main' });
      await git.add({ fs, dir: '/repo', filepath: 'partial.txt' });
      await git.commit({
        fs,
        dir: '/repo',
        message: 'Initial commit',
        author: { name: 'Test', email: 'test@test.com' },
      });

      // When the file is modified to v2 and staged
      // NOTE: Must change size for isomorphic-git to detect change in memfs
      fs.writeFileSync('/repo/partial.txt', 'v2 staged version here');
      await git.add({ fs, dir: '/repo', filepath: 'partial.txt' });

      // And then modified AGAIN to v3 (without staging)
      fs.writeFileSync(
        '/repo/partial.txt',
        'v3 unstaged changes after staging - much longer'
      );

      // Then the file should appear in BOTH getStagedFiles() AND getUnstagedFiles()
      // Status matrix will be: [filepath='partial.txt', HEAD=1, WORKDIR=2, STAGE=3]
      // - getStagedFiles(): stage !== head → 3 !== 1 → true ✅
      // - getUnstagedFiles(): workdir !== stage → 2 !== 3 → true ✅
      const staged = await getStagedFiles('/repo', { fs });
      const unstaged = await getUnstagedFiles('/repo', { fs });

      // File should be in staged (because v2 is staged)
      expect(staged).toContain('partial.txt');

      // File should ALSO be in unstaged (because v3 differs from staged v2)
      // VERIFIED: Current logic handles this correctly - not a bug!
      expect(unstaged).toContain('partial.txt');
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
      await expect(
        getStagedFiles('/not-a-repo', { fs, strict: true })
      ).rejects.toThrow();
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

/**
 * Feature: spec/features/changed-files-view-missing-unstaged-files.feature
 *
 * TUI-028: Changed files view missing unstaged files
 * This test suite validates that untracked files appear in the changed files view.
 */
describe('Feature: Changed files view missing unstaged files', () => {
  const fs = vol as any;

  beforeEach(() => {
    // Reset in-memory filesystem before each test
    vol.reset();
  });

  describe('Scenario: View untracked file in changed files view', () => {
    it('should show untracked files with A indicator in green', async () => {
      // @step Given I have created a new file "newfile.txt" that is untracked
      vol.fromJSON({
        '/repo/newfile.txt': 'new content',
      });
      await git.init({ fs, dir: '/repo', defaultBranch: 'main' });

      // @step When I open the changed files view with the F key
      const unstagedFiles = await getUnstagedFilesWithChangeType('/repo', {
        fs,
      });

      // @step Then I should see "newfile.txt" listed in the unstaged section
      const newfile = unstagedFiles.find(f => f.filepath === 'newfile.txt');
      expect(newfile).toBeDefined();

      // @step And the status indicator should be "A" in green color
      expect(newfile?.changeType).toBe('A');
      expect(newfile?.staged).toBe(false);
    });
  });

  describe('Scenario: View unstaged modification in changed files view', () => {
    it('should show unstaged modifications with M indicator in yellow', async () => {
      // @step Given I have modified an existing tracked file "src/index.ts"
      vol.fromJSON({
        '/repo/src/index.ts': 'initial content',
      });
      await git.init({ fs, dir: '/repo', defaultBranch: 'main' });
      await git.add({ fs, dir: '/repo', filepath: 'src/index.ts' });
      await git.commit({
        fs,
        dir: '/repo',
        message: 'Initial commit',
        author: { name: 'Test', email: 'test@test.com' },
      });

      // @step And the file has not been staged
      fs.writeFileSync(
        '/repo/src/index.ts',
        'modified content with different size'
      );

      // @step When I open the changed files view with the F key
      const unstagedFiles = await getUnstagedFilesWithChangeType('/repo', {
        fs,
      });

      // @step Then I should see "src/index.ts" listed in the unstaged section
      const indexFile = unstagedFiles.find(f => f.filepath === 'src/index.ts');
      expect(indexFile).toBeDefined();

      // @step And the status indicator should be "M" in yellow color
      expect(indexFile?.changeType).toBe('M');
      expect(indexFile?.staged).toBe(false);
    });
  });

  describe('Scenario: View staged file in changed files view', () => {
    it('should show staged files with appropriate status indicator', async () => {
      // @step Given I have staged a file "README.md" using "git add README.md"
      vol.fromJSON({
        '/repo/README.md': '# Project',
      });
      await git.init({ fs, dir: '/repo', defaultBranch: 'main' });
      await git.add({ fs, dir: '/repo', filepath: 'README.md' });

      // @step When I open the changed files view with the F key
      const stagedFiles = await getStagedFilesWithChangeType('/repo', { fs });

      // @step Then I should see "README.md" listed in the staged section
      const readme = stagedFiles.find(f => f.filepath === 'README.md');
      expect(readme).toBeDefined();

      // @step And the file should have an appropriate status indicator based on its change type
      expect(readme?.changeType).toBe('A'); // Added since it's a new file
      expect(readme?.staged).toBe(true);
    });
  });

  describe('Scenario: View deleted file in changed files view', () => {
    it('should show deleted files with D indicator in red', async () => {
      // @step Given I have deleted an existing tracked file "oldfile.ts"
      vol.fromJSON({
        '/repo/oldfile.ts': 'old content',
      });
      await git.init({ fs, dir: '/repo', defaultBranch: 'main' });
      await git.add({ fs, dir: '/repo', filepath: 'oldfile.ts' });
      await git.commit({
        fs,
        dir: '/repo',
        message: 'Initial commit',
        author: { name: 'Test', email: 'test@test.com' },
      });

      // @step And the deletion has not been staged
      fs.unlinkSync('/repo/oldfile.ts');

      // @step When I open the changed files view with the F key
      const unstagedFiles = await getUnstagedFilesWithChangeType('/repo', {
        fs,
      });

      // @step Then I should see "oldfile.ts" listed in the unstaged section
      const oldfile = unstagedFiles.find(f => f.filepath === 'oldfile.ts');
      expect(oldfile).toBeDefined();

      // @step And the status indicator should be "D" in red color
      expect(oldfile?.changeType).toBe('D');
      expect(oldfile?.staged).toBe(false);
    });
  });
});
