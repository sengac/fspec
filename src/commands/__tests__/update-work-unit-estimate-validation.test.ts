/**
 * Feature: spec/features/prevent-story-point-estimation-before-feature-file-completion.feature
 *
 * This test file validates the acceptance criteria for preventing premature story point estimation.
 * Tests map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile } from 'fs/promises';
import { join } from 'path';
import { updateWorkUnitEstimate } from '../update-work-unit-estimate';
import {
  setupWorkUnitTest,
  type WorkUnitTestSetup,
} from '../../test-helpers/universal-test-setup';
import { writeJsonTestFile } from '../../test-helpers/test-file-operations';

describe('Feature: Prevent story point estimation before feature file completion', () => {
  let setup: WorkUnitTestSetup;

  beforeEach(async () => {
    setup = await setupWorkUnitTest('update-work-unit-estimate-validation');
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: Block estimation for story work unit without feature file', () => {
    it('should throw error when estimating story work unit without feature file', async () => {
      // Given I have a story work unit "AUTH-001" in "backlog" state
      // And the work unit has no linked feature file
      const workUnitsFile = setup.workUnitsFile;
      await writeJsonTestFile(workUnitsFile, {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'User Authentication',
            type: 'story',
            status: 'backlog',
            createdAt: '2025-01-15T10:00:00.000Z',
          },
        },
        states: {
          backlog: ['AUTH-001'],
        },
      });

      // When I run "fspec update-work-unit-estimate AUTH-001 5"
      // Then the command should exit with code 1
      // And the output should contain a system-reminder explaining ACDD estimation rules
      await expect(
        updateWorkUnitEstimate({
          workUnitId: 'AUTH-001',
          estimate: 5,
          cwd: setup.testDir,
        })
      ).rejects.toThrow(/ACDD requires feature file/);
    });
  });

  describe('Scenario: Allow estimation for story work unit with completed feature file', () => {
    it('should allow estimation when feature file exists and is complete', async () => {
      // Given I have a story work unit "AUTH-001" in "testing" state
      // And the work unit has a linked feature file with complete scenarios
      // And the feature file has no prefill placeholders
      const workUnitsFile = setup.workUnitsFile;
      await writeJsonTestFile(workUnitsFile, {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'User Authentication',
            type: 'story',
            status: 'testing',
            createdAt: '2025-01-15T10:00:00.000Z',
          },
        },
        states: {
          testing: ['AUTH-001'],
        },
      });

      // Create feature file with complete content (no prefill)
      const featureFile = join(
        setup.testDir,
        'spec',
        'features',
        'user-authentication.feature'
      );
      await writeFile(
        featureFile,
        `@AUTH-001
Feature: User Authentication

  Background: User Story
    As a user
    I want to log in securely
    So that I can access protected features

  Scenario: Login with valid credentials
    Given I am on the login page
    When I enter valid credentials
    Then I should be logged in
`
      );

      // When I run "fspec update-work-unit-estimate AUTH-001 5"
      const result = await updateWorkUnitEstimate({
        workUnitId: 'AUTH-001',
        estimate: 5,
        cwd: setup.testDir,
      });

      // Then the command should exit with code 0
      // And the work unit estimate should be updated to 5
      expect(result.success).toBe(true);
    });
  });

  describe('Scenario: Allow estimation for task work unit at any stage', () => {
    it('should allow estimation for task work units without feature file', async () => {
      // Given I have a task work unit "TASK-001" in "backlog" state
      // And the work unit has no linked feature file
      const workUnitsFile = setup.workUnitsFile;
      await writeJsonTestFile(workUnitsFile, {
        workUnits: {
          'TASK-001': {
            id: 'TASK-001',
            title: 'Setup CI/CD Pipeline',
            type: 'task',
            status: 'backlog',
            createdAt: '2025-01-15T10:00:00.000Z',
          },
        },
        states: {
          backlog: ['TASK-001'],
        },
      });

      // When I run "fspec update-work-unit-estimate TASK-001 3"
      const result = await updateWorkUnitEstimate({
        workUnitId: 'TASK-001',
        estimate: 3,
        cwd: setup.testDir,
      });

      // Then the command should exit with code 0
      // And the work unit estimate should be updated to 3
      expect(result.success).toBe(true);
    });
  });

  describe('Scenario: Block estimation for bug work unit with incomplete feature file', () => {
    it('should throw error when feature file has prefill placeholders', async () => {
      // Given I have a bug work unit "BUG-001" in "specifying" state
      // And the work unit has a linked feature file with prefill placeholders
      const workUnitsFile = setup.workUnitsFile;
      await writeJsonTestFile(workUnitsFile, {
        workUnits: {
          'BUG-001': {
            id: 'BUG-001',
            title: 'Fix login bug',
            type: 'bug',
            status: 'specifying',
            createdAt: '2025-01-15T10:00:00.000Z',
          },
        },
        states: {
          specifying: ['BUG-001'],
        },
      });

      // Create feature file with prefill placeholders
      const featureFile = join(
        setup.testDir,
        'spec',
        'features',
        'fix-login-bug.feature'
      );
      await writeFile(
        featureFile,
        `@BUG-001
Feature: Fix login bug

  Background: User Story
    As a [role]
    I want to [action]
    So that [benefit]

  Scenario: [scenario name]
    Given [precondition]
    When [action]
    Then [expected outcome]
`
      );

      // When I run "fspec update-work-unit-estimate BUG-001 2"
      // Then the command should exit with code 1
      // And the output should contain a system-reminder about incomplete feature file
      await expect(
        updateWorkUnitEstimate({
          workUnitId: 'BUG-001',
          estimate: 2,
          cwd: setup.testDir,
        })
      ).rejects.toThrow(/prefill placeholders must be removed/);
    });
  });
});
