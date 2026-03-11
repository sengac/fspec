/**
 * Feature: spec/features/checkpoint-restore-shows-confusing-merge-terminology-when-it-actually-overwrites-files.feature
 *
 * This test file validates the acceptance criteria for BUG-053.
 * Tests verify that restore-checkpoint shows accurate overwrite terminology
 * instead of misleading "merge" language.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile } from 'fs/promises';
import { join } from 'path';
import {
  gitInit,
  gitSetConfig,
  gitAdd,
  gitCommit,
  resolveRef,
} from '@sengac/codelet-napi';
import fs from 'fs';
import { restoreCheckpoint } from '../restore-checkpoint';
import { checkpoint } from '../checkpoint';
import {
  setupGitTest,
  type GitTestSetup,
} from '../../test-helpers/universal-test-setup';

describe('Feature: Checkpoint restore shows confusing merge terminology when it actually overwrites files', () => {
  let setup: GitTestSetup;

  beforeEach(async () => {
    setup = await setupGitTest('restore-checkpoint-terminology');
    await setup.initGit();

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
      join(setup.testDir, 'spec/work-units.json'),
      JSON.stringify(workUnitsData, null, 2)
    );

    // Create initial commit so HEAD exists
    await writeFile(join(setup.testDir, 'README.md'), '# Test Project');
    gitAdd(setup.testDir, 'README.md');
    gitCommit(setup.testDir, 'Initial commit', 'Test User', 'test@example.com');
  });

  afterEach(async () => {
    await setup.cleanup();
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
      await writeFile(join(setup.testDir, 'test.txt'), 'initial content');
      gitAdd(setup.testDir, 'test.txt');
      gitCommit(
        setup.testDir,
        'Add test file',
        'Test User',
        'test@example.com'
      );

      // Create checkpoint
      await checkpoint({
        workUnitId: 'AUTH-001',
        checkpointName: 'baseline',
        cwd: setup.testDir,
      });

      // And: I have uncommitted changes in my working directory
      await writeFile(join(setup.testDir, 'test.txt'), 'modified content');

      // When: I run 'fspec restore-checkpoint AUTH-001 baseline'
      const result = await restoreCheckpoint({
        workUnitId: 'AUTH-001',
        checkpointName: 'baseline',
        cwd: setup.testDir,
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
      await writeFile(join(setup.testDir, 'test.txt'), 'initial content');
      gitAdd(setup.testDir, 'test.txt');
      gitCommit(
        setup.testDir,
        'Add test file',
        'Test User',
        'test@example.com'
      );

      // Create checkpoint
      await checkpoint({
        workUnitId: 'AUTH-001',
        checkpointName: 'baseline',
        cwd: setup.testDir,
      });

      // Modify file (uncommitted changes)
      await writeFile(join(setup.testDir, 'test.txt'), 'modified content');

      // When: I view the restore-checkpoint prompt options
      const result = await restoreCheckpoint({
        workUnitId: 'AUTH-001',
        checkpointName: 'baseline',
        cwd: setup.testDir,
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
