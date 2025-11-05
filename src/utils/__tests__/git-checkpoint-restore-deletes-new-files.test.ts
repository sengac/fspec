/**
 * Feature: spec/features/checkpoint-restore-shows-file-not-found-but-doesn-t-delete-files-added-after-checkpoint.feature
 *
 * Tests for GIT-012: Checkpoint restore shows 'File not found' but doesn't delete files added after checkpoint
 *
 * CRITICAL: These tests are written BEFORE implementation (ACDD red phase).
 * All tests MUST FAIL initially to prove they actually test something.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile, readFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import * as git from 'isomorphic-git';
import fs from 'fs';
import { createCheckpoint, restoreCheckpoint } from '../git-checkpoint';
import { getCheckpointFileDiff } from '../../git/diff';

describe("Feature: Checkpoint restore shows file not found but doesn't delete files added after checkpoint", () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-checkpoint-delete-test-'));

    // Initialize git repository
    await git.init({ fs, dir: testDir, defaultBranch: 'main' });

    // Configure git
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

    // Create initial directory structure
    await mkdir(join(testDir, 'spec'), { recursive: true });

    // Create work-units.json
    await writeFile(
      join(testDir, 'spec', 'work-units.json'),
      '{"version":"1.0","workUnits":{}}'
    );
    await git.add({ fs, dir: testDir, filepath: 'spec/work-units.json' });
    await git.commit({
      fs,
      dir: testDir,
      message: 'Initialize fspec',
      author: { name: 'Test User', email: 'test@example.com' },
    });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Restore checkpoint deletes files added after checkpoint creation', () => {
    it('should delete files added after checkpoint and restore original files', async () => {
      // @step Given a checkpoint "baseline" was created containing files "A.txt" and "B.txt"
      await writeFile(join(testDir, 'A.txt'), 'Content A');
      await writeFile(join(testDir, 'B.txt'), 'Content B');

      // Create checkpoint with uncommitted files
      const checkpointResult = await createCheckpoint({
        workUnitId: 'TEST-001',
        checkpointName: 'baseline',
        cwd: testDir,
        includeUntracked: true,
      });
      expect(checkpointResult.success).toBe(true);

      // Now commit the checkpoint to HEAD
      await git.add({ fs, dir: testDir, filepath: 'A.txt' });
      await git.add({ fs, dir: testDir, filepath: 'B.txt' });
      await git.commit({
        fs,
        dir: testDir,
        message: 'Add A and B',
        author: { name: 'Test User', email: 'test@example.com' },
      });

      // @step And a new file "C.txt" was added after the checkpoint
      await writeFile(
        join(testDir, 'C.txt'),
        'Content C - added after checkpoint'
      );
      await git.add({ fs, dir: testDir, filepath: 'C.txt' });
      await git.commit({
        fs,
        dir: testDir,
        message: 'Add C',
        author: { name: 'Test User', email: 'test@example.com' },
      });

      // Verify C.txt exists before restore
      const cExists = fs.existsSync(join(testDir, 'C.txt'));
      expect(cExists).toBe(true);

      // @step When I restore checkpoint "baseline"
      const restoreResult = await restoreCheckpoint({
        workUnitId: 'TEST-001',
        checkpointName: 'baseline',
        cwd: testDir,
        force: true, // Force to skip conflict detection for test
      });

      // @step Then file "C.txt" should be deleted from the working directory
      const cExistsAfterRestore = fs.existsSync(join(testDir, 'C.txt'));
      expect(cExistsAfterRestore).toBe(false);

      // @step And files "A.txt" and "B.txt" should be restored to their checkpoint state
      const aContent = await readFile(join(testDir, 'A.txt'), 'utf-8');
      const bContent = await readFile(join(testDir, 'B.txt'), 'utf-8');
      expect(aContent).toBe('Content A');
      expect(bContent).toBe('Content B');

      // @step And the working directory should match the exact state at checkpoint creation
      expect(restoreResult.success).toBe(true);
    });
  });

  describe('Scenario: Diff viewer shows clear deletion message for files not in checkpoint', () => {
    it('should show "Will be deleted on restore" message instead of "File not found in checkpoint"', async () => {
      // @step Given a checkpoint "baseline" exists
      await writeFile(join(testDir, 'existing.txt'), 'Existing file');

      // Create checkpoint with uncommitted file
      const checkpointResult = await createCheckpoint({
        workUnitId: 'TEST-002',
        checkpointName: 'baseline',
        cwd: testDir,
        includeUntracked: true,
      });
      expect(checkpointResult.success).toBe(true);

      // Commit the file to HEAD
      await git.add({ fs, dir: testDir, filepath: 'existing.txt' });
      await git.commit({
        fs,
        dir: testDir,
        message: 'Add existing file',
        author: { name: 'Test User', email: 'test@example.com' },
      });

      // Get checkpoint ref
      const checkpointRef = `refs/fspec-checkpoints/TEST-002/baseline`;
      const checkpointOid = await git.resolveRef({
        fs,
        dir: testDir,
        ref: checkpointRef,
      });

      // @step And file "D.txt" exists in HEAD but not in the checkpoint
      await writeFile(join(testDir, 'D.txt'), 'File D - not in checkpoint');
      await git.add({ fs, dir: testDir, filepath: 'D.txt' });
      await git.commit({
        fs,
        dir: testDir,
        message: 'Add D after checkpoint',
        author: { name: 'Test User', email: 'test@example.com' },
      });

      // @step When I view the checkpoint diff for "D.txt"
      const diff = await getCheckpointFileDiff(testDir, 'D.txt', checkpointRef);

      // @step Then the diff should show "Will be deleted on restore" instead of "File not found in checkpoint"
      expect(diff).toBeDefined();
      expect(diff).not.toContain('File not found in checkpoint');
      expect(diff).toContain('Will be deleted on restore');

      // @step And the message should clearly indicate the file will be removed during restoration
      expect(diff).toMatch(/delete|remov/i);
    });
  });

  describe('Scenario: Restore checkpoint deletes multiple new files and restores modified files', () => {
    it('should delete multiple new files and restore modified files to checkpoint state', async () => {
      // @step Given a checkpoint "before-changes" was created
      await writeFile(join(testDir, 'main.ts'), 'const main = "original";');
      await writeFile(join(testDir, 'config.json'), '{"version": "1.0"}');

      // Create checkpoint with uncommitted files
      const checkpointResult = await createCheckpoint({
        workUnitId: 'TEST-003',
        checkpointName: 'before-changes',
        cwd: testDir,
        includeUntracked: true,
      });
      expect(checkpointResult.success).toBe(true);

      // Commit the files to HEAD
      await git.add({ fs, dir: testDir, filepath: 'main.ts' });
      await git.add({ fs, dir: testDir, filepath: 'config.json' });
      await git.commit({
        fs,
        dir: testDir,
        message: 'Initial state',
        author: { name: 'Test User', email: 'test@example.com' },
      });

      // @step And 3 new files were added after checkpoint: "new-feature.ts", "test.spec.ts", "README-draft.md"
      await writeFile(
        join(testDir, 'new-feature.ts'),
        'export const newFeature = true;'
      );
      await writeFile(
        join(testDir, 'test.spec.ts'),
        'describe("test", () => {});'
      );
      await writeFile(join(testDir, 'README-draft.md'), '# Draft README');
      await git.add({ fs, dir: testDir, filepath: 'new-feature.ts' });
      await git.add({ fs, dir: testDir, filepath: 'test.spec.ts' });
      await git.add({ fs, dir: testDir, filepath: 'README-draft.md' });

      // @step And 2 files were modified after checkpoint: "main.ts", "config.json"
      await writeFile(join(testDir, 'main.ts'), 'const main = "modified";');
      await writeFile(join(testDir, 'config.json'), '{"version": "2.0"}');
      await git.add({ fs, dir: testDir, filepath: 'main.ts' });
      await git.add({ fs, dir: testDir, filepath: 'config.json' });
      await git.commit({
        fs,
        dir: testDir,
        message: 'Add new files and modify existing',
        author: { name: 'Test User', email: 'test@example.com' },
      });

      // Verify all files exist before restore
      expect(fs.existsSync(join(testDir, 'new-feature.ts'))).toBe(true);
      expect(fs.existsSync(join(testDir, 'test.spec.ts'))).toBe(true);
      expect(fs.existsSync(join(testDir, 'README-draft.md'))).toBe(true);

      // @step When I restore checkpoint "before-changes"
      const restoreResult = await restoreCheckpoint({
        workUnitId: 'TEST-003',
        checkpointName: 'before-changes',
        cwd: testDir,
        force: true,
      });

      // @step Then the 3 new files should be deleted: "new-feature.ts", "test.spec.ts", "README-draft.md"
      expect(fs.existsSync(join(testDir, 'new-feature.ts'))).toBe(false);
      expect(fs.existsSync(join(testDir, 'test.spec.ts'))).toBe(false);
      expect(fs.existsSync(join(testDir, 'README-draft.md'))).toBe(false);

      // @step And the 2 modified files should be restored to checkpoint state: "main.ts", "config.json"
      const mainContent = await readFile(join(testDir, 'main.ts'), 'utf-8');
      const configContent = await readFile(
        join(testDir, 'config.json'),
        'utf-8'
      );
      expect(mainContent).toBe('const main = "original";');
      expect(configContent).toBe('{"version": "1.0"}');

      // @step And no files added after checkpoint should remain in the working directory
      expect(restoreResult.success).toBe(true);
    });
  });
});
