/**
 * Feature: spec/features/coverage-enforcement-gap-21-3-of-features-bypass-validation.feature
 *
 * Tests for COV-054: Coverage enforcement gap auto-discovery from @TAG
 */

import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { describe, it, beforeEach, afterEach, expect } from 'vitest';
import { updateWorkUnitStatus } from '../update-work-unit-status.js';

describe('Feature: Coverage enforcement gap: 21.3% of features bypass validation', () => {
  let tempDir: string;

  beforeEach(async () => {
    // Create temp directory for each test
    tempDir = join(tmpdir(), `fspec-test-${Date.now()}`);
    await mkdir(tempDir, { recursive: true });
    await mkdir(join(tempDir, 'spec'), { recursive: true });
    await mkdir(join(tempDir, 'spec', 'features'), { recursive: true });
  });

  afterEach(async () => {
    // Clean up temp directory
    await rm(tempDir, { recursive: true, force: true });
  });

  describe('Scenario: Auto-discover features from @TAG when linkedFeatures is empty', () => {
    it('should auto-discover tagged feature and run coverage validation', async () => {
      // @step Given a work unit AUTH-001 with linkedFeatures: null
      const workUnitsData = {
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'User Authentication',
            type: 'story',
            status: 'validating',
            linkedFeatures: null, // Explicitly null
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              {
                state: 'backlog',
                timestamp: new Date(Date.now() - 10000).toISOString(),
              },
              {
                state: 'validating',
                timestamp: new Date(Date.now() - 5000).toISOString(),
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
      };

      await writeFile(
        join(tempDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // @step And a feature file spec/features/user-authentication.feature with @AUTH-001 tag
      const featureContent = `@AUTH-001 @critical @cli @validation
Feature: User Authentication

  Background: User Story
    As a developer
    I want to authenticate users
    So that they can access protected features

  Scenario: Login with valid credentials
    Given I am on the login page
    When I enter valid credentials
    Then I should be logged in

  Scenario: Login with invalid credentials
    Given I am on the login page
    When I enter invalid credentials
    Then I should see an error

  Scenario: Logout
    Given I am logged in
    When I click logout
    Then I should be logged out
`;

      await writeFile(
        join(tempDir, 'spec', 'features', 'user-authentication.feature'),
        featureContent
      );

      // @step And the feature has 3 scenarios with 0% coverage
      const coverageContent = {
        featureName: 'user-authentication',
        scenarios: [
          {
            name: 'Login with valid credentials',
            testMappings: [], // No test coverage
          },
          {
            name: 'Login with invalid credentials',
            testMappings: [], // No test coverage
          },
          {
            name: 'Logout',
            testMappings: [], // No test coverage
          },
        ],
        stats: {
          totalScenarios: 3,
          coveredScenarios: 0,
          coveragePercent: 0,
        },
      };

      await writeFile(
        join(
          tempDir,
          'spec',
          'features',
          'user-authentication.feature.coverage'
        ),
        JSON.stringify(coverageContent, null, 2)
      );

      // @step When moving AUTH-001 to done status
      const result = await updateWorkUnitStatus({
        workUnitId: 'AUTH-001',
        status: 'done',
        cwd: tempDir,
      });

      // @step Then auto-discovery should find user-authentication.feature
      // @step And coverage validation should run for all 3 scenarios
      // @step And the status update should fail with coverage error
      expect(result.success).toBe(false);
      expect(result.message).toMatch(/uncovered|coverage/i);
      expect(result.message).toMatch(/user-authentication/);
    });
  });

  describe('Scenario: Explicit linkedFeatures takes precedence over @TAG auto-discovery', () => {
    it('should use explicit linkedFeatures instead of auto-discovered @TAG', async () => {
      // @step Given a work unit AUTH-002 with linkedFeatures: ['custom-feature']
      const workUnitsData = {
        workUnits: {
          'AUTH-002': {
            id: 'AUTH-002',
            title: 'Custom Authentication',
            type: 'story',
            status: 'validating',
            linkedFeatures: ['custom-feature'], // Explicitly set
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              {
                state: 'backlog',
                timestamp: new Date(Date.now() - 10000).toISOString(),
              },
              {
                state: 'validating',
                timestamp: new Date(Date.now() - 5000).toISOString(),
              },
            ],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: ['AUTH-002'],
          done: [],
          blocked: [],
        },
      };

      await writeFile(
        join(tempDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // @step And a feature file spec/features/user-login.feature with @AUTH-002 tag
      const loginFeatureContent = `@AUTH-002 @critical @cli
Feature: User Login

  Scenario: Login
    Given I am on the login page
    When I login
    Then I should be logged in
`;

      await writeFile(
        join(tempDir, 'spec', 'features', 'user-login.feature'),
        loginFeatureContent
      );

      // @step And a feature file spec/features/custom-feature.feature without @AUTH-002 tag
      const customFeatureContent = `@critical @cli
Feature: Custom Feature

  Scenario: Custom scenario
    Given I have a custom setup
    When I do something
    Then it should work
`;

      await writeFile(
        join(tempDir, 'spec', 'features', 'custom-feature.feature'),
        customFeatureContent
      );

      // Create coverage file for custom-feature with 0% coverage
      const customCoverageContent = {
        featureName: 'custom-feature',
        scenarios: [
          {
            name: 'Custom scenario',
            testMappings: [], // No coverage
          },
        ],
        stats: {
          totalScenarios: 1,
          coveredScenarios: 0,
          coveragePercent: 0,
        },
      };

      await writeFile(
        join(tempDir, 'spec', 'features', 'custom-feature.feature.coverage'),
        JSON.stringify(customCoverageContent, null, 2)
      );

      // @step When moving AUTH-002 to done status
      const result = await updateWorkUnitStatus({
        workUnitId: 'AUTH-002',
        status: 'done',
        cwd: tempDir,
      });

      // @step Then coverage validation should use custom-feature.feature
      // @step And coverage validation should NOT use user-login.feature
      // @step And explicit linkedFeatures should override auto-discovery
      expect(result.success).toBe(false);
      expect(result.message).toMatch(/uncovered|coverage/i);
      expect(result.message).toMatch(/custom-feature/);
      expect(result.message).not.toMatch(/user-login/);
    });
  });

  describe('Scenario: Task work units are exempt from coverage validation', () => {
    it('should allow task work units without coverage to progress to done', async () => {
      // @step Given a work unit TASK-001 with type: 'task'
      // @step And linkedFeatures: null
      // @step And no feature files with @TASK-001 tag
      const workUnitsData = {
        workUnits: {
          'TASK-001': {
            id: 'TASK-001',
            title: 'Setup CI/CD',
            type: 'task', // Task type
            status: 'validating',
            linkedFeatures: null,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              {
                state: 'backlog',
                timestamp: new Date(Date.now() - 10000).toISOString(),
              },
              {
                state: 'validating',
                timestamp: new Date(Date.now() - 5000).toISOString(),
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
      };

      await writeFile(
        join(tempDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // @step When moving TASK-001 to done status
      await updateWorkUnitStatus({
        workUnitId: 'TASK-001',
        status: 'done',
        cwd: tempDir,
      });

      // @step Then auto-discovery should find no features
      // @step And the status update should succeed with warning
      // @step And tasks should be exempt from coverage requirements
      // Read the updated work units file to verify status changed to done
      const updatedWorkUnitsData = JSON.parse(
        await readFile(join(tempDir, 'spec', 'work-units.json'), 'utf-8')
      );
      expect(updatedWorkUnitsData.workUnits['TASK-001'].status).toBe('done');
    });
  });

  describe('Scenario: Block progression when auto-discovered feature has incomplete coverage', () => {
    it('should block when auto-discovered feature has 0% coverage', async () => {
      // @step Given a work unit BUG-042 with linkedFeatures: null
      const workUnitsData = {
        workUnits: {
          'BUG-042': {
            id: 'BUG-042',
            title: 'Authentication Bug Fix',
            type: 'bug',
            status: 'validating',
            linkedFeatures: null,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            stateHistory: [
              {
                state: 'backlog',
                timestamp: new Date(Date.now() - 10000).toISOString(),
              },
              {
                state: 'validating',
                timestamp: new Date(Date.now() - 5000).toISOString(),
              },
            ],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: ['BUG-042'],
          done: [],
          blocked: [],
        },
      };

      await writeFile(
        join(tempDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // @step And a feature file spec/features/auth-bug-fix.feature with @BUG-042 tag
      const featureContent = `@BUG-042 @critical @cli @validation
Feature: Auth Bug Fix

  Scenario: Fix login bug
    Given I am on the login page
    When I try to login
    Then the bug should be fixed

  Scenario: Verify no regression
    Given the bug is fixed
    When I test related features
    Then there should be no regressions
`;

      await writeFile(
        join(tempDir, 'spec', 'features', 'auth-bug-fix.feature'),
        featureContent
      );

      // @step And the feature has 2 scenarios
      // @step And coverage shows 0% (0/2 scenarios covered)
      const coverageContent = {
        featureName: 'auth-bug-fix',
        scenarios: [
          {
            name: 'Fix login bug',
            testMappings: [], // No coverage
          },
          {
            name: 'Verify no regression',
            testMappings: [], // No coverage
          },
        ],
        stats: {
          totalScenarios: 2,
          coveredScenarios: 0,
          coveragePercent: 0,
        },
      };

      await writeFile(
        join(tempDir, 'spec', 'features', 'auth-bug-fix.feature.coverage'),
        JSON.stringify(coverageContent, null, 2)
      );

      // @step When moving BUG-042 to done status
      const result = await updateWorkUnitStatus({
        workUnitId: 'BUG-042',
        status: 'done',
        cwd: tempDir,
      });

      // @step Then auto-discovery should find auth-bug-fix.feature
      // @step And coverage validation should detect 0% coverage
      // @step And the status update should fail with coverage error
      expect(result.success).toBe(false);
      expect(result.message).toMatch(/auth-bug-fix/);
      expect(result.message).toMatch(/uncovered|coverage/i);
    });
  });
});
