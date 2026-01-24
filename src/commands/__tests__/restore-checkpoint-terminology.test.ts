/**
 * Feature: spec/features/checkpoint-restore-shows-confusing-merge-terminology-when-it-actually-overwrites-files.feature
 *
 * This test file validates the acceptance criteria for BUG-053.
 * Tests verify that restore-checkpoint shows accurate overwrite terminology
 * instead of misleading "merge" language.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import * as git from 'isomorphic-git';
import fs from 'fs';
import { restoreCheckpoint } from '../restore-checkpoint';
import { checkpoint } from '../checkpoint';

describe('Feature: Checkpoint restore shows confusing merge terminology when it actually overwrites files', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-restore-terminology-test-'));

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

    // Create work-units.json with AUTH-001 fixture
    const workUnitsData = {
      version: '1.0',
      workUnits: {
        'AUTH-001': {
          id: 'AUTH-001',
          title: 'User Login',
          description: 'Test work unit for restore checkpoint',
          type: 'story',
          status: 'implementing',
          prefix: 'AUTH',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
          stateHistory: [
            {
              state: 'backlog',
              timestamp: new Date(Date.now() - 3600000).toISOString(),
            },
            {
              state: 'implementing',
              timestamp: new Date().toISOString(),
            },
          ],
        },
      },
      states: {
        backlog: [],
        specifying: [],
        testing: [],
        implementing: ['AUTH-001'],
        validating: [],
        done: [],
        blocked: [],
      },
      prefixes: {
        AUTH: {
          prefix: 'AUTH',
          description: 'Authentication',
          nextId: 2,
        },
      },
    };

    await writeFile(
      join(testDir, 'spec/work-units.json'),
      JSON.stringify(workUnitsData, null, 2)
    );

    // Create initial commit so HEAD exists
    await writeFile(join(testDir, 'README.md'), '# Test Project');
    await git.add({ fs, dir: testDir, filepath: 'README.md' });
    await git.commit({
      fs,
      dir: testDir,
      message: 'Initial commit',
      author: { name: 'Test User', email: 'test@example.com' },
    });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Restore with dirty working directory shows accurate overwrite option', () => {
    // @step Given I have a work unit AUTH-001 with a checkpoint named 'baseline'
    // @step And I have uncommitted changes in my working directory
    // @step When I run 'fspec restore-checkpoint AUTH-001 baseline'
    // @step Then option 3 should be labeled 'Overwrite files (discard changes)'
    // @step And option 3 should have risk level 'High'
    // @step And option 3 description should warn 'Overwrites working directory with checkpoint. Current changes will be LOST FOREVER unless committed or stashed.'

    it('should show option 3 labeled as Overwrite files (discard changes)', async () => {
      // Given: I have a work unit AUTH-001 with a checkpoint named 'baseline'
      await writeFile(join(testDir, 'test.txt'), 'initial content');
      await git.add({ fs, dir: testDir, filepath: 'test.txt' });
      await git.commit({
        fs,
        dir: testDir,
        message: 'Add test file',
        author: { name: 'Test User', email: 'test@example.com' },
      });

      // Create checkpoint
      await checkpoint({
        workUnitId: 'AUTH-001',
        checkpointName: 'baseline',
        cwd: testDir,
      });

      // And: I have uncommitted changes in my working directory
      await writeFile(join(testDir, 'test.txt'), 'modified content');

      // When: I run 'fspec restore-checkpoint AUTH-001 baseline'
      const result = await restoreCheckpoint({
        workUnitId: 'AUTH-001',
        checkpointName: 'baseline',
        cwd: testDir,
      });

      // Then: option 3 should be labeled 'Overwrite files (discard changes)'
      expect(result.options).toBeDefined();
      expect(result.options).toHaveLength(3);

      const option3 = result.options![2];
      expect(option3.name).toBe('Overwrite files (discard changes)');

      // And: option 3 should have risk level 'High'
      expect(option3.riskLevel).toBe('High');

      // And: option 3 description should warn about data loss
      expect(option3.description).toContain(
        'Overwrites working directory with checkpoint'
      );
      expect(option3.description).toContain('LOST FOREVER');
      expect(option3.description).toContain('committed or stashed');
    });
  });

  describe('Scenario: Option text removes all misleading merge terminology', () => {
    // @step Given I have a work unit with uncommitted changes
    // @step When I view the restore-checkpoint prompt options
    // @step Then no option should contain the word 'merge'
    // @step And no option description should mention 'conflicts' or 'manual resolution'
    // @step And the terminology should accurately reflect pure file overwrite behavior

    it('should not contain any merge terminology in options', async () => {
      // Given: I have a work unit with uncommitted changes
      await writeFile(join(testDir, 'test.txt'), 'initial content');
      await git.add({ fs, dir: testDir, filepath: 'test.txt' });
      await git.commit({
        fs,
        dir: testDir,
        message: 'Add test file',
        author: { name: 'Test User', email: 'test@example.com' },
      });

      // Create checkpoint
      await checkpoint({
        workUnitId: 'AUTH-001',
        checkpointName: 'baseline',
        cwd: testDir,
      });

      // Modify file (uncommitted changes)
      await writeFile(join(testDir, 'test.txt'), 'modified content');

      // When: I view the restore-checkpoint prompt options
      const result = await restoreCheckpoint({
        workUnitId: 'AUTH-001',
        checkpointName: 'baseline',
        cwd: testDir,
      });

      // Then: no option should contain the word 'merge'
      expect(result.options).toBeDefined();
      for (const option of result.options!) {
        expect(option.name.toLowerCase()).not.toContain('merge');
        expect(option.description.toLowerCase()).not.toContain('merge');
      }

      // And: no option description should mention 'conflicts' or 'manual resolution'
      for (const option of result.options!) {
        expect(option.description.toLowerCase()).not.toContain('conflicts');
        expect(option.description.toLowerCase()).not.toContain(
          'manual resolution'
        );
      }

      // And: the terminology should accurately reflect pure file overwrite behavior
      // Option 3 should explicitly mention "overwrite" or "discard"
      const option3 = result.options![2];
      const combinedText = (
        option3.name +
        ' ' +
        option3.description
      ).toLowerCase();
      const hasAccurateTerminology =
        combinedText.includes('overwrite') ||
        combinedText.includes('discard') ||
        combinedText.includes('replace');

      expect(hasAccurateTerminology).toBe(true);
    });
  });
});
