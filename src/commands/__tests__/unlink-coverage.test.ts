/**
 * Feature: spec/features/unlink-coverage-mappings.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, readFile, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { unlinkCoverage } from '../unlink-coverage';

describe('Feature: Unlink Coverage Mappings', () => {
  let testDir: string;

  beforeEach(async () => {
    // Create temporary directory for each test
    testDir = await mkdtemp(join(tmpdir(), 'fspec-unlink-coverage-test-'));
  });

  afterEach(async () => {
    // Clean up temporary directory
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Remove all mappings from scenario with --all flag', () => {
    it('should remove all mappings and update stats', async () => {
      // Given I have a scenario with test and implementation mappings
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const coverageData = {
        scenarios: [
          {
            name: 'Login',
            testMappings: [
              {
                file: 'src/__tests__/auth.test.ts',
                lines: '10-20',
                implMappings: [
                  {
                    file: 'src/auth/login.ts',
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
          testFiles: ['src/__tests__/auth.test.ts'],
          implFiles: ['src/auth/login.ts'],
          totalLinesCovered: 14,
        },
      };

      const coverageFile = join(featuresDir, 'user-login.feature.coverage');
      await writeFile(coverageFile, JSON.stringify(coverageData, null, 2));

      // When I run 'fspec unlink-coverage user-login --scenario "Login" --all'
      const result = await unlinkCoverage('user-login', {
        scenario: 'Login',
        all: true,
        cwd: testDir,
      });

      // Then all mappings should be removed
      expect(result.success).toBe(true);

      // And the scenario should have empty testMappings array
      const updatedContent = await readFile(coverageFile, 'utf-8');
      const updated = JSON.parse(updatedContent);
      expect(updated.scenarios[0].testMappings).toEqual([]);

      // And stats should show coveragePercent decreased
      expect(updated.stats.coveredScenarios).toBe(0);
      expect(updated.stats.coveragePercent).toBe(0);
      expect(updated.stats.testFiles).toEqual([]);
      expect(updated.stats.implFiles).toEqual([]);
    });
  });

  describe('Scenario: Remove test mapping removes implementation mappings too', () => {
    it('should remove test mapping and all its implementation mappings', async () => {
      // Given I have a test mapping with implementation mappings
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const coverageData = {
        scenarios: [
          {
            name: 'Login',
            testMappings: [
              {
                file: 'src/__tests__/auth.test.ts',
                lines: '10-20',
                implMappings: [
                  {
                    file: 'src/auth/login.ts',
                    lines: [5, 6, 7],
                  },
                  {
                    file: 'src/auth/validate.ts',
                    lines: [10, 11],
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
          testFiles: ['src/__tests__/auth.test.ts'],
          implFiles: ['src/auth/login.ts', 'src/auth/validate.ts'],
          totalLinesCovered: 16,
        },
      };

      const coverageFile = join(featuresDir, 'user-login.feature.coverage');
      await writeFile(coverageFile, JSON.stringify(coverageData, null, 2));

      // When I run 'fspec unlink-coverage user-login --scenario "Login" --test-file src/__tests__/auth.test.ts'
      const result = await unlinkCoverage('user-login', {
        scenario: 'Login',
        testFile: 'src/__tests__/auth.test.ts',
        cwd: testDir,
      });

      // Then the test mapping and all its impl mappings should be removed
      expect(result.success).toBe(true);

      const updatedContent = await readFile(coverageFile, 'utf-8');
      const updated = JSON.parse(updatedContent);
      expect(updated.scenarios[0].testMappings).toEqual([]);
      expect(updated.stats.coveredScenarios).toBe(0);
      expect(updated.stats.implFiles).toEqual([]);
    });
  });

  describe('Scenario: Remove only implementation mapping keeps test mapping', () => {
    it('should remove only implementation mapping and keep test mapping', async () => {
      // Given I have a test mapping with an implementation mapping
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const coverageData = {
        scenarios: [
          {
            name: 'Login',
            testMappings: [
              {
                file: 'src/__tests__/auth.test.ts',
                lines: '10-20',
                implMappings: [
                  {
                    file: 'src/auth/old.ts',
                    lines: [5, 6, 7],
                  },
                  {
                    file: 'src/auth/new.ts',
                    lines: [10, 11],
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
          testFiles: ['src/__tests__/auth.test.ts'],
          implFiles: ['src/auth/old.ts', 'src/auth/new.ts'],
          totalLinesCovered: 16,
        },
      };

      const coverageFile = join(featuresDir, 'user-login.feature.coverage');
      await writeFile(coverageFile, JSON.stringify(coverageData, null, 2));

      // When I run 'fspec unlink-coverage user-login --scenario "Login" --test-file src/__tests__/auth.test.ts --impl-file src/auth/old.ts'
      const result = await unlinkCoverage('user-login', {
        scenario: 'Login',
        testFile: 'src/__tests__/auth.test.ts',
        implFile: 'src/auth/old.ts',
        cwd: testDir,
      });

      // Then only the implementation mapping should be removed
      expect(result.success).toBe(true);

      const updatedContent = await readFile(coverageFile, 'utf-8');
      const updated = JSON.parse(updatedContent);

      // And the test mapping should still exist
      expect(updated.scenarios[0].testMappings).toHaveLength(1);
      expect(updated.scenarios[0].testMappings[0].file).toBe(
        'src/__tests__/auth.test.ts'
      );

      // But only one impl mapping remains
      expect(updated.scenarios[0].testMappings[0].implMappings).toHaveLength(1);
      expect(updated.scenarios[0].testMappings[0].implMappings[0].file).toBe(
        'src/auth/new.ts'
      );

      // Stats updated
      expect(updated.stats.implFiles).toEqual(['src/auth/new.ts']);
    });
  });
});
