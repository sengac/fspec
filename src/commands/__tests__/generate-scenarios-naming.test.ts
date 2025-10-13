/**
 * Feature: spec/features/generate-scenarios-with-capability-based-naming.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, readFile, mkdir, writeFile } from 'fs/promises';
import { existsSync } from 'fs';
import { tmpdir } from 'os';
import { join } from 'path';
import { generateScenarios } from '../example-mapping';

describe('Feature: Generate Scenarios with Capability-Based Naming', () => {
  let testDir: string;
  let specDir: string;
  let workUnitsFile: string;
  let featuresDir: string;

  beforeEach(async () => {
    // Create temporary directory for each test
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    specDir = join(testDir, 'spec');
    workUnitsFile = join(specDir, 'work-units.json');
    featuresDir = join(specDir, 'features');

    // Create spec directory structure
    await mkdir(specDir, { recursive: true });
    await mkdir(featuresDir, { recursive: true });

    // Initialize work units file
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
    // Clean up temporary directory
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Generate scenarios using work unit title as default', () => {
    it('should create feature file named after work unit title, not ID', async () => {
      // Given I have a work unit "AUTH-001" with title "User Authentication"
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
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
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec generate-scenarios AUTH-001"
      const result = await generateScenarios('AUTH-001', { cwd: testDir });

      // Then a feature file "spec/features/user-authentication.feature" should be created
      const expectedPath = join(featuresDir, 'user-authentication.feature');
      expect(existsSync(expectedPath)).toBe(true);

      // And the file should contain scenarios generated from the examples
      const featureContent = await readFile(expectedPath, 'utf-8');
      expect(featureContent).toContain('Scenario:');
      expect(featureContent).toContain('@AUTH-001');

      // And the file should NOT be named "auth-001.feature"
      const wrongPath = join(featuresDir, 'auth-001.feature');
      expect(existsSync(wrongPath)).toBe(false);
    });
  });

  describe('Scenario: Generate scenarios with explicit feature name override', () => {
    it('should create feature file with specified name when --feature provided', async () => {
      // Given I have a work unit "AUTH-001" with title "User Login"
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        title: 'User Login',
        status: 'specifying',
        examples: [
          'User logs in with email',
          'User logs in with username',
        ],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec generate-scenarios AUTH-001 --feature=user-authentication"
      const result = await generateScenarios('AUTH-001', {
        cwd: testDir,
        feature: 'user-authentication',
      });

      // Then a feature file "spec/features/user-authentication.feature" should be created
      const expectedPath = join(featuresDir, 'user-authentication.feature');
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
      const workUnits = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
      workUnits.workUnits['AUTH-001'] = {
        id: 'AUTH-001',
        // No title property
        status: 'specifying',
        examples: ['User logs in with credentials'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      workUnits.states.specifying.push('AUTH-001');
      await writeFile(workUnitsFile, JSON.stringify(workUnits, null, 2));

      // When I run "fspec generate-scenarios AUTH-001"
      // Then the command should fail with exit code 1
      await expect(
        generateScenarios('AUTH-001', { cwd: testDir })
      ).rejects.toThrow();

      // And the error message should contain "Cannot determine feature file name"
      await expect(
        generateScenarios('AUTH-001', { cwd: testDir })
      ).rejects.toThrow('Cannot determine feature file name');

      // And the error message should suggest using "--feature flag with a capability-based name"
      await expect(
        generateScenarios('AUTH-001', { cwd: testDir })
      ).rejects.toThrow('--feature');
    });
  });
});
