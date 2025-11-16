/**
 * Feature: spec/features/coverage-validation-missing-for-implementing-validating-transition.feature
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, writeFile, mkdir, readFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { updateWorkUnitStatus } from '../update-work-unit-status';
import type { WorkUnitsData } from '../../types';

describe('Feature: Coverage validation missing for implementing→validating transition', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-coverage-validation-'));
    await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Block implementing→validating when implementation coverage is missing', () => {
    it('should block transition with error when implementation files are not linked', async () => {
      // @step Given I have a work unit "TEST-001" in implementing status
      const workUnitsData: WorkUnitsData = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: ['TEST-001'],
          validating: [],
          done: [],
          blocked: [],
        },
        workUnits: {
          'TEST-001': {
            id: 'TEST-001',
            prefix: 'TEST',
            type: 'story',
            title: 'Test Work Unit',
            description: 'Testing coverage validation',
            status: 'implementing',
            createdAt: '2025-01-15T10:00:00.000Z',
            updatedAt: '2025-01-15T10:00:00.000Z',
            stateHistory: [
              {
                state: 'backlog',
                enteredAt: '2025-01-15T10:00:00.000Z',
              },
              {
                state: 'specifying',
                enteredAt: '2025-01-15T10:05:00.000Z',
              },
              {
                state: 'testing',
                enteredAt: '2025-01-15T10:10:00.000Z',
              },
              {
                state: 'implementing',
                enteredAt: '2025-01-15T10:15:00.000Z',
              },
            ],
          },
        },
      };

      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // Create feature file with @TEST-001 tag
      const featureContent = `@TEST-001
Feature: Test Feature

  Background: User Story
    As a developer
    I want to test coverage validation
    So that I ensure implementation is complete

  Scenario: Test scenario
    Given a precondition
    When an action
    Then an outcome
`;

      await writeFile(
        join(testDir, 'spec', 'features', 'test-feature.feature'),
        featureContent
      );

      // @step And the work unit has test files linked in coverage
      // @step And the work unit has NO implementation files linked in coverage
      const coverageData = {
        scenarios: [
          {
            name: 'Test scenario',
            testMappings: [
              {
                file: 'src/__tests__/test.test.ts',
                lines: '10-20',
                implMappings: [], // NO implementation files linked!
              },
            ],
          },
        ],
        stats: {
          totalScenarios: 1,
          coveredScenarios: 1,
          coveragePercent: 100,
          testFiles: ['src/__tests__/test.test.ts'],
          implFiles: [], // Empty implementation files!
          totalLinesCovered: 10,
        },
      };

      await writeFile(
        join(testDir, 'spec', 'features', 'test-feature.feature.coverage'),
        JSON.stringify(coverageData, null, 2)
      );

      // Create test file (must exist for validation)
      await mkdir(join(testDir, 'src', '__tests__'), { recursive: true });
      await writeFile(
        join(testDir, 'src', '__tests__', 'test.test.ts'),
        `
describe('Test scenario', () => {
  it('should test', () => {
    // @step Given a precondition
    // @step When an action
    // @step Then an outcome
    expect(true).toBe(true);
  });
});
        `
      );

      // @step When I run "fspec update-work-unit-status TEST-001 validating"
      const result = await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'validating',
        cwd: testDir,
      });

      // @step Then the command should fail with error code 1
      expect(result.success).toBe(false);

      // @step And the error message should contain "implementation coverage is incomplete"
      expect(result.message).toContain('implementation coverage is incomplete');

      // @step And the error message should suggest "fspec link-coverage" command
      expect(result.systemReminder).toContain('fspec link-coverage');

      // @step And the work unit status should remain "implementing"
      const workUnitsAfter = JSON.parse(
        await readFile(join(testDir, 'spec', 'work-units.json'), 'utf-8')
      ) as WorkUnitsData;

      expect(workUnitsAfter.workUnits['TEST-001'].status).toBe('implementing');
      expect(workUnitsAfter.states.implementing).toContain('TEST-001');
      expect(workUnitsAfter.states.validating).not.toContain('TEST-001');
    });
  });
});
