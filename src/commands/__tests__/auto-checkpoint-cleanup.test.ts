/**
 * Feature: spec/features/auto-checkpoints-not-cleaned-up-when-work-unit-moves-to-done.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile, readFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import * as git from 'isomorphic-git';
import fs from 'fs';
import { updateWorkUnitStatus } from '../update-work-unit-status';
import { createCheckpoint } from '../../utils/git-checkpoint';
import { existsSync } from 'fs';

describe('Feature: Auto-checkpoints not cleaned up when work unit moves to done', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-auto-checkpoint-cleanup-'));

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

    // Create initial commit so HEAD exists
    await writeFile(join(testDir, 'README.md'), '# Test Project');
    await git.add({ fs, dir: testDir, filepath: 'README.md' });
    await git.commit({
      fs,
      dir: testDir,
      message: 'Initial commit',
      author: {
        name: 'Test User',
        email: 'test@example.com',
      },
    });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  // @step Given a work unit "AUTH-001" exists with status "implementing"
  // @step And "AUTH-001" has automatic checkpoint "AUTH-001-auto-specifying"
  // @step And "AUTH-001" has automatic checkpoint "AUTH-001-auto-testing"
  // @step And "AUTH-001" has manual checkpoint "before-major-refactor"
  // @step When I run "fspec update-work-unit-status AUTH-001 done"
  // @step Then the command should succeed
  // @step And checkpoint "AUTH-001-auto-specifying" should be deleted
  // @step And checkpoint "AUTH-001-auto-testing" should be deleted
  // @step And checkpoint "before-major-refactor" should exist
  // @step And the git ref "refs/fspec-checkpoints/AUTH-001/AUTH-001-auto-specifying" should not exist
  // @step And the git ref "refs/fspec-checkpoints/AUTH-001/AUTH-001-auto-testing" should not exist
  // @step And the git ref "refs/fspec-checkpoints/AUTH-001/before-major-refactor" should exist
  // @step And the index file should not contain "AUTH-001-auto-specifying"
  // @step And the index file should not contain "AUTH-001-auto-testing"
  // @step And the index file should contain "before-major-refactor"
  describe('Scenario: Work unit with mixed auto and manual checkpoints', () => {
    it('should delete only auto-checkpoints when work unit moves to done', async () => {
      // Given a work unit "AUTH-001" exists with status "validating"
      const workUnitsData = {
        version: '1.0',
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'Test work unit',
            description: 'Test',
            type: 'story',
            status: 'validating',
            prefix: 'AUTH',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              {
                state: 'backlog',
                timestamp: new Date().toISOString(),
              },
              {
                state: 'validating',
                timestamp: new Date().toISOString(),
              },
            ],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: ['AUTH-001'],
          done: [],
          blocked: [],
        },
        prefixes: {
          AUTH: {
            prefix: 'AUTH',
            description: 'Auth stories',
            nextId: 2,
          },
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // Create test file for commit
      await writeFile(join(testDir, 'test.txt'), 'test content');
      await git.add({ fs, dir: testDir, filepath: 'test.txt' });
      await git.commit({
        fs,
        dir: testDir,
        message: 'Test commit',
        author: { name: 'Test User', email: 'test@example.com' },
      });

      // And "AUTH-001" has automatic checkpoint "AUTH-001-auto-specifying"
      await writeFile(join(testDir, 'auto-spec.txt'), 'auto spec checkpoint');
      await createCheckpoint({
        workUnitId: 'AUTH-001',
        checkpointName: 'AUTH-001-auto-specifying',
        cwd: testDir,
        includeUntracked: true,
      });

      // And "AUTH-001" has automatic checkpoint "AUTH-001-auto-testing"
      await writeFile(join(testDir, 'auto-test.txt'), 'auto test checkpoint');
      await createCheckpoint({
        workUnitId: 'AUTH-001',
        checkpointName: 'AUTH-001-auto-testing',
        cwd: testDir,
        includeUntracked: true,
      });

      // And "AUTH-001" has manual checkpoint "before-major-refactor"
      await writeFile(join(testDir, 'manual.txt'), 'manual checkpoint');
      await createCheckpoint({
        workUnitId: 'AUTH-001',
        checkpointName: 'before-major-refactor',
        cwd: testDir,
        includeUntracked: true,
      });

      // Commit everything so working directory is clean
      await git.add({ fs, dir: testDir, filepath: '.' });
      await git.commit({
        fs,
        dir: testDir,
        message: 'Clean state',
        author: { name: 'Test User', email: 'test@example.com' },
      });

      // When I run "fspec update-work-unit-status AUTH-001 done"
      const result = await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'done',
        cwd: testDir,
        skipTemporalValidation: true,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And checkpoint "AUTH-001-auto-specifying" should be deleted
      const autoSpecRefPath = join(
        testDir,
        '.git/refs/fspec-checkpoints/AUTH-001/AUTH-001-auto-specifying'
      );
      expect(existsSync(autoSpecRefPath)).toBe(false);

      // And checkpoint "AUTH-001-auto-testing" should be deleted
      const autoTestRefPath = join(
        testDir,
        '.git/refs/fspec-checkpoints/AUTH-001/AUTH-001-auto-testing'
      );
      expect(existsSync(autoTestRefPath)).toBe(false);

      // And checkpoint "before-major-refactor" should exist
      const manualRefPath = join(
        testDir,
        '.git/refs/fspec-checkpoints/AUTH-001/before-major-refactor'
      );
      expect(existsSync(manualRefPath)).toBe(true);

      // And the index file should not contain "AUTH-001-auto-specifying"
      // And the index file should not contain "AUTH-001-auto-testing"
      // And the index file should contain "before-major-refactor"
      const indexPath = join(
        testDir,
        '.git/fspec-checkpoints-index/AUTH-001.json'
      );
      const indexContent = await readFile(indexPath, 'utf-8');
      const index = JSON.parse(indexContent);

      expect(
        index.checkpoints.some(
          (cp: any) => cp.name === 'AUTH-001-auto-specifying'
        )
      ).toBe(false);
      expect(
        index.checkpoints.some((cp: any) => cp.name === 'AUTH-001-auto-testing')
      ).toBe(false);
      expect(
        index.checkpoints.some((cp: any) => cp.name === 'before-major-refactor')
      ).toBe(true);
    });
  });

  // @step Given a work unit "BUG-027" exists with status "validating"
  // @step And "BUG-027" has manual checkpoint "before-fix"
  // @step And "BUG-027" has manual checkpoint "after-tests"
  // @step When I run "fspec update-work-unit-status BUG-027 done"
  // @step Then the command should succeed
  // @step And checkpoint "before-fix" should exist
  // @step And checkpoint "after-tests" should exist
  // @step And the git ref "refs/fspec-checkpoints/BUG-027/before-fix" should exist
  // @step And the git ref "refs/fspec-checkpoints/BUG-027/after-tests" should exist
  // @step And no checkpoints should be deleted
  describe('Scenario: Work unit with only manual checkpoints', () => {
    it('should preserve all manual checkpoints when work unit moves to done', async () => {
      // Given a work unit "BUG-027" exists with status "validating"
      const workUnitsData = {
        version: '1.0',
        workUnits: {
          'BUG-027': {
            id: 'BUG-027',
            title: 'Test bug',
            description: 'Test',
            type: 'bug',
            status: 'validating',
            prefix: 'BUG',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              {
                state: 'backlog',
                timestamp: new Date().toISOString(),
              },
              {
                state: 'validating',
                timestamp: new Date().toISOString(),
              },
            ],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: ['BUG-027'],
          done: [],
          blocked: [],
        },
        prefixes: {
          BUG: {
            prefix: 'BUG',
            description: 'Bug fixes',
            nextId: 28,
          },
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // Create test file for commit
      await writeFile(join(testDir, 'test.txt'), 'test content');
      await git.add({ fs, dir: testDir, filepath: 'test.txt' });
      await git.commit({
        fs,
        dir: testDir,
        message: 'Test commit',
        author: { name: 'Test User', email: 'test@example.com' },
      });

      // And "BUG-027" has manual checkpoint "before-fix"
      await writeFile(join(testDir, 'before-fix.txt'), 'before fix');
      await createCheckpoint({
        workUnitId: 'BUG-027',
        checkpointName: 'before-fix',
        cwd: testDir,
        includeUntracked: true,
      });

      // And "BUG-027" has manual checkpoint "after-tests"
      await writeFile(join(testDir, 'after-tests.txt'), 'after tests');
      await createCheckpoint({
        workUnitId: 'BUG-027',
        checkpointName: 'after-tests',
        cwd: testDir,
        includeUntracked: true,
      });

      // Commit everything
      await git.add({ fs, dir: testDir, filepath: '.' });
      await git.commit({
        fs,
        dir: testDir,
        message: 'Clean state',
        author: { name: 'Test User', email: 'test@example.com' },
      });

      // When I run "fspec update-work-unit-status BUG-027 done"
      const result = await updateWorkUnitStatus({
        workUnitId: 'BUG-027',
        status: 'done',
        cwd: testDir,
        skipTemporalValidation: true,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And checkpoint "before-fix" should exist
      const beforeFixRefPath = join(
        testDir,
        '.git/refs/fspec-checkpoints/BUG-027/before-fix'
      );
      expect(existsSync(beforeFixRefPath)).toBe(true);

      // And checkpoint "after-tests" should exist
      const afterTestsRefPath = join(
        testDir,
        '.git/refs/fspec-checkpoints/BUG-027/after-tests'
      );
      expect(existsSync(afterTestsRefPath)).toBe(true);

      // And no checkpoints should be deleted (verify index still has both)
      const indexPath = join(
        testDir,
        '.git/fspec-checkpoints-index/BUG-027.json'
      );
      const indexContent = await readFile(indexPath, 'utf-8');
      const index = JSON.parse(indexContent);

      expect(index.checkpoints.length).toBe(2);
      expect(
        index.checkpoints.some((cp: any) => cp.name === 'before-fix')
      ).toBe(true);
      expect(
        index.checkpoints.some((cp: any) => cp.name === 'after-tests')
      ).toBe(true);
    });
  });

  // @step Given a work unit "FEAT-010" exists with status "validating"
  // @step And "FEAT-010" has automatic checkpoint "FEAT-010-auto-backlog"
  // @step And "FEAT-010" has automatic checkpoint "FEAT-010-auto-specifying"
  // @step When I run "fspec update-work-unit-status FEAT-010 done"
  // @step Then the command should succeed
  // @step And checkpoint "FEAT-010-auto-backlog" should be deleted
  // @step And checkpoint "FEAT-010-auto-specifying" should be deleted
  // @step And the git ref "refs/fspec-checkpoints/FEAT-010/FEAT-010-auto-backlog" should not exist
  // @step And the git ref "refs/fspec-checkpoints/FEAT-010/FEAT-010-auto-specifying" should not exist
  // @step And all checkpoints for "FEAT-010" should be deleted
  describe('Scenario: Work unit with only automatic checkpoints', () => {
    it('should delete all auto-checkpoints when work unit moves to done', async () => {
      // Given a work unit "FEAT-010" exists with status "validating"
      const workUnitsData = {
        version: '1.0',
        workUnits: {
          'FEAT-010': {
            id: 'FEAT-010',
            title: 'Test feature',
            description: 'Test',
            type: 'story',
            status: 'validating',
            prefix: 'FEAT',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              {
                state: 'backlog',
                timestamp: new Date().toISOString(),
              },
              {
                state: 'validating',
                timestamp: new Date().toISOString(),
              },
            ],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: ['FEAT-010'],
          done: [],
          blocked: [],
        },
        prefixes: {
          FEAT: {
            prefix: 'FEAT',
            description: 'Features',
            nextId: 11,
          },
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // Create test file for commit
      await writeFile(join(testDir, 'test.txt'), 'test content');
      await git.add({ fs, dir: testDir, filepath: 'test.txt' });
      await git.commit({
        fs,
        dir: testDir,
        message: 'Test commit',
        author: { name: 'Test User', email: 'test@example.com' },
      });

      // And "FEAT-010" has automatic checkpoint "FEAT-010-auto-backlog"
      await writeFile(join(testDir, 'auto-backlog.txt'), 'auto backlog');
      await createCheckpoint({
        workUnitId: 'FEAT-010',
        checkpointName: 'FEAT-010-auto-backlog',
        cwd: testDir,
        includeUntracked: true,
      });

      // And "FEAT-010" has automatic checkpoint "FEAT-010-auto-specifying"
      await writeFile(join(testDir, 'auto-spec.txt'), 'auto specifying');
      await createCheckpoint({
        workUnitId: 'FEAT-010',
        checkpointName: 'FEAT-010-auto-specifying',
        cwd: testDir,
        includeUntracked: true,
      });

      // Commit everything
      await git.add({ fs, dir: testDir, filepath: '.' });
      await git.commit({
        fs,
        dir: testDir,
        message: 'Clean state',
        author: { name: 'Test User', email: 'test@example.com' },
      });

      // When I run "fspec update-work-unit-status FEAT-010 done"
      const result = await updateWorkUnitStatus({
        workUnitId: 'FEAT-010',
        status: 'done',
        cwd: testDir,
        skipTemporalValidation: true,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And checkpoint "FEAT-010-auto-backlog" should be deleted
      const autoBacklogRefPath = join(
        testDir,
        '.git/refs/fspec-checkpoints/FEAT-010/FEAT-010-auto-backlog'
      );
      expect(existsSync(autoBacklogRefPath)).toBe(false);

      // And checkpoint "FEAT-010-auto-specifying" should be deleted
      const autoSpecRefPath = join(
        testDir,
        '.git/refs/fspec-checkpoints/FEAT-010/FEAT-010-auto-specifying'
      );
      expect(existsSync(autoSpecRefPath)).toBe(false);

      // And all checkpoints for "FEAT-010" should be deleted
      const indexPath = join(
        testDir,
        '.git/fspec-checkpoints-index/FEAT-010.json'
      );
      const indexContent = await readFile(indexPath, 'utf-8');
      const index = JSON.parse(indexContent);

      expect(index.checkpoints.length).toBe(0);
    });
  });

  // @step Given a work unit "TASK-001" exists with status "implementing"
  // @step And "TASK-001" has no checkpoints
  // @step When I run "fspec update-work-unit-status TASK-001 done"
  // @step Then the command should succeed
  // @step And no errors should occur
  // @step And no checkpoints should be deleted
  describe('Scenario: Work unit with no checkpoints', () => {
    it('should complete successfully when work unit has no checkpoints', async () => {
      // Given a work unit "TASK-001" exists with status "validating"
      const workUnitsData = {
        version: '1.0',
        workUnits: {
          'TASK-001': {
            id: 'TASK-001',
            title: 'Test task',
            description: 'Test',
            type: 'task',
            status: 'validating',
            prefix: 'TASK',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              {
                state: 'backlog',
                timestamp: new Date().toISOString(),
              },
              {
                state: 'validating',
                timestamp: new Date().toISOString(),
              },
            ],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: ['TASK-001'],
          done: [],
          blocked: [],
        },
        prefixes: {
          TASK: {
            prefix: 'TASK',
            description: 'Tasks',
            nextId: 2,
          },
        },
      };

      await writeFile(
        join(testDir, 'spec/work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // Create test file for commit (clean working directory)
      await writeFile(join(testDir, 'test.txt'), 'test content');
      await git.add({ fs, dir: testDir, filepath: '.' });
      await git.commit({
        fs,
        dir: testDir,
        message: 'Clean state',
        author: { name: 'Test User', email: 'test@example.com' },
      });

      // And "TASK-001" has no checkpoints (no checkpoint creation)

      // When I run "fspec update-work-unit-status TASK-001 done"
      const result = await updateWorkUnitStatus({
        workUnitId: 'TASK-001',
        status: 'done',
        cwd: testDir,
        skipTemporalValidation: true,
      });

      // Then the command should succeed
      expect(result.success).toBe(true);

      // And no errors should occur (already verified by success)
      // And no checkpoints should be deleted (verify no index file exists)
      const indexPath = join(
        testDir,
        '.git/fspec-checkpoints-index/TASK-001.json'
      );
      expect(existsSync(indexPath)).toBe(false);
    });
  });
});
