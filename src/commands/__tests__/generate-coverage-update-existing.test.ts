/**
 * Feature: spec/features/generate-coverage-does-not-detect-new-scenarios-in-existing-feature-files.feature
 * Bug: BUG-007 - generate-coverage does not detect new scenarios in existing feature files
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { join } from 'path';
import { writeFile, readFile } from 'fs/promises';
import {
  setupTestDirectory,
  type TestDirectorySetup,
} from '../../test-helpers/universal-test-setup';
import { ensureTestDirectory } from '../../test-helpers/test-file-operations';

describe('Feature: generate-coverage updates existing .coverage files', () => {
  let setup: TestDirectorySetup;
  let featuresDir: string;

  beforeEach(async () => {
    // Given I have a project with spec/features directory
    setup = await setupTestDirectory('generate-coverage-update-existing');
    featuresDir = join(setup.testDir, 'spec', 'features');
    await ensureTestDirectory(featuresDir);
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: Add new scenarios to existing .coverage file', () => {
    it('should detect and add new scenarios when feature file has more scenarios than coverage file', async () => {
      // Given I have a feature file with 3 scenarios
      const featureContent = `@TEST-001
Feature: Test Feature

  Background: User Story
    As a developer
    I want to test coverage
    So that I can verify it works

  Scenario: Existing scenario 1
    Given a precondition
    When I do something
    Then it should work

  Scenario: Existing scenario 2
    Given another precondition
    When I do something else
    Then that should work too

  Scenario: New scenario 3
    Given a new precondition
    When I do a new thing
    Then the new thing should work
`;

      const featureFile = join(featuresDir, 'test-feature.feature');
      await writeFile(featureFile, featureContent);

      // And I have a .coverage file with only 2 scenarios (missing the 3rd)
      const coverageData = {
        scenarios: [
          {
            name: 'Existing scenario 1',
            testMappings: [],
          },
          {
            name: 'Existing scenario 2',
            testMappings: [],
          },
        ],
        stats: {
          totalScenarios: 2,
          coveredScenarios: 0,
          coveragePercent: 0,
          testFiles: [],
          implFiles: [],
          totalLinesCovered: 0,
        },
      };

      const coverageFile = join(featuresDir, 'test-feature.feature.coverage');
      await writeFile(coverageFile, JSON.stringify(coverageData, null, 2));

      // When I run generate-coverage
      const { generateCoverage } = await import('../generate-coverage');
      await generateCoverage({ cwd: setup.testDir });

      // Then the coverage file should now have 3 scenarios
      const updatedCoverage = JSON.parse(await readFile(coverageFile, 'utf-8'));

      expect(updatedCoverage.scenarios).toHaveLength(3);
      expect(updatedCoverage.scenarios[0].name).toBe('Existing scenario 1');
      expect(updatedCoverage.scenarios[1].name).toBe('Existing scenario 2');
      expect(updatedCoverage.scenarios[2].name).toBe('New scenario 3');

      // And stats should be updated
      expect(updatedCoverage.stats.totalScenarios).toBe(3);
    });

    it('should preserve existing test mappings when adding new scenarios', async () => {
      // Given I have a feature file with 2 scenarios
      const featureContent = `@TEST-002
Feature: Another Test

  Background: User Story
    As a developer
    I want to preserve mappings
    So that existing coverage data is not lost

  Scenario: Scenario with coverage
    Given it has test mapping
    When I run generate-coverage
    Then mapping should be preserved

  Scenario: New scenario without coverage
    Given it has no test mapping
    When I run generate-coverage
    Then it should be added
`;

      const featureFile = join(featuresDir, 'another-test.feature');
      await writeFile(featureFile, featureContent);

      // And the coverage file has 1 scenario with test mappings
      const coverageData = {
        scenarios: [
          {
            name: 'Scenario with coverage',
            testMappings: [
              {
                file: 'src/__tests__/test.test.ts',
                lines: '10-20',
                implMappings: [
                  {
                    file: 'src/test.ts',
                    lines: [5, 6, 7],
                  },
                ],
              },
            ],
          },
        ],
        stats: {
          totalScenarios: 1,
          coveredScenarios: 1,
          coveragePercent: 100,
          testFiles: ['src/__tests__/test.test.ts'],
          implFiles: ['src/test.ts'],
          totalLinesCovered: 10,
        },
      };

      const coverageFile = join(featuresDir, 'another-test.feature.coverage');
      await writeFile(coverageFile, JSON.stringify(coverageData, null, 2));

      // When I run generate-coverage
      const { generateCoverage } = await import('../generate-coverage');
      await generateCoverage({ cwd: setup.testDir });

      // Then the existing test mapping should be preserved
      const updatedCoverage = JSON.parse(await readFile(coverageFile, 'utf-8'));

      expect(updatedCoverage.scenarios).toHaveLength(2);
      expect(updatedCoverage.scenarios[0].name).toBe('Scenario with coverage');
      expect(updatedCoverage.scenarios[0].testMappings).toHaveLength(1);
      expect(updatedCoverage.scenarios[0].testMappings[0].file).toBe(
        'src/__tests__/test.test.ts'
      );
      expect(updatedCoverage.scenarios[0].testMappings[0].lines).toBe('10-20');
      expect(
        updatedCoverage.scenarios[0].testMappings[0].implMappings
      ).toHaveLength(1);

      // And the new scenario should be added with empty mappings
      expect(updatedCoverage.scenarios[1].name).toBe(
        'New scenario without coverage'
      );
      expect(updatedCoverage.scenarios[1].testMappings).toEqual([]);
    });
  });
});
