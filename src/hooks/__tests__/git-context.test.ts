/**
 * Tests for git context detection
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, writeFile, mkdir } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { execa } from 'execa';
import { getGitContext } from '../git-context';

describe('Git context detection', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-git-context-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('getGitContext', () => {
    it('should return empty arrays when not in a git repository', async () => {
      const context = await getGitContext(testDir);

      expect(context.stagedFiles).toEqual([]);
      expect(context.unstagedFiles).toEqual([]);
    });

    it('should detect staged files', async () => {
      // Initialize git repo
      await execa('git', ['init'], { cwd: testDir });
      await execa('git', ['config', 'user.email', 'test@example.com'], {
        cwd: testDir,
      });
      await execa('git', ['config', 'user.name', 'Test User'], {
        cwd: testDir,
      });

      // Create and stage files
      await writeFile(join(testDir, 'file1.txt'), 'content1');
      await writeFile(join(testDir, 'file2.txt'), 'content2');
      await execa('git', ['add', 'file1.txt', 'file2.txt'], { cwd: testDir });

      const context = await getGitContext(testDir);

      expect(context.stagedFiles).toContain('file1.txt');
      expect(context.stagedFiles).toContain('file2.txt');
      expect(context.stagedFiles).toHaveLength(2);
      expect(context.unstagedFiles).toEqual([]);
    });

    it('should detect unstaged files', async () => {
      // Initialize git repo
      await execa('git', ['init'], { cwd: testDir });
      await execa('git', ['config', 'user.email', 'test@example.com'], {
        cwd: testDir,
      });
      await execa('git', ['config', 'user.name', 'Test User'], {
        cwd: testDir,
      });

      // Create, commit, then modify file
      await writeFile(join(testDir, 'file1.txt'), 'original');
      await execa('git', ['add', 'file1.txt'], { cwd: testDir });
      await execa('git', ['commit', '-m', 'initial'], { cwd: testDir });

      // Modify file (now unstaged)
      await writeFile(join(testDir, 'file1.txt'), 'modified');

      const context = await getGitContext(testDir);

      expect(context.stagedFiles).toEqual([]);
      expect(context.unstagedFiles).toContain('file1.txt');
      expect(context.unstagedFiles).toHaveLength(1);
    });

    it('should detect both staged and unstaged files', async () => {
      // Initialize git repo
      await execa('git', ['init'], { cwd: testDir });
      await execa('git', ['config', 'user.email', 'test@example.com'], {
        cwd: testDir,
      });
      await execa('git', ['config', 'user.name', 'Test User'], {
        cwd: testDir,
      });

      // Create and commit initial file
      await writeFile(join(testDir, 'file1.txt'), 'original');
      await execa('git', ['add', 'file1.txt'], { cwd: testDir });
      await execa('git', ['commit', '-m', 'initial'], { cwd: testDir });

      // Create new staged file
      await writeFile(join(testDir, 'file2.txt'), 'new file');
      await execa('git', ['add', 'file2.txt'], { cwd: testDir });

      // Modify existing file (unstaged)
      await writeFile(join(testDir, 'file1.txt'), 'modified');

      const context = await getGitContext(testDir);

      expect(context.stagedFiles).toContain('file2.txt');
      expect(context.stagedFiles).toHaveLength(1);
      expect(context.unstagedFiles).toContain('file1.txt');
      expect(context.unstagedFiles).toHaveLength(1);
    });

    it('should handle files in subdirectories', async () => {
      // Initialize git repo
      await execa('git', ['init'], { cwd: testDir });
      await execa('git', ['config', 'user.email', 'test@example.com'], {
        cwd: testDir,
      });
      await execa('git', ['config', 'user.name', 'Test User'], {
        cwd: testDir,
      });

      // Create file in subdirectory
      await mkdir(join(testDir, 'src'), { recursive: true });
      await writeFile(join(testDir, 'src', 'index.ts'), 'code');
      await execa('git', ['add', 'src/index.ts'], { cwd: testDir });

      const context = await getGitContext(testDir);

      expect(context.stagedFiles).toContain('src/index.ts');
      expect(context.stagedFiles).toHaveLength(1);
    });

    it('should return empty arrays when no changes exist', async () => {
      // Initialize git repo with a commit
      await execa('git', ['init'], { cwd: testDir });
      await execa('git', ['config', 'user.email', 'test@example.com'], {
        cwd: testDir,
      });
      await execa('git', ['config', 'user.name', 'Test User'], {
        cwd: testDir,
      });
      await writeFile(join(testDir, 'file.txt'), 'content');
      await execa('git', ['add', 'file.txt'], { cwd: testDir });
      await execa('git', ['commit', '-m', 'initial'], { cwd: testDir });

      const context = await getGitContext(testDir);

      expect(context.stagedFiles).toEqual([]);
      expect(context.unstagedFiles).toEqual([]);
    });
  });
});
