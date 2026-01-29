/**
 * Feature: spec/features/duplicate-scenario-detection-in-generate-scenarios-command.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { readFile, mkdir, writeFile } from 'fs/promises';
import { existsSync } from 'fs';
import { join } from 'path';
import {
  setupWorkUnitTest,
  type WorkUnitTestSetup,
} from '../../test-helpers/universal-test-setup';
import { writeJsonTestFile } from '../../test-helpers/test-file-operations';
import { generateScenarios } from '../generate-scenarios';

describe('Feature: Duplicate scenario detection in generate-scenarios command', () => {
  let setup: WorkUnitTestSetup;

  beforeEach(async () => {
    setup = await setupWorkUnitTest('generate-scenarios-duplicate-detection');

    // Initialize work units file
    await writeJsonTestFile(setup.workUnitsFile, {
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
    });
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: Detect duplicate scenarios above threshold and display system-reminder', () => {
    it('should detect duplicates and emit system-reminder with clear next steps', async () => {
      // Given I have a work unit "WORK-001" with example mapping data
      const workUnits = JSON.parse(
        await readFile(setup.workUnitsFile, 'utf-8')
      );
      workUnits.workUnits['WORK-001'] = {
        id: 'WORK-001',
        title: 'User Login',
        status: 'specifying',
        type: 'story',
        userStory: {
          role: 'developer using fspec',
          action: 'detect duplicate scenarios before generating new ones',
          benefit: 'I avoid creating duplicate scenarios across feature files',
        },
        examples: [
          'Given I have valid credentials When I log in Then I should be logged in',
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('WORK-001');
      await writeFile(setup.workUnitsFile, JSON.stringify(workUnits, null, 2));

      // And existing feature files contain similar scenarios above the similarity threshold
      const existingFeature = `@AUTH-001
Feature: User Authentication

  Background: User Story
    As a user
    I want to log in
    So that I can access the system

  Scenario: User logs in with valid credentials
    Given I have valid credentials
    When I log in
    Then I should be logged in
`;
      await writeFile(
        join(setup.featuresDir, 'user-authentication.feature'),
        existingFeature
      );

      // When I run "fspec generate-scenarios WORK-001"
      let caughtError: Error | null = null;
      try {
        await generateScenarios({ workUnitId: 'WORK-001', cwd: setup.testDir });
      } catch (error) {
        caughtError = error as Error;
      }

      // Then the command should detect duplicate scenarios
      expect(caughtError).not.toBeNull();
      expect(caughtError?.message).toContain('duplicate scenarios detected');

      // And a system-reminder should be displayed
      expect(caughtError?.message).toContain('<system-reminder>');

      // And the system-reminder should list feature files to investigate
      expect(caughtError?.message).toContain('user-authentication.feature');

      // And the system-reminder should include clear next steps
      expect(caughtError?.message).toContain('Next steps:');

      // And the system-reminder should provide instructions to use --ignore-possible-duplicates flag
      expect(caughtError?.message).toContain('--ignore-possible-duplicates');
    });
  });

  describe('Scenario: Bypass duplicate check with --ignore-possible-duplicates flag', () => {
    it('should bypass duplicate check and generate scenarios normally', async () => {
      // Given I have reviewed the duplicate scenarios warning
      // And I have investigated the suggested feature files
      // And I have determined the matches are false positives
      const workUnits = JSON.parse(
        await readFile(setup.workUnitsFile, 'utf-8')
      );
      workUnits.workUnits['WORK-001'] = {
        id: 'WORK-001',
        title: 'User Login',
        status: 'specifying',
        type: 'story',
        userStory: {
          role: 'developer using fspec',
          action: 'detect duplicate scenarios before generating new ones',
          benefit: 'I avoid creating duplicate scenarios across feature files',
        },
        examples: [
          'Given I have valid credentials When I log in Then I should be logged in',
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('WORK-001');
      await writeFile(setup.workUnitsFile, JSON.stringify(workUnits, null, 2));

      // Existing feature with similar scenario
      const existingFeature = `@AUTH-001
Feature: User Authentication

  Scenario: User logs in with valid credentials
    Given I have valid credentials
    When I log in
    Then I should be redirected to the dashboard
`;
      await writeFile(
        join(setup.featuresDir, 'user-authentication.feature'),
        existingFeature
      );

      // When I run "fspec generate-scenarios WORK-001 --ignore-possible-duplicates"
      const result = await generateScenarios({
        workUnitId: 'WORK-001',
        cwd: setup.testDir,
        ignorePossibleDuplicates: true,
      });

      // Then the duplicate check should be bypassed
      // And scenarios should be generated normally
      expect(result.success).toBe(true);
      expect(existsSync(result.featureFile)).toBe(true);

      // And no duplicate warning should be displayed (other reminders are OK)
      const hasDuplicateReminder = result.systemReminders?.some(r =>
        r.includes('DUPLICATE SCENARIOS DETECTED')
      );
      expect(hasDuplicateReminder).toBeFalsy();
    });
  });

  describe('Scenario: Generate scenarios normally when no duplicates found', () => {
    it('should generate scenarios without warnings when no duplicates exist', async () => {
      // Given I have a work unit "WORK-001" with example mapping data
      const workUnits = JSON.parse(
        await readFile(setup.workUnitsFile, 'utf-8')
      );
      workUnits.workUnits['WORK-001'] = {
        id: 'WORK-001',
        title: 'User Login',
        status: 'specifying',
        type: 'story',
        userStory: {
          role: 'developer using fspec',
          action: 'detect duplicate scenarios before generating new ones',
          benefit: 'I avoid creating duplicate scenarios across feature files',
        },
        examples: ['User logs in with valid credentials'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('WORK-001');
      await writeFile(setup.workUnitsFile, JSON.stringify(workUnits, null, 2));

      // And no existing feature files contain similar scenarios
      // (no existing features created)

      // When I run "fspec generate-scenarios WORK-001"
      const result = await generateScenarios({
        workUnitId: 'WORK-001',
        cwd: setup.testDir,
      });

      // Then the duplicate check should complete
      // And scenarios should be generated without warnings
      expect(result.success).toBe(true);
      expect(existsSync(result.featureFile)).toBe(true);

      // And no duplicate warning should be displayed (other reminders are OK)
      const hasDuplicateReminder = result.systemReminders?.some(r =>
        r.includes('DUPLICATE SCENARIOS DETECTED')
      );
      expect(hasDuplicateReminder).toBeFalsy();
    });
  });
});
