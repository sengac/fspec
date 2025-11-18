/**
 * Feature: spec/features/multiple-test-files-mapped-to-single-feature-file-causes-validation-errors.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, writeFile, mkdir } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { updateWorkUnitStatus } from '../update-work-unit-status';

describe('Feature: Multiple test files mapped to single feature file causes validation errors', () => {
  let testDir: string;
  let specDir: string;
  let workUnitsFile: string;
  let featuresDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    specDir = join(testDir, 'spec');
    workUnitsFile = join(specDir, 'work-units.json');
    featuresDir = join(specDir, 'features');

    await mkdir(specDir, { recursive: true });
    await mkdir(featuresDir, { recursive: true });
    await mkdir(join(testDir, 'src/__tests__'), { recursive: true });

    await writeFile(
      workUnitsFile,
      JSON.stringify(
        {
          workUnits: {},
          states: {
            backlog: [],
            specifying: [],
            testing: [],
            implementing: [],
            validating: [],
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
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Allow transition when 1 feature has 1 test file', () => {
    it('should allow transition when coverage has 1 test file', async () => {
      // @step Given a work unit has 1 feature file
      const featureContent = `@TEST-001
Feature: Test Feature

  Scenario: Test scenario
    Given a precondition
    When an action
    Then a result
`;
      await writeFile(
        join(featuresDir, 'test-feature.feature'),
        featureContent
      );

      // @step And the coverage file links 1 test file to the feature
      const coverageContent = {
        scenarios: [
          {
            name: 'Test scenario',
            testMappings: [
              {
                file: 'src/__tests__/test.test.ts',
                lines: '10-15',
              },
            ],
          },
        ],
      };
      await writeFile(
        join(featuresDir, 'test-feature.feature.coverage'),
        JSON.stringify(coverageContent, null, 2)
      );

      // Create test file with @step comments
      const testContent = `// Feature: spec/features/test-feature.feature

describe('Test scenario', () => {
  it('test', () => {
    // @step Given a precondition
    // @step When an action
    // @step Then a result
  });
});
`;
      await writeFile(join(testDir, 'src/__tests__/test.test.ts'), testContent);

      // Create work unit
      await writeFile(
        workUnitsFile,
        JSON.stringify(
          {
            workUnits: {
              'TEST-001': {
                id: 'TEST-001',
                type: 'story',
                title: 'Test Feature',
                status: 'testing',
                createdAt: new Date().toISOString(),
                updatedAt: new Date().toISOString(),
                stateHistory: [
                  {
                    state: 'testing',
                    timestamp: new Date(Date.now() - 1000).toISOString(),
                  },
                ],
              },
            },
            states: {
              backlog: [],
              specifying: [],
              testing: ['TEST-001'],
              implementing: [],
              validating: [],
              done: [],
              blocked: [],
            },
          },
          null,
          2
        )
      );

      // @step When validation runs
      // @step Then validation should proceed normally
      // @step And the transition should succeed
      await expect(
        updateWorkUnitStatus({
          workUnitId: 'TEST-001',
          status: 'implementing',
          cwd: testDir,
          skipTemporalValidation: true,
        })
      ).resolves.not.toThrow();
    });
  });

  describe('Scenario: Block transition when 1 feature has multiple test files', () => {
    it('should throw error when coverage has 2 or more test files', async () => {
      // @step Given a work unit has 1 feature file
      const featureContent = `@TEST-002
Feature: Test Feature

  Scenario: Scenario 1
    Given step 1
    When action 1
    Then result 1

  Scenario: Scenario 2
    Given step 2
    When action 2
    Then result 2
`;
      await writeFile(
        join(featuresDir, 'test-feature.feature'),
        featureContent
      );

      // @step And the coverage file links 2 or more test files to the feature
      const coverageContent = {
        scenarios: [
          {
            name: 'Scenario 1',
            testMappings: [
              {
                file: 'src/__tests__/test1.test.ts',
                lines: '10-15',
              },
            ],
          },
          {
            name: 'Scenario 2',
            testMappings: [
              {
                file: 'src/__tests__/test2.test.ts',
                lines: '10-15',
              },
            ],
          },
        ],
      };
      await writeFile(
        join(featuresDir, 'test-feature.feature.coverage'),
        JSON.stringify(coverageContent, null, 2)
      );

      // Create test files with @step comments
      const testContent1 = `// Feature: spec/features/test-feature.feature

describe('Scenario 1', () => {
  it('test', () => {
    // @step Given step 1
    // @step When action 1
    // @step Then result 1
  });
});
`;
      await writeFile(
        join(testDir, 'src/__tests__/test1.test.ts'),
        testContent1
      );

      const testContent2 = `// Feature: spec/features/test-feature.feature

describe('Scenario 2', () => {
  it('test', () => {
    // @step Given step 2
    // @step When action 2
    // @step Then result 2
  });
});
`;
      await writeFile(
        join(testDir, 'src/__tests__/test2.test.ts'),
        testContent2
      );

      // Create work unit
      await writeFile(
        workUnitsFile,
        JSON.stringify(
          {
            workUnits: {
              'TEST-002': {
                id: 'TEST-002',
                type: 'story',
                title: 'Test Feature',
                status: 'testing',
                createdAt: new Date().toISOString(),
                updatedAt: new Date().toISOString(),
                stateHistory: [
                  {
                    state: 'testing',
                    timestamp: new Date(Date.now() - 1000).toISOString(),
                  },
                ],
              },
            },
            states: {
              backlog: [],
              specifying: [],
              testing: ['TEST-002'],
              implementing: [],
              validating: [],
              done: [],
              blocked: [],
            },
          },
          null,
          2
        )
      );

      // @step When validation runs
      // @step Then an error should be thrown
      // @step And the error should say "Multiple test files detected"
      // @step And the error should say "Split feature file"
      // @step And the transition should be blocked
      await expect(
        updateWorkUnitStatus({
          workUnitId: 'TEST-002',
          status: 'implementing',
          cwd: testDir,
          skipTemporalValidation: true,
        })
      ).rejects.toThrow(/Multiple test files detected/);

      await expect(
        updateWorkUnitStatus({
          workUnitId: 'TEST-002',
          status: 'implementing',
          cwd: testDir,
          skipTemporalValidation: true,
        })
      ).rejects.toThrow(/Split feature file/);
    });
  });
});
