/**
 * Feature: spec/features/checkpoint-restore-shows-file-not-found-but-doesn-t-delete-files-added-after-checkpoint.feature
 *
 * Tests for GIT-012: Checkpoint restore shows 'File not found' but doesn't delete files added after checkpoint
 *
 * CRITICAL: These tests are written BEFORE implementation (ACDD red phase).
 * All tests MUST FAIL initially to prove they actually test something.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, readFile } from 'fs/promises';
import { join } from 'path';
import {
  gitInit,
  gitSetConfig,
  gitAdd,
  gitCommit,
  resolveRef,
} from '@sengac/codelet-napi';
import fs from 'fs';
import { createCheckpoint, restoreCheckpoint } from '../git-checkpoint';
import { getCheckpointFileDiff } from '../../git/diff';
import {
  setupGitTest,
  type GitTestSetup,
} from '../../test-helpers/universal-test-setup';

describe("Feature: Checkpoint restore shows file not found but doesn't delete files added after checkpoint", () => {
  let setup: GitTestSetup;

  beforeEach(async () => {
    setup = await setupGitTest('git-checkpoint-restore-deletes-new-files');

    // Initialize git repository
    gitInit(setup.testDir, 'main');

    // Configure git
    gitSetConfig(setup.testDir, 'user.name', 'Test User');
    gitSetConfig(setup.testDir, 'user.email', 'test@example.com');

    // Create initial directory structure
    await mkdir(join(setup.testDir, 'spec'), { recursive: true });

    // Create work-units.json
    await writeFile(
      join(setup.testDir, 'spec', 'work-units.json'),
      '{"version":"1.0","workUnits":{}}'
    );
    gitAdd(setup.testDir, 'spec/work-units.json');
    gitCommit(
      setup.testDir,
      'Initialize fspec',
      'Test User',
      'test@example.com'
    );
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: Restore checkpoint deletes files added after checkpoint creation', () => {
    it('should delete files added after checkpoint and restore original files', async () => {
      // @step Given a checkpoint "baseline" was created containing files "A.txt" and "B.txt"
      await writeFile(join(setup.testDir, 'A.txt'), 'Content A');
      await writeFile(join(setup.testDir, 'B.txt'), 'Content B');

      // Create checkpoint with uncommitted files
      const checkpointResult = await createCheckpoint({
        workUnitId: 'TEST-001',
        checkpointName: 'baseline',
        cwd: setup.testDir,
        includeUntracked: true,
      });
      expect(checkpointResult.success).toBe(true);

      // Now commit the checkpoint to HEAD
      gitAdd(setup.testDir, 'A.txt');
      gitAdd(setup.testDir, 'B.txt');
      gitCommit(setup.testDir, 'Add A and B', 'Test User', 'test@example.com');

      // @step And a new file "C.txt" was added after the checkpoint
      await writeFile(
        join(setup.testDir, 'C.txt'),
        'Content C - added after checkpoint'
      );
      gitAdd(setup.testDir, 'C.txt');
      gitCommit(setup.testDir, 'Add C', 'Test User', 'test@example.com');

      // Verify C.txt exists before restore
      const cExists = fs.existsSync(join(setup.testDir, 'C.txt'));
      expect(cExists).toBe(true);

      // @step When I restore checkpoint "baseline"
      const restoreResult = await restoreCheckpoint({
        workUnitId: 'TEST-001',
        checkpointName: 'baseline',
        cwd: setup.testDir,
        force: true, // Force to skip conflict detection for test
      });

      // @step Then file "C.txt" should be deleted from the working directory
      const cExistsAfterRestore = fs.existsSync(join(setup.testDir, 'C.txt'));
      expect(cExistsAfterRestore).toBe(false);

      // @step And files "A.txt" and "B.txt" should be restored to their checkpoint state
      const aContent = await readFile(join(setup.testDir, 'A.txt'), 'utf-8');
      const bContent = await readFile(join(setup.testDir, 'B.txt'), 'utf-8');
      expect(aContent).toBe('Content A');
      expect(bContent).toBe('Content B');

      // @step And the working directory should match the exact state at checkpoint creation
      expect(restoreResult.success).toBe(true);
    });
  });

  describe('Scenario: Diff viewer shows clear deletion message for files not in checkpoint', () => {
    it('should show "Will be deleted on restore" message instead of "File not found in checkpoint"', async () => {
      // @step Given a checkpoint "baseline" exists
      await writeFile(join(setup.testDir, 'existing.txt'), 'Existing file');

      // Create checkpoint with uncommitted file
      const checkpointResult = await createCheckpoint({
        workUnitId: 'TEST-002',
        checkpointName: 'baseline',
        cwd: setup.testDir,
        includeUntracked: true,
      });
      expect(checkpointResult.success).toBe(true);

      // Commit the file to HEAD
      gitAdd(setup.testDir, 'existing.txt');
      gitCommit(
        setup.testDir,
        'Add existing file',
        'Test User',
        'test@example.com'
      );

      // Get checkpoint ref
      const checkpointRef = `refs/fspec-checkpoints/TEST-002/baseline`;
      const checkpointOid = resolveRef(setup.testDir, checkpointRef);

      // @step And file "D.txt" exists in HEAD but not in the checkpoint
      await writeFile(
        join(setup.testDir, 'D.txt'),
        'File D - not in checkpoint'
      );
      gitAdd(setup.testDir, 'D.txt');
      gitCommit(
        setup.testDir,
        'Add D after checkpoint',
        'Test User',
        'test@example.com'
      );

      // @step When I view the checkpoint diff for "D.txt"
      const diff = await getCheckpointFileDiff(
        setup.testDir,
        'D.txt',
        checkpointRef
      );

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
      await writeFile(
        join(setup.testDir, 'main.ts'),
        'const main = "original";'
      );
      await writeFile(join(setup.testDir, 'config.json'), '{"version": "1.0"}');

      // Create checkpoint with uncommitted files
      const checkpointResult = await createCheckpoint({
        workUnitId: 'TEST-003',
        checkpointName: 'before-changes',
        cwd: setup.testDir,
        includeUntracked: true,
      });
      expect(checkpointResult.success).toBe(true);

      // Commit the files to HEAD
      gitAdd(setup.testDir, 'main.ts');
      gitAdd(setup.testDir, 'config.json');
      gitCommit(
        setup.testDir,
        'Initial state',
        'Test User',
        'test@example.com'
      );

      // @step And 3 new files were added after checkpoint: "new-feature.ts", "test.spec.ts", "README-draft.md"
      await writeFile(
        join(setup.testDir, 'new-feature.ts'),
        'export const newFeature = true;'
      );
      await writeFile(
        join(setup.testDir, 'test.spec.ts'),
        'describe("test", () => {});'
      );
      await writeFile(join(setup.testDir, 'README-draft.md'), '# Draft README');
      gitAdd(setup.testDir, 'new-feature.ts');
      gitAdd(setup.testDir, 'test.spec.ts');
      gitAdd(setup.testDir, 'README-draft.md');

      // @step And 2 files were modified after checkpoint: "main.ts", "config.json"
      await writeFile(
        join(setup.testDir, 'main.ts'),
        'const main = "modified";'
      );
      await writeFile(join(setup.testDir, 'config.json'), '{"version": "2.0"}');
      gitAdd(setup.testDir, 'main.ts');
      gitAdd(setup.testDir, 'config.json');
      gitCommit(
        setup.testDir,
        'Add new files and modify existing',
        'Test User',
        'test@example.com'
      );

      // Verify all files exist before restore
      expect(fs.existsSync(join(setup.testDir, 'new-feature.ts'))).toBe(true);
      expect(fs.existsSync(join(setup.testDir, 'test.spec.ts'))).toBe(true);
      expect(fs.existsSync(join(setup.testDir, 'README-draft.md'))).toBe(true);

      // @step When I restore checkpoint "before-changes"
      const restoreResult = await restoreCheckpoint({
        workUnitId: 'TEST-003',
        checkpointName: 'before-changes',
        cwd: setup.testDir,
        force: true,
      });

      // @step Then the 3 new files should be deleted: "new-feature.ts", "test.spec.ts", "README-draft.md"
      expect(fs.existsSync(join(setup.testDir, 'new-feature.ts'))).toBe(false);
      expect(fs.existsSync(join(setup.testDir, 'test.spec.ts'))).toBe(false);
      expect(fs.existsSync(join(setup.testDir, 'README-draft.md'))).toBe(false);

      // @step And the 2 modified files should be restored to checkpoint state: "main.ts", "config.json"
      const mainContent = await readFile(
        join(setup.testDir, 'main.ts'),
        'utf-8'
      );
      const configContent = await readFile(
        join(setup.testDir, 'config.json'),
        'utf-8'
      );
      expect(mainContent).toBe('const main = "original";');
      expect(configContent).toBe('{"version": "1.0"}');

      // @step And no files added after checkpoint should remain in the working directory
      expect(restoreResult.success).toBe(true);
    });
  });
});
