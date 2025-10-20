/**
 * Feature: spec/features/generate-coverage-files-for-existing-features.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import {
  mkdtemp,
  rm,
  readFile,
  mkdir,
  writeFile,
  access,
  readdir,
} from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { generateCoverage } from '../generate-coverage';

describe('Feature: Generate Coverage Files for Existing Features', () => {
  let testDir: string;

  beforeEach(async () => {
    // Create temporary directory for each test
    testDir = await mkdtemp(join(tmpdir(), 'fspec-generate-coverage-test-'));
  });

  afterEach(async () => {
    // Clean up temporary directory
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Generate coverage files for all features without coverage', () => {
    it('should create coverage files for all .feature files without .coverage files', async () => {
      // Given I have a project with spec/features/ directory
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      // And there are 3 .feature files with no .coverage files
      const featureContent = `@phase1
Feature: Test Feature

  Scenario: Test scenario
    Given a precondition
    When an action
    Then an outcome
`;
      await writeFile(join(featuresDir, 'feature1.feature'), featureContent);
      await writeFile(join(featuresDir, 'feature2.feature'), featureContent);
      await writeFile(join(featuresDir, 'feature3.feature'), featureContent);

      // When I run 'fspec generate-coverage'
      const result = await generateCoverage({ cwd: testDir });

      // Then 3 .feature.coverage files should be created
      await access(join(featuresDir, 'feature1.feature.coverage'));
      await access(join(featuresDir, 'feature2.feature.coverage'));
      await access(join(featuresDir, 'feature3.feature.coverage'));

      // And the output should display 'Created 3 coverage files'
      expect(result.created).toBe(3);
      expect(result.skipped).toBe(0);
      expect(result.recreated).toBe(0);
    });
  });

  describe('Scenario: Skip existing valid coverage files', () => {
    it('should create coverage files only for features without coverage', async () => {
      // Given I have 5 .feature files
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const featureContent = `@phase1
Feature: Test Feature

  Scenario: Test scenario
    Given a precondition
    When an action
    Then an outcome
`;
      await writeFile(join(featuresDir, 'feature1.feature'), featureContent);
      await writeFile(join(featuresDir, 'feature2.feature'), featureContent);
      await writeFile(join(featuresDir, 'feature3.feature'), featureContent);
      await writeFile(join(featuresDir, 'feature4.feature'), featureContent);
      await writeFile(join(featuresDir, 'feature5.feature'), featureContent);

      // And 3 of them already have valid .feature.coverage files
      const validCoverage = {
        scenarios: [
          {
            name: 'Test scenario',
            testMappings: [],
          },
        ],
        stats: {
          totalScenarios: 1,
          coveredScenarios: 0,
          coveragePercent: 0,
          testFiles: [],
          implFiles: [],
          totalLinesCovered: 0,
        },
      };
      await writeFile(
        join(featuresDir, 'feature1.feature.coverage'),
        JSON.stringify(validCoverage, null, 2)
      );
      await writeFile(
        join(featuresDir, 'feature2.feature.coverage'),
        JSON.stringify(validCoverage, null, 2)
      );
      await writeFile(
        join(featuresDir, 'feature3.feature.coverage'),
        JSON.stringify(validCoverage, null, 2)
      );

      // When I run 'fspec generate-coverage'
      const result = await generateCoverage({ cwd: testDir });

      // Then 2 new .feature.coverage files should be created
      await access(join(featuresDir, 'feature4.feature.coverage'));
      await access(join(featuresDir, 'feature5.feature.coverage'));

      // And the 3 existing files should remain unchanged
      const coverage1 = JSON.parse(
        await readFile(join(featuresDir, 'feature1.feature.coverage'), 'utf-8')
      );
      expect(coverage1).toEqual(validCoverage);

      // And the output should display 'Created 2, Skipped 3'
      expect(result.created).toBe(2);
      expect(result.skipped).toBe(3);
      expect(result.recreated).toBe(0);
    });
  });

  describe('Scenario: Recreate corrupted coverage files with invalid JSON', () => {
    it('should overwrite coverage files with invalid JSON', async () => {
      // Given I have a .feature file user-login.feature
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const featureContent = `@phase1
Feature: User Login

  Scenario: Login with valid credentials
    Given I am on the login page
    When I enter valid credentials
    Then I should be logged in
`;
      await writeFile(join(featuresDir, 'user-login.feature'), featureContent);

      // And a corrupted user-login.feature.coverage file with invalid JSON
      await writeFile(
        join(featuresDir, 'user-login.feature.coverage'),
        '{ invalid json here }'
      );

      // When I run 'fspec generate-coverage'
      const result = await generateCoverage({ cwd: testDir });

      // Then the corrupted file should be overwritten with valid JSON
      const coverageContent = await readFile(
        join(featuresDir, 'user-login.feature.coverage'),
        'utf-8'
      );
      const coverage = JSON.parse(coverageContent); // Should NOT throw
      expect(coverage.scenarios).toBeDefined();
      expect(coverage.stats).toBeDefined();
      expect(coverage.scenarios[0].name).toBe('Login with valid credentials');

      // And the output should display 'Recreated 1 (invalid JSON)'
      expect(result.recreated).toBe(1);
      expect(result.created).toBe(0);
      expect(result.skipped).toBe(0);
    });
  });

  describe('Scenario: Dry-run mode previews without creating files', () => {
    it('should preview what would be created without actually creating files', async () => {
      // Given I have 3 .feature files with no .coverage files
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const featureContent = `@phase1
Feature: Test Feature

  Scenario: Test scenario
    Given a precondition
    When an action
    Then an outcome
`;
      await writeFile(join(featuresDir, 'feature1.feature'), featureContent);
      await writeFile(join(featuresDir, 'feature2.feature'), featureContent);
      await writeFile(join(featuresDir, 'feature3.feature'), featureContent);

      // When I run 'fspec generate-coverage --dry-run'
      const result = await generateCoverage({ cwd: testDir, dryRun: true });

      // Then no .coverage files should be created
      const files = await readdir(featuresDir);
      const coverageFiles = files.filter(f => f.endsWith('.coverage'));
      expect(coverageFiles).toHaveLength(0);

      // And the output should display 'Would create 3 coverage files (DRY RUN)'
      expect(result.created).toBe(3);
      expect(result.skipped).toBe(0);
      expect(result.recreated).toBe(0);
      expect(result.dryRun).toBe(true);

      // And the output should list the filenames that would be created
      expect(result.files).toContain('feature1.feature.coverage');
      expect(result.files).toContain('feature2.feature.coverage');
      expect(result.files).toContain('feature3.feature.coverage');
    });
  });
});
