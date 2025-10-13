/**
 * Feature: spec/features/show-coverage-statistics.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, readFile, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import type { CoverageFile } from '../../utils/coverage-file';

describe('Feature: Show Coverage Statistics', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-show-coverage-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Display markdown report for single file with full breakdown', () => {
    it('should display markdown with 80% coverage, symbols, and coverage gaps section', async () => {
      // Given a feature file with coverage data (4/5 scenarios covered)
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const coverageData: CoverageFile = {
        scenarios: [
          {
            name: 'Login with valid credentials',
            testMappings: [
              {
                file: 'src/__tests__/auth.test.ts',
                lines: '45-62',
                implMappings: [
                  {
                    file: 'src/auth/login.ts',
                    lines: [10, 11, 12, 15, 20],
                  },
                ],
              },
            ],
          },
          {
            name: 'Login with invalid credentials',
            testMappings: [
              {
                file: 'src/__tests__/auth.test.ts',
                lines: '64-80',
                implMappings: [
                  {
                    file: 'src/auth/login.ts',
                    lines: [22, 23, 24, 25],
                  },
                ],
              },
            ],
          },
          {
            name: 'Remember me functionality',
            testMappings: [
              {
                file: 'src/__tests__/auth.test.ts',
                lines: '82-95',
                implMappings: [], // Partially covered
              },
            ],
          },
          {
            name: 'Session timeout',
            testMappings: [
              {
                file: 'src/__tests__/auth.test.ts',
                lines: '97-110',
                implMappings: [
                  {
                    file: 'src/auth/session.ts',
                    lines: [5, 6, 7],
                  },
                ],
              },
            ],
          },
          {
            name: 'Password reset flow',
            testMappings: [], // Uncovered
          },
        ],
        stats: {
          totalScenarios: 5,
          coveredScenarios: 4,
          coveragePercent: 80,
          testFiles: ['src/__tests__/auth.test.ts'],
          implFiles: ['src/auth/login.ts', 'src/auth/session.ts'],
          totalLinesCovered: 80,
        },
      };

      const coverageFile = join(featuresDir, 'user-login.feature.coverage');
      await writeFile(coverageFile, JSON.stringify(coverageData, null, 2));

      // When I run 'fspec show-coverage user-login.feature'
      const { showCoverage } = await import('../show-coverage');
      const output = await showCoverage('user-login.feature', {
        format: 'markdown',
        cwd: testDir,
      });

      // Then the markdown should contain coverage percentage
      expect(output).toContain('80%');
      expect(output).toContain('4/5');

      // And should have ✅ symbols for fully covered scenarios
      expect(output).toContain('✅');
      expect(output).toContain('Login with valid credentials');

      // And should have ⚠️ symbols for partially covered scenarios
      expect(output).toContain('⚠️');
      expect(output).toContain('Remember me functionality');

      // And should have ❌ symbols for uncovered scenarios
      expect(output).toContain('❌');
      expect(output).toContain('Password reset flow');

      // And should have a Coverage Gaps section
      expect(output).toContain('Coverage Gaps');
      expect(output).toContain('Password reset flow');
    });
  });

  describe('Scenario: Display JSON output for single file', () => {
    it('should output JSON with scenarios, stats, and three-tier coverage', async () => {
      // Given a feature file with coverage data
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const coverageData: CoverageFile = {
        scenarios: [
          {
            name: 'Test scenario',
            testMappings: [
              {
                file: 'src/__tests__/test.test.ts',
                lines: '10-20',
                implMappings: [
                  {
                    file: 'src/test.ts',
                    lines: [5, 6],
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
          totalLinesCovered: 13,
        },
      };

      const coverageFile = join(featuresDir, 'test.feature.coverage');
      await writeFile(coverageFile, JSON.stringify(coverageData, null, 2));

      // When I run 'fspec show-coverage test.feature --format=json'
      const { showCoverage } = await import('../show-coverage');
      const output = await showCoverage('test.feature', {
        format: 'json',
        cwd: testDir,
      });

      // Then the output should be valid JSON
      const json = JSON.parse(output);

      // And should have scenarios array
      expect(json.scenarios).toBeDefined();
      expect(json.scenarios).toHaveLength(1);

      // And should have stats object
      expect(json.stats).toBeDefined();
      expect(json.stats.coveragePercent).toBe(100);

      // And should have coverage status for each scenario
      expect(json.scenarios[0].coverageStatus).toBeDefined();
      expect(json.scenarios[0].coverageStatus).toBe('fully-covered');
    });
  });

  describe('Scenario: Display project-wide coverage for all files', () => {
    it('should show aggregated summary and per-feature breakdown', async () => {
      // Given multiple feature files with coverage data
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      // Feature 1: user-login.feature (80%)
      const coverage1: CoverageFile = {
        scenarios: [
          {
            name: 'Scenario 1',
            testMappings: [
              {
                file: 'src/__tests__/test1.test.ts',
                lines: '10-20',
                implMappings: [{ file: 'src/test1.ts', lines: [5] }],
              },
            ],
          },
          { name: 'Scenario 2', testMappings: [] },
        ],
        stats: {
          totalScenarios: 2,
          coveredScenarios: 1,
          coveragePercent: 50,
          testFiles: ['src/__tests__/test1.test.ts'],
          implFiles: ['src/test1.ts'],
          totalLinesCovered: 12,
        },
      };

      // Feature 2: user-registration.feature (100%)
      const coverage2: CoverageFile = {
        scenarios: [
          {
            name: 'Scenario 1',
            testMappings: [
              {
                file: 'src/__tests__/test2.test.ts',
                lines: '5-15',
                implMappings: [{ file: 'src/test2.ts', lines: [3, 4] }],
              },
            ],
          },
        ],
        stats: {
          totalScenarios: 1,
          coveredScenarios: 1,
          coveragePercent: 100,
          testFiles: ['src/__tests__/test2.test.ts'],
          implFiles: ['src/test2.ts'],
          totalLinesCovered: 13,
        },
      };

      await writeFile(
        join(featuresDir, 'user-login.feature.coverage'),
        JSON.stringify(coverage1, null, 2)
      );
      await writeFile(
        join(featuresDir, 'user-registration.feature.coverage'),
        JSON.stringify(coverage2, null, 2)
      );

      // When I run 'fspec show-coverage' (no args)
      const { showCoverage } = await import('../show-coverage');
      const output = await showCoverage(undefined, {
        format: 'markdown',
        cwd: testDir,
      });

      // Then should show project summary
      expect(output).toContain('Project Coverage');
      expect(output).toContain('Total Features: 2');
      expect(output).toContain('Total Scenarios: 3');

      // And should show per-feature breakdown
      expect(output).toContain('user-login.feature');
      expect(output).toContain('50%');
      expect(output).toContain('user-registration.feature');
      expect(output).toContain('100%');
    });
  });

  describe('Scenario: Handle missing coverage file', () => {
    it('should error with exit code 1 and suggestion message', async () => {
      // Given no coverage file exists
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      // When I run 'fspec show-coverage missing.feature'
      const { showCoverage } = await import('../show-coverage');

      // Then should throw error
      await expect(
        showCoverage('missing.feature', { format: 'markdown', cwd: testDir })
      ).rejects.toThrow('Coverage file not found');

      // And error message should suggest creating coverage file
      try {
        await showCoverage('missing.feature', {
          format: 'markdown',
          cwd: testDir,
        });
      } catch (error: any) {
        expect(error.message).toContain('fspec create-feature');
      }
    });
  });

  describe('Scenario: Handle invalid JSON in coverage file', () => {
    it('should error with parse details and suggestion', async () => {
      // Given a coverage file with invalid JSON
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const coverageFile = join(featuresDir, 'invalid.feature.coverage');
      await writeFile(coverageFile, '{ invalid json here }');

      // When I run 'fspec show-coverage invalid.feature'
      const { showCoverage } = await import('../show-coverage');

      // Then should throw error with parse details
      await expect(
        showCoverage('invalid.feature', { format: 'markdown', cwd: testDir })
      ).rejects.toThrow('Invalid JSON');

      // And error message should mention recreation
      try {
        await showCoverage('invalid.feature', {
          format: 'markdown',
          cwd: testDir,
        });
      } catch (error: any) {
        expect(error.message).toMatch(/recreat/i);
      }
    });
  });

  describe('Scenario: Validate file paths and show warnings', () => {
    it('should warn about missing files but still display coverage', async () => {
      // Given coverage data with non-existent file paths
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const coverageData: CoverageFile = {
        scenarios: [
          {
            name: 'Test scenario',
            testMappings: [
              {
                file: 'src/__tests__/deleted.test.ts', // This file doesn't exist
                lines: '10-20',
                implMappings: [
                  {
                    file: 'src/deleted.ts', // This file doesn't exist
                    lines: [5, 6],
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
          testFiles: ['src/__tests__/deleted.test.ts'],
          implFiles: ['src/deleted.ts'],
          totalLinesCovered: 13,
        },
      };

      const coverageFile = join(featuresDir, 'test.feature.coverage');
      await writeFile(coverageFile, JSON.stringify(coverageData, null, 2));

      // When I run 'fspec show-coverage test.feature'
      const { showCoverage } = await import('../show-coverage');
      const output = await showCoverage('test.feature', {
        format: 'markdown',
        cwd: testDir,
      });

      // Then should show warning about missing files
      expect(output).toContain('⚠️');
      expect(output).toContain('File not found');
      expect(output).toContain('src/__tests__/deleted.test.ts');

      // And should still display the coverage data
      expect(output).toContain('Test scenario');
      expect(output).toContain('100%');
    });
  });
});
