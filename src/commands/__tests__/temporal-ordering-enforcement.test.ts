/**
 * Feature: spec/features/prevent-retroactive-state-walking-enforce-temporal-ordering.feature
 *
 * Tests for FEAT-011: Prevent retroactive state walking - enforce temporal ordering
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, utimes } from 'fs/promises';
import { join } from 'path';
import { updateWorkUnitStatus } from '../update-work-unit-status';
import { ensureWorkUnitsFile } from '../../utils/ensure-files';
import type { WorkUnitsData } from '../../types';

import {
  createTempTestDir,
  removeTempTestDir,
} from '../../test-helpers/temp-directory';

describe('Feature: Prevent retroactive state walking - enforce temporal ordering', () => {
  let tmpDir: string;

  beforeEach(async () => {
    tmpDir = await createTempTestDir('temporal-ordering');
    await mkdir(join(tmpDir, 'spec', 'features'), { recursive: true });
    await mkdir(join(tmpDir, 'src', '__tests__'), { recursive: true });
  });

  afterEach(async () => {
    await removeTempTestDir(tmpDir);
  });

  describe('Scenario: Detect retroactive feature file creation', () => {
    it('should block transition to testing when feature file exists before specifying state', async () => {
      // Given I have a work unit that entered specifying state at time T1
      const workUnitsData: WorkUnitsData = await ensureWorkUnitsFile(tmpDir);
      const workUnitId = 'TEST-001';
      const specifyingTime = new Date('2025-01-15T10:00:00.000Z');

      workUnitsData.workUnits[workUnitId] = {
        id: workUnitId,
        title: 'Test Feature',
        status: 'specifying',
        type: 'story',
        stateHistory: [
          {
            state: 'specifying',
            timestamp: specifyingTime.toISOString(),
          },
        ],
        createdAt: '2025-01-15T09:00:00.000Z',
        updatedAt: specifyingTime.toISOString(),
        rules: [
          'Feature files must be created after entering specifying state',
        ],
        examples: ['Valid ACDD workflow with proper temporal ordering'],
        architectureNotes: [
          'Implementation: Validate file timestamps during state transitions',
        ],
        attachments: ['spec/attachments/TEST-001/ast-research.json'],
      };

      workUnitsData.states.specifying = [workUnitId];

      await writeFile(
        join(tmpDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // And I have a feature file tagged with @TEST-001 that was created BEFORE T1
      const featureFile = join(
        tmpDir,
        'spec',
        'features',
        'test-feature.feature'
      );
      const featureContent = `@TEST-001
Feature: Test Feature

  Background: User Story
    As a developer
    I want to test temporal ordering
    So that ACDD is enforced

  Scenario: Test scenario
    Given a precondition
    When an action occurs
    Then an outcome is expected
`;

      await writeFile(featureFile, featureContent);

      // Set file modification time to BEFORE specifying state
      const fileTime = new Date('2025-01-15T09:00:00.000Z'); // 1 hour before specifying
      await utimes(featureFile, fileTime, fileTime);

      // When I try to move to testing state
      // Then the command should fail with temporal ordering violation
      const result = await updateWorkUnitStatus({
        workUnitId,
        status: 'testing',
        cwd: tmpDir,
      });

      expect(result.success).toBe(false);
      expect(result.error).toMatch(/ACDD temporal ordering violation/);
      expect(result.error).toMatch(
        /Feature files were created.*BEFORE entering specifying state/
      );
    });
  });

  describe('Scenario: Allow valid ACDD workflow', () => {
    it('should allow transition to testing when feature file created after specifying state', async () => {
      // Given I have a work unit that entered specifying state at time T1
      const workUnitsData: WorkUnitsData = await ensureWorkUnitsFile(tmpDir);
      const workUnitId = 'TEST-002';
      const specifyingTime = new Date('2025-01-15T10:00:00.000Z');

      workUnitsData.workUnits[workUnitId] = {
        id: workUnitId,
        title: 'Test Feature',
        status: 'specifying',
        type: 'story',
        stateHistory: [
          {
            state: 'specifying',
            timestamp: specifyingTime.toISOString(),
          },
        ],
        createdAt: '2025-01-15T09:00:00.000Z',
        updatedAt: specifyingTime.toISOString(),
        rules: [
          'Feature files must be created after entering specifying state',
        ],
        examples: ['Valid ACDD workflow with proper temporal ordering'],
        architectureNotes: [
          'Implementation: Validate file timestamps during state transitions',
        ],
        attachments: ['spec/attachments/TEST-002/ast-research.json'],
      };

      workUnitsData.states.specifying = [workUnitId];

      await writeFile(
        join(tmpDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // And I have a feature file tagged with @TEST-002 that was created AFTER T1
      const featureFile = join(
        tmpDir,
        'spec',
        'features',
        'test-feature.feature'
      );
      const featureContent = `@TEST-002
Feature: Test Feature

  Background: User Story
    As a developer
    I want to test temporal ordering
    So that ACDD is enforced

  Scenario: Test scenario
    Given a precondition
    When an action occurs
    Then an outcome is expected
`;

      await writeFile(featureFile, featureContent);

      // Set file modification time to AFTER specifying state
      const fileTime = new Date('2025-01-15T11:00:00.000Z'); // 1 hour after specifying
      await utimes(featureFile, fileTime, fileTime);

      // When I try to move to testing state
      // Then the command should succeed
      const result = await updateWorkUnitStatus({
        workUnitId,
        status: 'testing',
        cwd: tmpDir,
      });

      expect(result.success).toBe(true);
    });
  });

  describe('Scenario: Detect retroactive test file creation', () => {
    it('should block transition to implementing when test file exists before testing state', async () => {
      // Given I have a work unit that entered testing state at time T2
      const workUnitsData: WorkUnitsData = await ensureWorkUnitsFile(tmpDir);
      const workUnitId = 'TEST-003';
      const specifyingTime = new Date('2025-01-15T10:00:00.000Z');
      const testingTime = new Date('2025-01-15T11:00:00.000Z');

      workUnitsData.workUnits[workUnitId] = {
        id: workUnitId,
        title: 'Test Feature',
        status: 'testing',
        type: 'story',
        stateHistory: [
          {
            state: 'specifying',
            timestamp: specifyingTime.toISOString(),
          },
          {
            state: 'testing',
            timestamp: testingTime.toISOString(),
          },
        ],
        createdAt: '2025-01-15T09:00:00.000Z',
        updatedAt: testingTime.toISOString(),
      };

      workUnitsData.states.testing = [workUnitId];

      await writeFile(
        join(tmpDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // And I have a feature file (required to pass other validation)
      const featureFile = join(
        tmpDir,
        'spec',
        'features',
        'test-feature.feature'
      );
      await writeFile(featureFile, `@TEST-003\nFeature: Test`);
      const featureTime = new Date('2025-01-15T10:30:00.000Z');
      await utimes(featureFile, featureTime, featureTime);

      // And I have a test file that references TEST-003, created BEFORE T2
      const testFile = join(tmpDir, 'src', '__tests__', 'test-feature.test.ts');
      const testContent = `// Feature: spec/features/test-feature.feature
// Work Unit: TEST-003

describe('Feature: Test Feature', () => {
  it('should test something', () => {
    expect(true).toBe(true);
  });
});
`;

      await writeFile(testFile, testContent);

      // Set test file modification time to BEFORE testing state
      const testFileTime = new Date('2025-01-15T10:30:00.000Z'); // 30 min before testing
      await utimes(testFile, testFileTime, testFileTime);

      // When I try to move to implementing state
      // Then the command should fail with temporal ordering violation
      await expect(
        updateWorkUnitStatus({
          workUnitId,
          status: 'implementing',
          cwd: tmpDir,
        })
      ).rejects.toThrow(/ACDD temporal ordering violation/);

      await expect(
        updateWorkUnitStatus({
          workUnitId,
          status: 'implementing',
          cwd: tmpDir,
        })
      ).rejects.toThrow(
        /Test files were created.*BEFORE entering testing state/
      );
    });
  });

  describe('Scenario: Escape hatch with --skip-temporal-validation', () => {
    it('should allow retroactive completion when explicitly skipping validation', async () => {
      // Given I have a work unit with retroactive files (reverse ACDD scenario)
      const workUnitsData: WorkUnitsData = await ensureWorkUnitsFile(tmpDir);
      const workUnitId = 'TEST-004';
      const specifyingTime = new Date('2025-01-15T10:00:00.000Z');

      workUnitsData.workUnits[workUnitId] = {
        id: workUnitId,
        title: 'Imported Feature',
        status: 'specifying',
        type: 'story',
        stateHistory: [
          {
            state: 'specifying',
            timestamp: specifyingTime.toISOString(),
          },
        ],
        createdAt: '2025-01-15T09:00:00.000Z',
        updatedAt: specifyingTime.toISOString(),
        rules: ['Legacy code can be imported with --skip-temporal-validation'],
        examples: [
          'Import existing feature files that predate work unit creation',
        ],
        architectureNotes: [
          'Implementation: Provide escape hatch for legacy code imports',
        ],
        attachments: ['spec/attachments/TEST-004/ast-research.json'],
      };

      workUnitsData.states.specifying = [workUnitId];

      await writeFile(
        join(tmpDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // And the feature file was created BEFORE specifying state (legacy code)
      const featureFile = join(
        tmpDir,
        'spec',
        'features',
        'imported-feature.feature'
      );
      const featureContent = `@TEST-004
Feature: Imported Feature

  Background: User Story
    As a developer
    I want to import existing code
    So that I can track it with fspec

  Scenario: Existing functionality
    Given legacy code exists
    When I import it
    Then it should be tracked
`;

      await writeFile(featureFile, featureContent);

      const fileTime = new Date('2025-01-15T08:00:00.000Z'); // 2 hours before specifying
      await utimes(featureFile, fileTime, fileTime);

      // When I use --skip-temporal-validation flag
      // Then the command should succeed
      const result = await updateWorkUnitStatus({
        workUnitId,
        status: 'testing',
        skipTemporalValidation: true,
        cwd: tmpDir,
      });

      expect(result.success).toBe(true);
    });
  });

  describe('Scenario: Tasks are exempt from test file temporal validation', () => {
    it('should not validate test file timestamps for task work units', async () => {
      // Given I have a task work unit (tasks don't require tests)
      const workUnitsData: WorkUnitsData = await ensureWorkUnitsFile(tmpDir);
      const workUnitId = 'TASK-001';
      const specifyingTime = new Date('2025-01-15T10:00:00.000Z');
      const implementingTime = new Date('2025-01-15T11:00:00.000Z');

      workUnitsData.workUnits[workUnitId] = {
        id: workUnitId,
        title: 'Clean up logs',
        status: 'implementing',
        type: 'task',
        stateHistory: [
          {
            state: 'specifying',
            timestamp: specifyingTime.toISOString(),
          },
          {
            state: 'implementing',
            timestamp: implementingTime.toISOString(),
          },
        ],
        createdAt: '2025-01-15T09:00:00.000Z',
        updatedAt: implementingTime.toISOString(),
      };

      workUnitsData.states.implementing = [workUnitId];

      await writeFile(
        join(tmpDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // When I move to validating state (skipping testing state for tasks)
      // Then the command should succeed without checking for test files
      const result = await updateWorkUnitStatus({
        workUnitId,
        status: 'validating',
        cwd: tmpDir,
      });

      expect(result.success).toBe(true);
    });
  });
});
