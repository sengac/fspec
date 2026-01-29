/**
 * Feature: spec/features/generate-scenarios-with-capability-based-naming.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { readFile } from 'fs/promises';
import { existsSync } from 'fs';
import { join } from 'path';
import { generateScenarios } from '../example-mapping';
import {
  setupWorkUnitTest,
  type WorkUnitTestSetup,
} from '../../test-helpers/universal-test-setup';
import {
  writeJsonTestFile,
  readJsonTestFile,
} from '../../test-helpers/test-file-operations';

describe('Feature: Generate Scenarios with Capability-Based Naming', () => {
  let setup: WorkUnitTestSetup;

  beforeEach(async () => {
    // Create temporary directory for each test
    setup = await setupWorkUnitTest('generate-scenarios-naming');

    // Initialize work units file
    const workUnitsData = {
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
    };
    await writeJsonTestFile(setup.workUnitsFile, workUnitsData);
  });

  afterEach(async () => {
    // Clean up temporary directory
    await setup.cleanup();
  });

  describe('Scenario: Generate scenarios using work unit title as default', () => {
    it('should create feature file named after work unit title, not ID', async () => {
      // Given I have a work unit "AUTH-001" with title "User Authentication"
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'User Authentication',
        status: 'specifying',
        examples: [
          'User logs in with valid credentials',
          'User logs in with invalid credentials',
          'User logs out successfully',
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // When I run "fspec generate-scenarios AUTH-001"
      const result = await generateScenarios('AUTH-001', {
        cwd: setup.testDir,
      });

      // Then a feature file "spec/features/user-authentication.feature" should be created
      const expectedPath = join(
        setup.featuresDir,
        'user-authentication.feature'
      );
      expect(existsSync(expectedPath)).toBe(true);

      // And the file should contain scenarios generated from the examples
      const featureContent = await readFile(expectedPath, 'utf-8');
      expect(featureContent).toContain('Scenario:');
      expect(featureContent).toContain('@AUTH-001');

      // And the file should NOT be named "auth-001.feature"
      const wrongPath = join(setup.featuresDir, 'auth-001.feature');
      expect(existsSync(wrongPath)).toBe(false);
    });
  });

  describe('Scenario: Generate scenarios with explicit feature name override', () => {
    it('should create feature file with specified name when --feature provided', async () => {
      // Given I have a work unit "AUTH-001" with title "User Login"
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'User Login',
        status: 'specifying',
        examples: ['User logs in with email', 'User logs in with username'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // When I run "fspec generate-scenarios AUTH-001 --feature=user-authentication"
      const result = await generateScenarios('AUTH-001', {
        cwd: setup.testDir,
        feature: 'user-authentication',
      });

      // Then a feature file "spec/features/user-authentication.feature" should be created
      const expectedPath = join(
        setup.featuresDir,
        'user-authentication.feature'
      );
      expect(existsSync(expectedPath)).toBe(true);

      // And the file should contain scenarios generated from the examples
      const featureContent = await readFile(expectedPath, 'utf-8');
      expect(featureContent).toContain('Scenario:');
      expect(featureContent).toContain('@AUTH-001');
    });
  });

  describe('Scenario: Error when work unit has no title and no --feature flag provided', () => {
    it('should fail with helpful error message', async () => {
      // Given I have a work unit "AUTH-001" with no title set
      const workUnits = await readJsonTestFile(setup.workUnitsFile);
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        // No title property
        status: 'specifying',
        examples: ['User logs in with credentials'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeJsonTestFile(setup.workUnitsFile, workUnits);

      // When I run "fspec generate-scenarios AUTH-001"
      // Then the command should fail with exit code 1
      await expect(
        generateScenarios('AUTH-001', { cwd: setup.testDir })
      ).rejects.toThrow();

      // And the error message should contain "Cannot determine feature file name"
      await expect(
        generateScenarios('AUTH-001', { cwd: setup.testDir })
      ).rejects.toThrow('Cannot determine feature file name');

      // And the error message should suggest using "--feature flag with a capability-based name"
      await expect(
        generateScenarios('AUTH-001', { cwd: setup.testDir })
      ).rejects.toThrow('--feature');
    });
  });
});
