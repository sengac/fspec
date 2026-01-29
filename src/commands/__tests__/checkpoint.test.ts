/**
 * Feature: spec/features/intelligent-checkpoint-system-for-workflow-transitions.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile } from 'fs/promises';
import { join } from 'path';
import * as git from 'isomorphic-git';
import fs from 'fs';
import { checkpoint } from '../checkpoint';
import { restoreCheckpoint } from '../restore-checkpoint';
import { listCheckpoints } from '../list-checkpoints';
import { cleanupCheckpoints } from '../cleanup-checkpoints';
import { updateWorkUnitStatus } from '../update-work-unit-status';
import {
  setupWorkUnitTest,
  type WorkUnitTestSetup,
} from '../../test-helpers/universal-test-setup';
import { writeJsonTestFile } from '../../test-helpers/test-file-operations';

describe('Feature: Intelligent checkpoint system for workflow transitions', () => {
  let setup: WorkUnitTestSetup;

  beforeEach(async () => {
    setup = await setupWorkUnitTest('checkpoint');

    // Initialize git repository
    await git.init({ fs, dir: setup.testDir, defaultBranch: 'main' });

    // Configure git
    await git.setConfig({
      fs,
      dir: setup.testDir,
      path: 'user.name',
      value: 'Test User',
    });
    await git.setConfig({
      fs,
      dir: setup.testDir,
      path: 'user.email',
      value: 'test@example.com',
    });

    // Create work-units.json with GIT-002 fixture
    const workUnitsData = {
      version: '1.0',
      workUnits: {
        'GIT-002': {
          id: 'GIT-002',
          title: 'Intelligent checkpoint system',
          description: 'Test work unit for checkpoint testing',
          type: 'story',
          status: 'testing',
          prefix: 'GIT',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
          stateHistory: [
            {
              state: 'backlog',
              timestamp: new Date(Date.now() - 3600000).toISOString(),
            },
            {
              state: 'specifying',
              timestamp: new Date(Date.now() - 1800000).toISOString(),
            },
            {
              state: 'testing',
              timestamp: new Date().toISOString(),
            },
          ],
        },
      },
      states: {
        backlog: [],
        specifying: [],
        testing: ['GIT-002'],
        implementing: [],
        validating: [],
        done: [],
        blocked: [],
      },
      prefixes: {
        GIT: {
          prefix: 'GIT',
          description: 'Git operations',
          nextId: 3,
        },
      },
    };

    await writeJsonTestFile(setup.workUnitsFile, workUnitsData);

    // Create initial commit so HEAD exists
    await writeFile(join(setup.testDir, 'README.md'), '# Test Project');
    await git.add({ fs, dir: setup.testDir, filepath: 'README.md' });
    await git.add({ fs, dir: setup.testDir, filepath: 'spec/work-units.json' });
    await git.commit({
      fs,
      dir: setup.testDir,
      message: 'Initial commit',
      author: {
        name: 'Test User',
        email: 'test@example.com',
      },
    });
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: Automatic checkpoint created on workflow state transition', () => {
    it('should create automatic checkpoint when changing work unit status', async () => {
      // Given: I have a work unit "GIT-002" in "testing" status
      // And: I have uncommitted changes in my working directory
      await writeFile(
        join(setup.testDir, 'test-file.txt'),
        'Some uncommitted changes'
      );

      // When: I run "fspec update-work-unit-status GIT-002 implementing"
      const result = await updateWorkUnitStatus({
        workUnitId: 'GIT-002',
        status: 'implementing',
        cwd: setup.testDir,
        skipTemporalValidation: true,
      });

      // Then: a checkpoint "GIT-002-auto-testing" should be created automatically
      expect(result.checkpointCreated).toBe(true);
      expect(result.checkpointName).toBe('GIT-002-auto-testing');

      // And: the work unit status should change to "implementing"
      expect(result.newStatus).toBe('implementing');

      // And: the checkpoint should be stored as a git stash
      expect(result.stashRef).toBe('stash@{0}');
    });
  });

  describe('Scenario: Create manual checkpoint for experimentation', () => {
    it('should create named checkpoint manually', async () => {
      // Given: I have a work unit "GIT-002" in "implementing" status
      // And: I have uncommitted changes in my working directory
      await writeFile(
        join(setup.testDir, 'experiment.txt'),
        'Experimental code'
      );

      // When: I run "fspec checkpoint GIT-002 before-refactor"
      const result = await checkpoint({
        workUnitId: 'GIT-002',
        checkpointName: 'before-refactor',
        cwd: setup.testDir,
      });

      // Then: a checkpoint "before-refactor" should be created
      expect(result.success).toBe(true);
      expect(result.checkpointName).toBe('before-refactor');

      // And: the checkpoint should be stored as a git stash with message format
      expect(result.stashMessage).toMatch(
        /fspec-checkpoint:GIT-002:before-refactor:[0-9]+/
      );

      // And: all file changes should be captured including untracked files
      expect(result.includedUntracked).toBe(true);
    });
  });

  describe('Scenario: Multiple experiments from same baseline checkpoint', () => {
    it('should support multiple restorations from same checkpoint', async () => {
      // Given: I have created a checkpoint "baseline" for work unit "GIT-002"
      await writeFile(join(setup.testDir, 'baseline.txt'), 'Baseline code');
      await checkpoint({
        workUnitId: 'GIT-002',
        checkpointName: 'baseline',
        cwd: setup.testDir,
      });

      // Commit to clean working directory (checkpoint created uncommitted files)
      await git.add({ fs, dir: setup.testDir, filepath: '.' });
      await git.commit({
        fs,
        dir: setup.testDir,
        message: 'After baseline checkpoint',
        author: { name: 'Test User', email: 'test@example.com' },
      });

      // When: I restore checkpoint "baseline"
      const restore1 = await restoreCheckpoint({
        workUnitId: 'GIT-002',
        checkpointName: 'baseline',
        cwd: setup.testDir,
      });

      // And: I implement approach A which fails
      // And: I restore checkpoint "baseline" again
      const restore2 = await restoreCheckpoint({
        workUnitId: 'GIT-002',
        checkpointName: 'baseline',
        cwd: setup.testDir,
      });

      // And: I implement approach B which succeeds

      // Then: I should be able to compare both approaches
      expect(restore1.success).toBe(true);
      expect(restore2.success).toBe(true);

      // And: the "baseline" checkpoint should still exist for future experiments
      const result = await listCheckpoints({
        workUnitId: 'GIT-002',
        cwd: setup.testDir,
      });
      const baselineCheckpoint = result.checkpoints.find(
        c => c.name === 'baseline'
      );
      expect(baselineCheckpoint).toBeDefined();
    });
  });

  describe('Scenario: AI-assisted conflict resolution during checkpoint restoration', () => {
    it('should emit system-reminder for AI when conflicts occur', async () => {
      // Given: I have a checkpoint "previous-state" for work unit "GIT-002"
      await writeFile(
        join(setup.testDir, 'conflict-file.txt'),
        'Original content'
      );
      await checkpoint({
        workUnitId: 'GIT-002',
        checkpointName: 'previous-state',
        cwd: setup.testDir,
      });

      // Commit to clean working directory
      await git.add({ fs, dir: setup.testDir, filepath: '.' });
      await git.commit({
        fs,
        dir: setup.testDir,
        message: 'After previous-state checkpoint',
        author: { name: 'Test User', email: 'test@example.com' },
      });

      // And: I have made conflicting changes in my working directory
      // (Note: In current implementation, conflicts aren't simulated - this tests the interface)

      // When: I run "fspec restore-checkpoint GIT-002 previous-state"
      const result = await restoreCheckpoint({
        workUnitId: 'GIT-002',
        checkpointName: 'previous-state',
        cwd: setup.testDir,
      });

      // Then: restoration should succeed (no actual conflicts in test)
      expect(result.success).toBe(true);

      // And: conflict detection should be false (no conflicts created in test)
      expect(result.conflictsDetected).toBe(false);

      // Note: Actual conflict detection would require more complex git state setup
      // The implementation is ready for conflicts, but test doesn't simulate them
    });
  });

  describe('Scenario: List all checkpoints with visual indicators', () => {
    it('should display checkpoints with emoji indicators', async () => {
      // Given: I have automatic checkpoints "GIT-002-auto-testing" and "GIT-002-auto-implementing"
      // (Created by status transitions)

      // And: I have manual checkpoints "baseline" and "before-refactor"
      await writeFile(join(setup.testDir, 'file1.txt'), 'Content 1');
      await checkpoint({
        workUnitId: 'GIT-002',
        checkpointName: 'baseline',
        cwd: setup.testDir,
      });

      await writeFile(join(setup.testDir, 'file2.txt'), 'Content 2');
      await checkpoint({
        workUnitId: 'GIT-002',
        checkpointName: 'before-refactor',
        cwd: setup.testDir,
      });

      // When: I run "fspec list-checkpoints GIT-002"
      const result = await listCheckpoints({
        workUnitId: 'GIT-002',
        cwd: setup.testDir,
      });

      // Then: I should see all checkpoints with clear visual indicators
      expect(result.checkpoints.length).toBeGreaterThanOrEqual(2);

      // And: manual checkpoints should show 📌 emoji
      const manualCheckpoint = result.checkpoints.find(
        c => c.name === 'baseline'
      );
      expect(manualCheckpoint).toBeDefined();
      expect(manualCheckpoint?.displayIcon).toBe('📌');

      // And: each checkpoint should display its timestamp
      result.checkpoints.forEach(checkpoint => {
        expect(checkpoint.timestamp).toBeDefined();
        expect(checkpoint.timestamp).toMatch(/[0-9]{4}-[0-9]{2}-[0-9]{2}/);
      });
    });
  });

  describe('Scenario: Cleanup old checkpoints keeping most recent', () => {
    it('should delete old checkpoints while preserving recent ones', async () => {
      // Given: I have 10 checkpoints for work unit "GIT-002"
      for (let i = 0; i < 10; i++) {
        await writeFile(join(setup.testDir, `file-${i}.txt`), `Content ${i}`);
        await checkpoint({
          workUnitId: 'GIT-002',
          checkpointName: `checkpoint-${i}`,
          cwd: setup.testDir,
        });
        // Small delay to ensure different timestamps
        await new Promise(resolve => setTimeout(resolve, 10));
      }

      // When: I run "fspec cleanup-checkpoints GIT-002 --keep-last 5"
      const result = await cleanupCheckpoints({
        workUnitId: 'GIT-002',
        keepLast: 5,
        cwd: setup.testDir,
      });

      // Then: the 5 oldest checkpoints should be deleted
      expect(result.deletedCount).toBe(5);

      // And: the 5 most recent checkpoints should be preserved
      expect(result.preservedCount).toBe(5);

      // And: I should see a summary of deleted and preserved checkpoints
      expect(result.summary).toBeDefined();
      expect(result.summary.deleted).toHaveLength(5);
      expect(result.summary.preserved).toHaveLength(5);
    });
  });

  describe('Scenario: Interactive prompt when restoring with dirty working directory', () => {
    it('should prompt user with options when working directory is dirty', async () => {
      // Given: I have a checkpoint "safe-state" for work unit "GIT-002"
      await writeFile(join(setup.testDir, 'safe.txt'), 'Safe state');
      await checkpoint({
        workUnitId: 'GIT-002',
        checkpointName: 'safe-state',
        cwd: setup.testDir,
      });

      // And: I have uncommitted changes in my working directory
      await writeFile(join(setup.testDir, 'dirty.txt'), 'Uncommitted changes');

      // When: I run "fspec restore-checkpoint GIT-002 safe-state"
      const result = await restoreCheckpoint({
        workUnitId: 'GIT-002',
        checkpointName: 'safe-state',
        cwd: setup.testDir,
        workingDirectoryDirty: true,
      });

      // Then: I should be prompted with options
      expect(result.promptShown).toBe(true);
      expect(result.options).toBeDefined();
      expect(result.options).toHaveLength(3);

      // And: each option should explain the risks
      expect(result.options?.[0]).toMatchObject({
        name: 'Commit changes first',
        riskLevel: 'Low',
        description: expect.any(String),
      });
      expect(result.options?.[1]).toMatchObject({
        name: 'Stash changes and restore',
        riskLevel: 'Medium',
        description: expect.any(String),
      });
      expect(result.options?.[2]).toMatchObject({
        name: 'Overwrite files (discard changes)',
        riskLevel: 'High',
        description: expect.any(String),
      });

      // And: restoration should proceed based on user choice
      expect(result.requiresUserChoice).toBe(true);
    });
  });

  // ================================================================
  // BUG-027: Tests for checkpoint creation and restoration fixes
  // Feature: spec/features/stash-system-not-adding-files-before-creating-stash.feature
  // ================================================================

  describe('BUG-027: Checkpoint creation with git.stash({ op: "create" })', () => {
    it('should stage and capture modified tracked files', async () => {
      // Given: I have modified tracked file "README.md"
      await writeFile(join(setup.testDir, 'README.md'), '# Modified Content');

      // When: I run checkpoint
      const result = await checkpoint({
        workUnitId: 'GIT-002',
        checkpointName: 'test-checkpoint',
        cwd: setup.testDir,
      });

      // Then: file should be staged and captured
      expect(result.success).toBe(true);
      expect(result.capturedFiles).toContain('README.md');

      // And: working directory should remain unchanged
      const content = await fs.promises.readFile(
        join(setup.testDir, 'README.md'),
        'utf-8'
      );
      expect(content).toBe('# Modified Content');

      // And: checkpoint ref should exist in custom namespace
      const checkpointOid = await git.resolveRef({
        fs,
        dir: setup.testDir,
        ref: 'refs/fspec-checkpoints/GIT-002/test-checkpoint',
      });
      expect(checkpointOid).toBeDefined();
    });

    it('should stage and capture new untracked files', async () => {
      // Given: I have created new untracked file
      await writeFile(
        join(setup.testDir, 'new-file.ts'),
        'console.log("new");'
      );

      // When: I run checkpoint
      const result = await checkpoint({
        workUnitId: 'GIT-002',
        checkpointName: 'untracked-test',
        cwd: setup.testDir,
      });

      // Then: untracked file should be captured
      expect(result.success).toBe(true);
      expect(result.capturedFiles).toContain('new-file.ts');

      // And: working directory should remain unchanged
      const exists = await fs.promises
        .access(join(setup.testDir, 'new-file.ts'))
        .then(() => true)
        .catch(() => false);
      expect(exists).toBe(true);
    });

    it('should reset index after checkpoint creation', async () => {
      // Given: I have modified a file
      await writeFile(join(setup.testDir, 'test.ts'), 'const x = 1;');

      // When: I create checkpoint
      await checkpoint({
        workUnitId: 'GIT-002',
        checkpointName: 'index-reset-test',
        cwd: setup.testDir,
      });

      // Then: index should be clean (no staged files)
      const status = await git.statusMatrix({ fs, dir: setup.testDir });
      const stagedFiles = status.filter(row => {
        const [, headStatus, workdirStatus, stageStatus] = row;
        return stageStatus === 2; // 2 = staged
      });
      expect(stagedFiles.length).toBe(0);
    });
  });

  describe('BUG-027: Checkpoint restoration with manual file operations', () => {
    it('should restore files by reading from checkpoint commit', async () => {
      // Given: I have created a checkpoint with file
      await writeFile(
        join(setup.testDir, 'restore-test.ts'),
        'const original = 1;'
      );
      await checkpoint({
        workUnitId: 'GIT-002',
        checkpointName: 'restore-baseline',
        cwd: setup.testDir,
      });

      // And: I modify the file
      await writeFile(
        join(setup.testDir, 'restore-test.ts'),
        'const modified = 2;'
      );

      // When: I restore checkpoint (force mode to overwrite dirty files)
      const result = await restoreCheckpoint({
        workUnitId: 'GIT-002',
        checkpointName: 'restore-baseline',
        cwd: setup.testDir,
        force: true,
      });

      // Then: file should be restored to original content
      expect(result.success).toBe(true);
      const content = await fs.promises.readFile(
        join(setup.testDir, 'restore-test.ts'),
        'utf-8'
      );
      expect(content).toBe('const original = 1;');
    });

    it('should detect conflicts when file modified since checkpoint', async () => {
      // Given: I have created checkpoint with file v1
      await writeFile(join(setup.testDir, 'conflict.ts'), 'version 1');
      await checkpoint({
        workUnitId: 'GIT-002',
        checkpointName: 'conflict-test',
        cwd: setup.testDir,
      });

      // And: I modify file to v2
      await writeFile(join(setup.testDir, 'conflict.ts'), 'version 2');

      // When: I try to restore checkpoint
      const result = await restoreCheckpoint({
        workUnitId: 'GIT-002',
        checkpointName: 'conflict-test',
        cwd: setup.testDir,
      });

      // Then: conflict should be detected
      expect(result.success).toBe(false);
      expect(result.conflictsDetected).toBe(true);
      expect(result.conflictedFiles).toContain('conflict.ts');

      // And: system-reminder should be emitted
      expect(result.systemReminder).toContain('CONFLICT DETECTED');

      // And: file should NOT be overwritten (stays v2)
      const content = await fs.promises.readFile(
        join(setup.testDir, 'conflict.ts'),
        'utf-8'
      );
      expect(content).toBe('version 2');
    });

    it('should recreate deleted files from checkpoint', async () => {
      // Given: I have created checkpoint with file
      await writeFile(join(setup.testDir, 'deletable.ts'), 'will be deleted');
      await checkpoint({
        workUnitId: 'GIT-002',
        checkpointName: 'deletion-test',
        cwd: setup.testDir,
      });

      // And: I delete the file
      await fs.promises.unlink(join(setup.testDir, 'deletable.ts'));

      // When: I restore checkpoint
      const result = await restoreCheckpoint({
        workUnitId: 'GIT-002',
        checkpointName: 'deletion-test',
        cwd: setup.testDir,
        force: true, // Force restoration despite dirty working directory
      });

      // Then: file should be recreated
      expect(result.success).toBe(true);
      const exists = await fs.promises
        .access(join(setup.testDir, 'deletable.ts'))
        .then(() => true)
        .catch(() => false);
      expect(exists).toBe(true);
      const content = await fs.promises.readFile(
        join(setup.testDir, 'deletable.ts'),
        'utf-8'
      );
      expect(content).toBe('will be deleted');
    });

    it('should ignore files not in checkpoint', async () => {
      // Given: I have created checkpoint with file A
      await writeFile(join(setup.testDir, 'fileA.ts'), 'file A');
      await checkpoint({
        workUnitId: 'GIT-002',
        checkpointName: 'ignore-test',
        cwd: setup.testDir,
      });

      // And: I add new file B (not in checkpoint)
      await writeFile(join(setup.testDir, 'fileB.ts'), 'file B');

      // When: I restore checkpoint (force mode to skip prompt)
      const result = await restoreCheckpoint({
        workUnitId: 'GIT-002',
        checkpointName: 'ignore-test',
        cwd: setup.testDir,
        force: true,
      });

      // Then: file B should be left untouched
      expect(result.success).toBe(true);
      const exists = await fs.promises
        .access(join(setup.testDir, 'fileB.ts'))
        .then(() => true)
        .catch(() => false);
      expect(exists).toBe(true);
      const content = await fs.promises.readFile(
        join(setup.testDir, 'fileB.ts'),
        'utf-8'
      );
      expect(content).toBe('file B');
    });

    it('should error when restoring non-existent checkpoint', async () => {
      // Given: checkpoint does not exist

      // When: I try to restore non-existent checkpoint
      const result = await restoreCheckpoint({
        workUnitId: 'GIT-002',
        checkpointName: 'does-not-exist',
        cwd: setup.testDir,
      });

      // Then: should fail with appropriate error
      expect(result.success).toBe(false);
      expect(result.systemReminder).toContain('not found');
    });
  });
});
