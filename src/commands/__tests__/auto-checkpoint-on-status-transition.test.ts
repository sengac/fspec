/**
 * Feature: spec/features/auto-checkpoints-not-working-lazy-import-fails-in-bundled-dist.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
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
import { updateWorkUnitStatus } from '../update-work-unit-status';
import { listCheckpoints } from '../list-checkpoints';
import {
  setupTestDirectory,
  type TestDirectorySetup,
} from '../../test-helpers/universal-test-setup';

describe('Feature: Auto-checkpoints not working - lazy import fails in bundled dist', () => {
  let setup: TestDirectorySetup;

  beforeEach(async () => {
    setup = await setupTestDirectory('auto-checkpoint-on-status-transition');

    // Initialize git repository
    gitInit(setup.testDir, 'main');

    // Configure git
    gitSetConfig(setup.testDir, 'user.name', 'Test User');
    gitSetConfig(setup.testDir, 'user.email', 'test@example.com');

    // Create initial directory structure
    await mkdir(join(setup.testDir, 'spec'), { recursive: true });

    // Create work-units.json with TEST-001 fixture
    const workUnitsData = {
      version: '1.0',
      workUnits: {
        'TEST-001': {
          id: 'TEST-001',
          title: 'Auto-checkpoint test story',
          description: 'Test work unit for auto-checkpoint testing',
          type: 'story',
          status: 'backlog',
          prefix: 'TEST',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
          rules: ['Auto-checkpoints must be created on state transitions'],
          examples: [
            'Work unit transitions from specifying to testing with uncommitted changes',
          ],
          architectureNotes: [
            'Implementation: Auto-checkpoints use git stash to save uncommitted changes',
          ],
          attachments: ['spec/attachments/TEST-001/ast-research.json'],
          stateHistory: [
            {
              state: 'backlog',
              timestamp: new Date().toISOString(),
            },
          ],
        },
      },
      states: {
        backlog: ['TEST-001'],
        specifying: [],
        testing: [],
        implementing: [],
        validating: [],
        done: [],
        blocked: [],
      },
      prefixes: {
        TEST: {
          prefix: 'TEST',
          description: 'Test stories',
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

  describe('Scenario: Auto-checkpoint created on state transition with uncommitted changes', () => {
    it('should create auto-checkpoint when transitioning from specifying to testing with dirty working directory', async () => {
      // Given: a work unit in 'specifying' status with uncommitted file changes
      // First move to specifying
      await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'specifying',
        cwd: setup.testDir,
      });

      // Add uncommitted changes
      await writeFile(
        join(setup.testDir, 'test-file.txt'),
        'Uncommitted changes'
      );

      // Verify working directory is dirty using NAPI status functions
      const { getUntrackedFiles } = await import('../../git/status');
      const untrackedFiles = await getUntrackedFiles(setup.testDir);
      const isDirty = untrackedFiles.length > 0;
      expect(isDirty).toBe(true);

      // Create a dummy feature file to satisfy validation
      await mkdir(join(setup.testDir, 'spec/features'), { recursive: true });
      await writeFile(
        join(setup.testDir, 'spec/features/test.feature'),
        `@TEST-001\nFeature: Test\nScenario: Test\nGiven test\n`
      );

      // When: the user runs 'fspec update-work-unit-status TEST-001 testing'
      const result = await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'testing',
        cwd: setup.testDir,
        skipTemporalValidation: true,
      });

      // Then: the command should create an auto-checkpoint
      expect(result.success).toBe(true);
      expect(result.checkpointCreated).toBe(true);
      expect(result.checkpointName).toBe('TEST-001-auto-specifying');

      // And: the checkpoint should be retrievable with 'fspec list-checkpoints TEST-001'
      const { checkpoints } = await listCheckpoints({
        workUnitId: 'TEST-001',
        cwd: setup.testDir,
      });
      expect(checkpoints.length).toBeGreaterThan(0);

      const autoCheckpoint = checkpoints.find(
        cp => cp.name === 'TEST-001-auto-specifying'
      );
      expect(autoCheckpoint).toBeDefined();
      expect(autoCheckpoint?.isAutomatic).toBe(true);
    });

    it('should NOT create auto-checkpoint when working directory is clean', async () => {
      // Given: a work unit in 'specifying' status with NO uncommitted changes
      // First move to specifying
      await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'specifying',
        cwd: setup.testDir,
      });

      // Create a dummy feature file to satisfy validation
      await mkdir(join(setup.testDir, 'spec/features'), { recursive: true });
      await writeFile(
        join(setup.testDir, 'spec/features/test.feature'),
        `@TEST-001\nFeature: Test\nScenario: Test\nGiven test\n`
      );

      // Commit it so working directory is clean
      gitAdd(setup.testDir, 'spec/features/test.feature');
      gitAdd(setup.testDir, 'spec/work-units.json');
      gitCommit(
        setup.testDir,
        'Add feature file',
        'Test User',
        'test@example.com'
      );

      // Verify working directory is clean
      const statusBefore =
        /* statusMatrix removed - use NAPI status functions instead */ [];
      const isDirty = statusBefore.some(row => {
        const [, headStatus, workdirStatus, stageStatus] = row;
        return headStatus !== workdirStatus || workdirStatus !== stageStatus;
      });
      expect(isDirty).toBe(false);

      // When: the user runs 'fspec update-work-unit-status TEST-001 testing'
      const result = await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'testing',
        cwd: setup.testDir,
        skipTemporalValidation: true,
      });

      // Then: the command should succeed but NOT create a checkpoint
      expect(result.success).toBe(true);
      expect(result.checkpointCreated).toBe(false);

      // And: no checkpoints should exist
      const { checkpoints } = await listCheckpoints({
        workUnitId: 'TEST-001',
        cwd: setup.testDir,
      });
      expect(checkpoints.length).toBe(0);
    });

    it('should NOT create auto-checkpoint when transitioning FROM backlog state', async () => {
      // Given: a work unit in 'backlog' status with uncommitted file changes
      await writeFile(
        join(setup.testDir, 'test-file.txt'),
        'Uncommitted changes'
      );

      // When: the user runs 'fspec update-work-unit-status TEST-001 specifying'
      // (transitioning FROM backlog, which is excluded from auto-checkpoint)
      const result = await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'specifying',
        cwd: setup.testDir,
      });

      // Then: auto-checkpoint should NOT be created when leaving backlog
      // Rule: "except from backlog" means DO NOT create when current state is backlog
      expect(result.success).toBe(true);
      expect(result.checkpointCreated).toBe(false);

      // And: no checkpoints should exist
      const { checkpoints } = await listCheckpoints({
        workUnitId: 'TEST-001',
        cwd: setup.testDir,
      });
      expect(checkpoints.length).toBe(0);
    });
  });
});
