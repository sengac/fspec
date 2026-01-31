/**
 * Tests for git context detection
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, mkdir } from 'fs/promises';
import { join } from 'path';
import { execa } from 'execa';
import { getGitContext } from '../git-context';
import {
  setupGitTest,
  type GitTestSetup,
} from '../../test-helpers/universal-test-setup';

describe('Git context detection', () => {
  let setup: GitTestSetup;

  beforeEach(async () => {
    setup = await setupGitTest('git-context');
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('getGitContext', () => {
    it('should return empty arrays when not in a git repository', async () => {
      const context = await getGitContext(setup.testDir);

      expect(context.stagedFiles).toEqual([]);
      expect(context.unstagedFiles).toEqual([]);
    });

    it('should detect staged files', async () => {
      // Initialize git repo
      await execa('git', ['init'], { cwd: setup.testDir });
      await execa('git', ['config', 'user.email', 'test@example.com'], {
        cwd: setup.testDir,
      });
      await execa('git', ['config', 'user.name', 'Test User'], {
        cwd: setup.testDir,
      });

      // Create and stage files
      await writeFile(join(setup.testDir, 'file1.txt'), 'content1');
      await writeFile(join(setup.testDir, 'file2.txt'), 'content2');
      await execa('git', ['add', 'file1.txt', 'file2.txt'], {
        cwd: setup.testDir,
      });

      const context = await getGitContext(setup.testDir);

      expect(context.stagedFiles).toContain('file1.txt');
      expect(context.stagedFiles).toContain('file2.txt');
      expect(context.stagedFiles).toHaveLength(2);
      expect(context.unstagedFiles).toEqual([]);
    });

    it('should detect unstaged files', async () => {
      // Initialize git repo
      await execa('git', ['init'], { cwd: setup.testDir });
      await execa('git', ['config', 'user.email', 'test@example.com'], {
        cwd: setup.testDir,
      });
      await execa('git', ['config', 'user.name', 'Test User'], {
        cwd: setup.testDir,
      });

      // Create, commit, then modify file
      await writeFile(join(setup.testDir, 'file1.txt'), 'original');
      await execa('git', ['add', 'file1.txt'], { cwd: setup.testDir });
      await execa('git', ['commit', '-m', 'initial'], { cwd: setup.testDir });

      // Modify file (now unstaged)
      await writeFile(join(setup.testDir, 'file1.txt'), 'modified');

      const context = await getGitContext(setup.testDir);

      expect(context.stagedFiles).toEqual([]);
      expect(context.unstagedFiles).toContain('file1.txt');
      expect(context.unstagedFiles).toHaveLength(1);
    });

    it('should detect both staged and unstaged files', async () => {
      // Initialize git repo
      await execa('git', ['init'], { cwd: setup.testDir });
      await execa('git', ['config', 'user.email', 'test@example.com'], {
        cwd: setup.testDir,
      });
      await execa('git', ['config', 'user.name', 'Test User'], {
        cwd: setup.testDir,
      });

      // Create and commit initial file
      await writeFile(join(setup.testDir, 'file1.txt'), 'original');
      await execa('git', ['add', 'file1.txt'], { cwd: setup.testDir });
      await execa('git', ['commit', '-m', 'initial'], { cwd: setup.testDir });

      // Create new staged file
      await writeFile(join(setup.testDir, 'file2.txt'), 'new file');
      await execa('git', ['add', 'file2.txt'], { cwd: setup.testDir });

      // Modify existing file (unstaged)
      await writeFile(join(setup.testDir, 'file1.txt'), 'modified');

      const context = await getGitContext(setup.testDir);

      expect(context.stagedFiles).toContain('file2.txt');
      expect(context.stagedFiles).toHaveLength(1);
      expect(context.unstagedFiles).toContain('file1.txt');
      expect(context.unstagedFiles).toHaveLength(1);
    });

    it('should handle files in subdirectories', async () => {
      // Initialize git repo
      await execa('git', ['init'], { cwd: setup.testDir });
      await execa('git', ['config', 'user.email', 'test@example.com'], {
        cwd: setup.testDir,
      });
      await execa('git', ['config', 'user.name', 'Test User'], {
        cwd: setup.testDir,
      });

      // Create file in subdirectory
      await mkdir(join(setup.testDir, 'src'), { recursive: true });
      await writeFile(join(setup.testDir, 'src', 'index.ts'), 'code');
      await execa('git', ['add', 'src/index.ts'], { cwd: setup.testDir });

      const context = await getGitContext(setup.testDir);

      expect(context.stagedFiles).toContain('src/index.ts');
      expect(context.stagedFiles).toHaveLength(1);
    });

    it('should return empty arrays when no changes exist', async () => {
      // Initialize git repo with a commit
      await execa('git', ['init'], { cwd: setup.testDir });
      await execa('git', ['config', 'user.email', 'test@example.com'], {
        cwd: setup.testDir,
      });
      await execa('git', ['config', 'user.name', 'Test User'], {
        cwd: setup.testDir,
      });
      await writeFile(join(setup.testDir, 'file.txt'), 'content');
      await execa('git', ['add', 'file.txt'], { cwd: setup.testDir });
      await execa('git', ['commit', '-m', 'initial'], { cwd: setup.testDir });

      const context = await getGitContext(setup.testDir);

      expect(context.stagedFiles).toEqual([]);
      expect(context.unstagedFiles).toEqual([]);
    });
  });
});
