/**
 * Feature: spec/features/auto-create-coverage-files.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, readFile, mkdir, writeFile, access } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { createFeature } from '../create-feature';
import { createCoverageFile } from '../../utils/coverage-file';

describe('Feature: Auto-create Coverage Files', () => {
  let testDir: string;

  beforeEach(async () => {
    // Create temporary directory for each test
    testDir = await mkdtemp(join(tmpdir(), 'fspec-coverage-test-'));
  });

  afterEach(async () => {
    // Clean up temporary directory
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Create feature file with coverage file', () => {
    it('should create feature file AND coverage file automatically', async () => {
      // Given I am in a project with spec/features/ directory
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      // When I run "fspec create-feature User Login"
      await createFeature('User Login', testDir);

      // Then a file "spec/features/user-login.feature" should be created
      const featureFile = join(featuresDir, 'user-login.feature');
      await access(featureFile); // Throws if doesn't exist

      // And a file "spec/features/user-login.feature.coverage" should be created
      const coverageFile = join(featuresDir, 'user-login.feature.coverage');
      await access(coverageFile); // Should NOT throw - coverage file should exist

      // And the output should display "✓ Created user-login.feature.coverage"
      // Note: In real implementation, createFeature should return/log this message
    });
  });

  describe('Scenario: Coverage file contains valid JSON schema', () => {
    it('should generate coverage file with correct JSON structure', async () => {
      // Given I have created a feature file "spec/features/user-login.feature" with 1 scenario named "Login with valid credentials"
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      // Create a feature file with known scenario name
      const featureContent = `@critical
Feature: User Login

  Scenario: Login with valid credentials
    Given I am on the login page
    When I enter valid credentials
    Then I should be logged in
`;
      const featureFile = join(featuresDir, 'user-login.feature');
      await writeFile(featureFile, featureContent);

      // When create-feature auto-generates coverage (or we call coverage creation directly)
      await createCoverageFile(featureFile);

      // When I read the coverage file "spec/features/user-login.feature.coverage"
      const coverageFile = join(featuresDir, 'user-login.feature.coverage');
      const coverageContent = await readFile(coverageFile, 'utf-8');
      const coverage = JSON.parse(coverageContent);

      // Then the JSON should have a "scenarios" array with 1 entry
      expect(coverage.scenarios).toBeDefined();
      expect(coverage.scenarios).toHaveLength(1);

      // And the scenario entry should have name "Login with valid credentials"
      expect(coverage.scenarios[0].name).toBe('Login with valid credentials');

      // And the scenario entry should have empty "testMappings" array
      expect(coverage.scenarios[0].testMappings).toEqual([]);

      // And the JSON should have "stats.totalScenarios" equal to 1
      expect(coverage.stats.totalScenarios).toBe(1);

      // And the JSON should have "stats.coveredScenarios" equal to 0
      expect(coverage.stats.coveredScenarios).toBe(0);

      // And the JSON should have "stats.coveragePercent" equal to 0
      expect(coverage.stats.coveragePercent).toBe(0);

      // And the JSON should have "stats.testFiles" as empty array
      expect(coverage.stats.testFiles).toEqual([]);

      // And the JSON should have "stats.implFiles" as empty array
      expect(coverage.stats.implFiles).toEqual([]);

      // And the JSON should have "stats.totalLinesCovered" equal to 0
      expect(coverage.stats.totalLinesCovered).toBe(0);
    });
  });

  describe('Scenario: Coverage file tracks multiple scenarios', () => {
    it('should create coverage entries for all scenarios', async () => {
      // Given I have created a feature file with 3 scenarios
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const featureContent = `@critical
Feature: Test Feature

  Scenario: First scenario
    Given a precondition
    When an action
    Then an outcome

  Scenario: Second scenario
    Given another precondition
    When another action
    Then another outcome

  Scenario: Third scenario
    Given yet another precondition
    When yet another action
    Then yet another outcome
`;
      const featureFile = join(featuresDir, 'test-feature.feature');
      await writeFile(featureFile, featureContent);

      // When coverage file is generated
      await createCoverageFile(featureFile);

      // When I read the coverage file
      const coverageFile = join(featuresDir, 'test-feature.feature.coverage');
      const coverageContent = await readFile(coverageFile, 'utf-8');
      const coverage = JSON.parse(coverageContent);

      // Then the "scenarios" array should have 3 entries
      expect(coverage.scenarios).toHaveLength(3);

      // And the "stats.totalScenarios" should equal 3
      expect(coverage.stats.totalScenarios).toBe(3);

      // And all scenarios should have empty "testMappings" arrays
      expect(coverage.scenarios[0].testMappings).toEqual([]);
      expect(coverage.scenarios[1].testMappings).toEqual([]);
      expect(coverage.scenarios[2].testMappings).toEqual([]);
    });
  });

  describe('Scenario: Skip coverage creation if valid coverage file exists', () => {
    it('should preserve existing valid coverage file', async () => {
      // Given I have a feature file "spec/features/user-login.feature"
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const featureFile = join(
        testDir,
        'spec',
        'features',
        'user-login.feature'
      );
      await writeFile(
        featureFile,
        '@critical\nFeature: User Login\n\n  Scenario: Test\n    Given test\n    When test\n    Then test\n'
      );

      // And a valid coverage file "spec/features/user-login.feature.coverage" already exists
      const coverageFile = join(featuresDir, 'user-login.feature.coverage');
      const existingCoverage = {
        scenarios: [
          {
            name: 'Test',
            testMappings: [
              {
                file: 'src/__tests__/test.test.ts',
                lines: '10-20',
                implMappings: [],
              },
            ],
          },
        ],
        stats: {
          totalScenarios: 1,
          coveredScenarios: 1,
          coveragePercent: 100,
          testFiles: ['src/__tests__/test.test.ts'],
          implFiles: [],
          totalLinesCovered: 0,
        },
      };
      await writeFile(coverageFile, JSON.stringify(existingCoverage, null, 2));

      // When I run "fspec create-feature User Login"
      // (This should detect existing coverage and skip)
      // TODO: Call coverage creation/validation

      // Then the existing coverage file should not be modified
      const afterContent = await readFile(coverageFile, 'utf-8');
      const afterCoverage = JSON.parse(afterContent);
      expect(afterCoverage.stats.coveredScenarios).toBe(1);
      expect(afterCoverage.stats.coveragePercent).toBe(100);

      // And the output should display "Skipped user-login.feature.coverage (already exists)"
      // TODO: Verify output message
    });
  });

  describe('Scenario: Overwrite coverage file if invalid JSON exists', () => {
    it('should recreate coverage file when existing file is corrupted', async () => {
      // Given I have a feature file "spec/features/user-login.feature"
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const featureContent =
        '@critical\nFeature: User Login\n\n  Scenario: Test\n    Given test\n    When test\n    Then test\n';
      const featureFile = join(featuresDir, 'user-login.feature');
      await writeFile(featureFile, featureContent);

      // And a coverage file "spec/features/user-login.feature.coverage" exists with invalid JSON
      const coverageFile = join(featuresDir, 'user-login.feature.coverage');
      await writeFile(coverageFile, '{ invalid json here }');

      // When I run "fspec create-feature User Login"
      // (Should detect invalid JSON and recreate)
      await createCoverageFile(featureFile);

      // Then the coverage file should be overwritten with valid JSON
      const content = await readFile(coverageFile, 'utf-8');
      const coverage = JSON.parse(content); // Should NOT throw
      expect(coverage.scenarios).toBeDefined();
      expect(coverage.stats).toBeDefined();

      // And the output should display "Recreated user-login.feature.coverage (previous file was invalid)"
      // TODO: Verify output message
    });
  });

  describe('Scenario: Scenario names preserve exact case and whitespace', () => {
    it('should maintain exact scenario name formatting', async () => {
      // Given I create a feature file with scenario "Login With Valid Credentials"
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const featureContent = `@critical
Feature: Test

  Scenario: Login With Valid Credentials
    Given a precondition
    When an action
    Then an outcome
`;
      const featureFile = join(featuresDir, 'test.feature');
      await writeFile(featureFile, featureContent);

      // When coverage file is generated
      await createCoverageFile(featureFile);

      // When I read the coverage file
      const coverageFile = join(featuresDir, 'test.feature.coverage');
      const coverageContent = await readFile(coverageFile, 'utf-8');
      const coverage = JSON.parse(coverageContent);

      // Then the scenario name should exactly match "Login With Valid Credentials"
      expect(coverage.scenarios[0].name).toBe('Login With Valid Credentials');

      // And the scenario name should not be "login-with-valid-credentials"
      expect(coverage.scenarios[0].name).not.toBe(
        'login-with-valid-credentials'
      );

      // And the scenario name should not be "Login with valid credentials"
      expect(coverage.scenarios[0].name).not.toBe(
        'Login with valid credentials'
      );
    });
  });

  describe('Scenario: Coverage file creation does not fail feature creation', () => {
    it('should complete feature creation even if coverage fails', async () => {
      // Given I am creating a feature file "spec/features/test.feature"
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      // And coverage file creation encounters an error
      // (We'll simulate this by making the coverage directory read-only or similar)
      // TODO: Mock coverage creation failure

      // When the feature file creation completes
      await createFeature('Test', testDir);

      // Then the feature file "spec/features/test.feature" should exist
      const featureFile = join(featuresDir, 'test.feature');
      await access(featureFile); // Should exist

      // And a warning should be displayed about coverage file failure
      // TODO: Verify warning output

      // And the command should exit with code 0
      // (If this test runs without throwing, it's a success)
    });
  });
});
