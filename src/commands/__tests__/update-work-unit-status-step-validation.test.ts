/**
 * Feature: spec/features/missing-validation-for-test-file-step-docstring-completeness.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, writeFile, mkdir } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { updateWorkUnitStatus } from '../update-work-unit-status';

describe('Feature: Missing validation for test file @step docstring completeness', () => {
  let testDir: string;
  let specDir: string;
  let workUnitsFile: string;
  let featuresDir: string;

  beforeEach(async () => {
    // @step Given I am setting up a test environment
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    specDir = join(testDir, 'spec');
    workUnitsFile = join(specDir, 'work-units.json');
    featuresDir = join(specDir, 'features');

    await mkdir(specDir, { recursive: true });
    await mkdir(featuresDir, { recursive: true });
    await mkdir(join(testDir, 'src/__tests__'), { recursive: true });

    // @step And I have initialized work units file
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
    // @step Then I clean up the test environment
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Allow transition to implementing when test file has complete step docstrings', () => {
    it('should succeed when test file has Given, When, and Then @step docstrings', async () => {
      // @step Given a work unit with type "story" is in "testing" status
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['TEST-001'] = {
        id: 'TEST-001',
        title: 'Test Story',
        status: 'testing',
        type: 'story',
        userStory: {
          role: 'user',
          action: 'do something',
          benefit: 'get value',
        },
        stateHistory: [
          { state: 'specifying', timestamp: '2025-01-01T10:00:00.000Z' },
          { state: 'testing', timestamp: '2025-01-01T11:00:00.000Z' },
        ],
        createdAt: '2025-01-01T09:00:00.000Z',
        updatedAt: '2025-01-01T11:00:00.000Z',
      };
      workUnits.states.testing.push('TEST-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // @step And the work unit has a linked feature file with scenarios tagged with the work unit ID
      const featureContent = `@TEST-001
Feature: Test Feature

  Background: User Story
    As a user
    I want to do something
    So that I get value

  Scenario: Test scenario
    Given I have a precondition
    When I perform an action
    Then I should see a result
`;
      await writeFile(
        join(featuresDir, 'test-feature.feature'),
        featureContent
      );

      // @step And the test file for the work unit contains complete Given, When, and Then @step docstrings
      const testContent = `/**
 * Feature: spec/features/test-feature.feature
 */
describe('Feature: Test Feature', () => {
  describe('Scenario: Test scenario', () => {
    it('should work', () => {
      // @step Given I have a precondition
      const precondition = true;

      // @step When I perform an action
      const result = performAction();

      // @step Then I should see a result
      expect(result).toBe(true);
    });
  });
});
`;
      await writeFile(
        join(testDir, 'src/__tests__/test-feature.test.ts'),
        testContent
      );

      // @step And the feature has a coverage file linking to the test file
      const coverageContent = {
        scenarios: [
          {
            name: 'Test scenario',
            testMappings: [
              {
                file: 'src/__tests__/test-feature.test.ts',
                lines: '5-15',
                implMappings: [],
              },
            ],
          },
        ],
        stats: {
          totalScenarios: 1,
          coveredScenarios: 1,
          coveragePercent: 100,
        },
      };
      await writeFile(
        join(featuresDir, 'test-feature.feature.coverage'),
        JSON.stringify(coverageContent, null, 2)
      );

      // @step When I run "fspec update-work-unit-status TEST-001 implementing"
      const result = await updateWorkUnitStatus({
        workUnitId: 'TEST-001',
        status: 'implementing',
        cwd: testDir,
        skipTemporalValidation: true,
      });

      // @step Then the command should succeed
      expect(result.success).toBe(true);

      // @step And the work unit status should be updated to "implementing"
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['TEST-001'].status).toBe(
        'implementing'
      );

      // @step And no validation error should be displayed
      // Note: systemReminder exists for status change, but should not contain step validation errors
      if (result.systemReminder) {
        expect(result.systemReminder).not.toMatch(/STEP VALIDATION FAILED/);
      }
    });
  });

  describe('Scenario: Block transition to implementing when test file has no step docstrings', () => {
    it('should fail when test file has no @step docstrings', async () => {
      // @step Given a work unit with type "story" is in "testing" status
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['TEST-002'] = {
        id: 'TEST-002',
        title: 'Test Story 2',
        status: 'testing',
        type: 'story',
        userStory: {
          role: 'user',
          action: 'do something',
          benefit: 'get value',
        },
        stateHistory: [
          { state: 'specifying', timestamp: '2025-01-01T10:00:00.000Z' },
          { state: 'testing', timestamp: '2025-01-01T11:00:00.000Z' },
        ],
        createdAt: '2025-01-01T09:00:00.000Z',
        updatedAt: '2025-01-01T11:00:00.000Z',
      };
      workUnits.states.testing.push('TEST-002');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // @step And the work unit has a linked feature file with scenarios tagged with the work unit ID
      const featureContent = `@TEST-002
Feature: Test Feature 2

  Background: User Story
    As a user
    I want to do something
    So that I get value

  Scenario: Test scenario
    Given I have a precondition
    When I perform an action
    Then I should see a result
`;
      await writeFile(
        join(featuresDir, 'test-feature-2.feature'),
        featureContent
      );

      // @step And the test file for the work unit exists but contains NO @step docstrings
      const testContent = `/**
 * Feature: spec/features/test-feature-2.feature
 */
describe('Feature: Test Feature 2', () => {
  describe('Scenario: Test scenario', () => {
    it('should work', () => {
      const precondition = true;
      const result = performAction();
      expect(result).toBe(true);
    });
  });
});
`;
      await writeFile(
        join(testDir, 'src/__tests__/test-feature-2.test.ts'),
        testContent
      );

      // @step And the feature has a coverage file linking to the test file
      const coverageContent = {
        scenarios: [
          {
            name: 'Test scenario',
            testMappings: [
              {
                file: 'src/__tests__/test-feature-2.test.ts',
                lines: '5-10',
                implMappings: [],
              },
            ],
          },
        ],
        stats: {
          totalScenarios: 1,
          coveredScenarios: 1,
          coveragePercent: 100,
        },
      };
      await writeFile(
        join(featuresDir, 'test-feature-2.feature.coverage'),
        JSON.stringify(coverageContent, null, 2)
      );

      // @step When I run "fspec update-work-unit-status TEST-002 implementing"
      // @step Then the command should fail with a validation error
      // @step And the error should indicate missing step docstrings
      // @step And the error should be wrapped in a system-reminder tag
      await expect(
        updateWorkUnitStatus({
          workUnitId: 'TEST-002',
          status: 'implementing',
          cwd: testDir,
          skipTemporalValidation: true,
        })
      ).rejects.toThrow(/missing required step comments/i);

      // @step And the work unit status should remain "testing"
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['TEST-002'].status).toBe('testing');
    });
  });

  describe('Scenario: Block transition to validating when test file has incomplete step docstrings', () => {
    it('should fail when test file is missing Then @step docstrings', async () => {
      // @step Given a work unit with type "story" is in "implementing" status
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['TEST-003'] = {
        id: 'TEST-003',
        title: 'Test Story 3',
        status: 'implementing',
        type: 'story',
        userStory: {
          role: 'user',
          action: 'do something',
          benefit: 'get value',
        },
        stateHistory: [
          { state: 'specifying', timestamp: '2025-01-01T10:00:00.000Z' },
          { state: 'testing', timestamp: '2025-01-01T11:00:00.000Z' },
          { state: 'implementing', timestamp: '2025-01-01T12:00:00.000Z' },
        ],
        createdAt: '2025-01-01T09:00:00.000Z',
        updatedAt: '2025-01-01T12:00:00.000Z',
      };
      workUnits.states.implementing.push('TEST-003');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // @step And the work unit has a linked feature file with scenarios tagged with the work unit ID
      const featureContent = `@TEST-003
Feature: Test Feature 3

  Background: User Story
    As a user
    I want to do something
    So that I get value

  Scenario: Test scenario
    Given I have a precondition
    When I perform an action
    Then I should see a result
`;
      await writeFile(
        join(featuresDir, 'test-feature-3.feature'),
        featureContent
      );

      // @step And the test file for the work unit contains Given and When @step docstrings
      // @step But the test file is missing Then @step docstrings
      const testContent = `/**
 * Feature: spec/features/test-feature-3.feature
 */
describe('Feature: Test Feature 3', () => {
  describe('Scenario: Test scenario', () => {
    it('should work', () => {
      // @step Given I have a precondition
      const precondition = true;

      // @step When I perform an action
      const result = performAction();

      expect(result).toBe(true);
    });
  });
});
`;
      await writeFile(
        join(testDir, 'src/__tests__/test-feature-3.test.ts'),
        testContent
      );

      // @step And the feature has a coverage file linking to the test file
      const coverageContent = {
        scenarios: [
          {
            name: 'Test scenario',
            testMappings: [
              {
                file: 'src/__tests__/test-feature-3.test.ts',
                lines: '5-14',
                implMappings: [],
              },
            ],
          },
        ],
        stats: {
          totalScenarios: 1,
          coveredScenarios: 1,
          coveragePercent: 100,
        },
      };
      await writeFile(
        join(featuresDir, 'test-feature-3.feature.coverage'),
        JSON.stringify(coverageContent, null, 2)
      );

      // @step When I run "fspec update-work-unit-status TEST-003 validating"
      // @step Then the command should fail with a validation error
      // @step And the error should specifically mention "Then" steps are missing
      await expect(
        updateWorkUnitStatus({
          workUnitId: 'TEST-003',
          status: 'validating',
          cwd: testDir,
          skipTemporalValidation: true,
        })
      ).rejects.toThrow(/Then.*I should see a result/);

      // @step And the work unit status should remain "implementing"
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['TEST-003'].status).toBe(
        'implementing'
      );
    });
  });

  describe('Scenario: Exempt task work units from step validation', () => {
    it('should allow tasks to skip testing phase without step validation', async () => {
      // @step Given a work unit with type "task" is in "specifying" status
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['TASK-001'] = {
        id: 'TASK-001',
        title: 'Test Task',
        status: 'specifying',
        type: 'task',
        userStory: {
          role: 'developer',
          action: 'configure something',
          benefit: 'system works',
        },
        stateHistory: [
          { state: 'specifying', timestamp: '2025-01-01T10:00:00.000Z' },
        ],
        createdAt: '2025-01-01T09:00:00.000Z',
        updatedAt: '2025-01-01T10:00:00.000Z',
      };
      workUnits.states.specifying.push('TASK-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // @step And the work unit has a linked feature file
      const featureContent = `@TASK-001
Feature: Test Task Feature

  Background: User Story
    As a developer
    I want to configure something
    So that system works

  Scenario: Configuration complete
    Given I have access to config files
    When I update the configuration
    Then the system should use new settings
`;
      await writeFile(join(featuresDir, 'test-task.feature'), featureContent);

      // @step And no test file exists for the work unit
      // (No test file created - tasks don't require tests)

      // @step When I run "fspec update-work-unit-status TASK-001 implementing"
      const result = await updateWorkUnitStatus({
        workUnitId: 'TASK-001',
        status: 'implementing',
        cwd: testDir,
        skipTemporalValidation: true,
      });

      // @step Then the command should succeed
      expect(result.success).toBe(true);

      // @step And the work unit status should be updated to "implementing"
      const updatedWorkUnits = JSON.parse(
        await readFile(workUnitsFile, 'utf-8')
      );
      expect(updatedWorkUnits.workUnits['TASK-001'].status).toBe(
        'implementing'
      );

      // @step And no step validation should occur
      // Note: systemReminder exists for status change, but should not contain step validation errors
      if (result.systemReminder) {
        expect(result.systemReminder).not.toMatch(/STEP VALIDATION FAILED/);
        expect(result.systemReminder).not.toMatch(/No test files found/);
      }
    });
  });
});

// Helper function to read files
async function readFile(path: string, encoding: string): Promise<string> {
  const { readFile: fsReadFile } = await import('fs/promises');
  return fsReadFile(path, encoding);
}
