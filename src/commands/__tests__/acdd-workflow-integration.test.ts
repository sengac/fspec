/**
 * Feature: spec/features/acdd-workflow-integration.feature
 *
 * This test file validates the acceptance criteria for ACDD workflow integration with coverage tracking.
 * Tests that work units cannot be marked done when coverage is incomplete.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { updateWorkUnitStatus } from '../update-work-unit-status';

describe('Feature: ACDD Workflow Integration', () => {
  let testDir: string;
  let specDir: string;
  let featuresDir: string;
  let workUnitsFile: string;

  beforeEach(async () => {
    // Create temporary directory for each test
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    specDir = join(testDir, 'spec');
    featuresDir = join(specDir, 'features');
    workUnitsFile = join(specDir, 'work-units.json');

    // Create spec directory structure
    await mkdir(featuresDir, { recursive: true });

    // Create work-units.json with AUTH-001
    await writeFile(
      workUnitsFile,
      JSON.stringify(
        {
          workUnits: {
            'AUTH-001': {
              id: 'AUTH-001',
              title: 'User Authentication',
              status: 'validating',
              linkedFeatures: ['user-login'],
              createdAt: new Date().toISOString(),
              updatedAt: new Date().toISOString(),
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
        },
        null,
        2
      )
    );
  });

  afterEach(async () => {
    // Clean up temporary directory
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Allow status update to done when all scenarios are covered', () => {
    it('should update work unit to done when all scenarios have coverage', async () => {
      // Given I have a work unit "AUTH-001" linked to "user-login.feature"
      // (Already created in beforeEach)

      // And the coverage file shows all 5 scenarios have testMappings
      const coverageFile = join(featuresDir, 'user-login.feature.coverage');
      await writeFile(
        coverageFile,
        JSON.stringify({
          featureFile: 'spec/features/user-login.feature',
          scenarios: [
            {
              name: 'Scenario 1',
              testMappings: [{ file: 'test1.ts', lines: [1, 2] }],
            },
            {
              name: 'Scenario 2',
              testMappings: [{ file: 'test2.ts', lines: [3, 4] }],
            },
            {
              name: 'Scenario 3',
              testMappings: [{ file: 'test3.ts', lines: [5, 6] }],
            },
            {
              name: 'Scenario 4',
              testMappings: [{ file: 'test4.ts', lines: [7, 8] }],
            },
            {
              name: 'Scenario 5',
              testMappings: [{ file: 'test5.ts', lines: [9, 10] }],
            },
          ],
        })
      );

      // When I run `fspec update-work-unit-status AUTH-001 done`
      const result = await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'done',
        cwd: testDir,
      });

      // Then the command should succeed with exit code 0
      expect(result.success).toBe(true);

      // And the work unit status should be updated to "done"
      expect(result.message).toContain('AUTH-001');
      expect(result.message).toContain('done');
    });
  });

  describe('Scenario: Block status update to done when scenarios are uncovered', () => {
    it('should fail to update work unit when scenarios lack coverage', async () => {
      // Given I have a work unit "AUTH-001" linked to "user-login.feature"
      // (Already created in beforeEach)

      // And the coverage file shows 2 out of 5 scenarios have empty testMappings
      const coverageFile = join(featuresDir, 'user-login.feature.coverage');
      await writeFile(
        coverageFile,
        JSON.stringify({
          featureFile: 'spec/features/user-login.feature',
          scenarios: [
            {
              name: 'Scenario 1',
              testMappings: [{ file: 'test1.ts', lines: [1, 2] }],
            },
            {
              name: 'Scenario 2',
              testMappings: [{ file: 'test2.ts', lines: [3, 4] }],
            },
            {
              name: 'Scenario 3',
              testMappings: [{ file: 'test3.ts', lines: [5, 6] }],
            },
            { name: 'Uncovered Scenario 1', testMappings: [] },
            { name: 'Uncovered Scenario 2', testMappings: [] },
          ],
        })
      );

      // When I run `fspec update-work-unit-status AUTH-001 done`
      const result = await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'done',
        cwd: testDir,
      });

      // Then the command should fail with exit code 1
      expect(result.success).toBe(false);

      // And the output should display "Cannot mark work unit done: 2 scenarios uncovered"
      expect(result.message).toContain('Cannot mark work unit done');
      expect(result.message).toContain('2 scenarios uncovered');

      // And the output should show a system-reminder with uncovered scenario names
      expect(result.systemReminder).toBeDefined();
      expect(result.systemReminder).toContain('Uncovered Scenario 1');
      expect(result.systemReminder).toContain('Uncovered Scenario 2');
    });
  });

  describe("Scenario: Allow status update when coverage file doesn't exist", () => {
    it('should update work unit with warning when no coverage file exists', async () => {
      // Given I have a work unit "AUTH-001" linked to "user-login.feature"
      // (Already created in beforeEach)

      // And no coverage file exists for "user-login.feature"
      // (Don't create coverage file)

      // When I run `fspec update-work-unit-status AUTH-001 done`
      const result = await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'done',
        cwd: testDir,
      });

      // Then the command should succeed with exit code 0
      expect(result.success).toBe(true);

      // And the output should display a warning about missing coverage file
      expect(result.warnings).toBeDefined();
      expect(
        result.warnings!.some((w: string) =>
          w.includes('Coverage file not found')
        )
      ).toBe(true);

      // And the work unit status should be updated to "done"
      expect(result.message).toContain('done');
    });
  });
});
