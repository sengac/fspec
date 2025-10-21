/**
 * Feature: spec/features/link-coverage-command.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, readFile, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import type { CoverageFile } from '../../utils/coverage-file';

describe('Feature: Link Coverage Command', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-link-coverage-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Link test mapping only', () => {
    it('should create test mapping and display success message', async () => {
      // Given I have a coverage file with an uncovered scenario
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const coverageData: CoverageFile = {
        scenarios: [
          {
            name: 'Login with valid credentials',
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

      const coverageFile = join(featuresDir, 'user-login.feature.coverage');
      await writeFile(coverageFile, JSON.stringify(coverageData, null, 2));

      // Create the test file that we'll link to
      const testFile = join(testDir, 'src', '__tests__', 'auth.test.ts');
      await mkdir(join(testDir, 'src', '__tests__'), { recursive: true });
      await writeFile(testFile, '// test file');

      // When I run link-coverage with test-only flags
      const { linkCoverage } = await import('../link-coverage');
      const result = await linkCoverage('user-login', {
        scenario: 'Login with valid credentials',
        testFile: 'src/__tests__/auth.test.ts',
        testLines: '45-62',
        cwd: testDir,
      });

      // Then the test mapping should be created
      const updatedContent = await readFile(coverageFile, 'utf-8');
      const updatedCoverage: CoverageFile = JSON.parse(updatedContent);

      expect(updatedCoverage.scenarios[0].testMappings).toHaveLength(1);
      expect(updatedCoverage.scenarios[0].testMappings[0]).toEqual({
        file: 'src/__tests__/auth.test.ts',
        lines: '45-62',
        implMappings: [],
      });

      // And should display success message
      expect(result.message).toContain('Linked test mapping');
      expect(result.message).toContain('src/__tests__/auth.test.ts');
    });
  });

  describe('Scenario: Link implementation to existing test', () => {
    it('should add impl mapping to existing test mapping', async () => {
      // Given I have a test mapping without implementation
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
                implMappings: [],
              },
            ],
          },
        ],
        stats: {
          totalScenarios: 1,
          coveredScenarios: 1,
          coveragePercent: 100,
          testFiles: ['src/__tests__/auth.test.ts'],
          implFiles: [],
          totalLinesCovered: 18,
        },
      };

      const coverageFile = join(featuresDir, 'user-login.feature.coverage');
      await writeFile(coverageFile, JSON.stringify(coverageData, null, 2));

      // Create the test file (required for validation)
      const testFile = join(testDir, 'src', '__tests__', 'auth.test.ts');
      await mkdir(join(testDir, 'src', '__tests__'), { recursive: true });
      await writeFile(testFile, '// test file');

      // Create the impl file
      const implFile = join(testDir, 'src', 'auth', 'login.ts');
      await mkdir(join(testDir, 'src', 'auth'), { recursive: true });
      await writeFile(implFile, '// impl file');

      // When I run link-coverage with impl flags
      const { linkCoverage } = await import('../link-coverage');
      const result = await linkCoverage('user-login', {
        scenario: 'Login with valid credentials',
        testFile: 'src/__tests__/auth.test.ts',
        implFile: 'src/auth/login.ts',
        implLines: '10,11,12',
        cwd: testDir,
      });

      // Then the impl mapping should be added
      const updatedContent = await readFile(coverageFile, 'utf-8');
      const updatedCoverage: CoverageFile = JSON.parse(updatedContent);

      expect(
        updatedCoverage.scenarios[0].testMappings[0].implMappings
      ).toHaveLength(1);
      expect(
        updatedCoverage.scenarios[0].testMappings[0].implMappings[0]
      ).toEqual({
        file: 'src/auth/login.ts',
        lines: [10, 11, 12],
      });

      // And should display success message
      expect(result.message).toContain('Added implementation mapping');
    });
  });

  describe('Scenario: Link both test and impl at once', () => {
    it('should create test mapping with impl mapping in one operation', async () => {
      // Given I have a coverage file with an uncovered scenario
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const coverageData: CoverageFile = {
        scenarios: [
          {
            name: 'Login with valid credentials',
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

      const coverageFile = join(featuresDir, 'user-login.feature.coverage');
      await writeFile(coverageFile, JSON.stringify(coverageData, null, 2));

      // Create both files
      const testFile = join(testDir, 'src', '__tests__', 'auth.test.ts');
      await mkdir(join(testDir, 'src', '__tests__'), { recursive: true });
      await writeFile(testFile, '// test file');

      const implFile = join(testDir, 'src', 'auth', 'login.ts');
      await mkdir(join(testDir, 'src', 'auth'), { recursive: true });
      await writeFile(implFile, '// impl file');

      // When I run link-coverage with all flags
      const { linkCoverage } = await import('../link-coverage');
      const result = await linkCoverage('user-login', {
        scenario: 'Login with valid credentials',
        testFile: 'src/__tests__/auth.test.ts',
        testLines: '45-62',
        implFile: 'src/auth/login.ts',
        implLines: '10,11,12',
        cwd: testDir,
      });

      // Then both mappings should be created
      const updatedContent = await readFile(coverageFile, 'utf-8');
      const updatedCoverage: CoverageFile = JSON.parse(updatedContent);

      expect(updatedCoverage.scenarios[0].testMappings).toHaveLength(1);
      expect(updatedCoverage.scenarios[0].testMappings[0]).toEqual({
        file: 'src/__tests__/auth.test.ts',
        lines: '45-62',
        implMappings: [
          {
            file: 'src/auth/login.ts',
            lines: [10, 11, 12],
          },
        ],
      });

      // And should display success message
      expect(result.message).toContain(
        'Linked test mapping with implementation'
      );
    });
  });

  describe('Scenario: File validation failure', () => {
    it('should error when test file does not exist', async () => {
      // Given I have a coverage file
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const coverageData: CoverageFile = {
        scenarios: [
          {
            name: 'Login with valid credentials',
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

      const coverageFile = join(featuresDir, 'user-login.feature.coverage');
      await writeFile(coverageFile, JSON.stringify(coverageData, null, 2));

      // When I run link-coverage with non-existent file (no --skip-validation)
      const { linkCoverage } = await import('../link-coverage');

      // Then should throw error
      await expect(
        linkCoverage('user-login', {
          scenario: 'Login with valid credentials',
          testFile: 'src/__tests__/missing.test.ts',
          testLines: '1-10',
          cwd: testDir,
        })
      ).rejects.toThrow('File not found');

      // And should suggest --skip-validation
      try {
        await linkCoverage('user-login', {
          scenario: 'Login with valid credentials',
          testFile: 'src/__tests__/missing.test.ts',
          testLines: '1-10',
          cwd: testDir,
        });
      } catch (error: any) {
        expect(error.message).toContain('--skip-validation');
      }
    });

    it('should succeed with warning when --skip-validation is used', async () => {
      // Given I have a coverage file
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const coverageData: CoverageFile = {
        scenarios: [
          {
            name: 'Login with valid credentials',
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

      const coverageFile = join(featuresDir, 'user-login.feature.coverage');
      await writeFile(coverageFile, JSON.stringify(coverageData, null, 2));

      // When I run with --skip-validation
      const { linkCoverage } = await import('../link-coverage');
      const result = await linkCoverage('user-login', {
        scenario: 'Login with valid credentials',
        testFile: 'src/__tests__/future.test.ts',
        testLines: '1-10',
        skipValidation: true,
        cwd: testDir,
      });

      // Then should succeed
      expect(result.success).toBe(true);

      // And should show warning
      expect(result.warnings).toBeDefined();
      expect(result.warnings).toContain('File not found');
      expect(result.warnings).toContain('src/__tests__/future.test.ts');
    });
  });

  describe('Scenario: Smart append for impl files', () => {
    it('should update existing impl file instead of creating duplicate', async () => {
      // Given I have a test mapping with existing impl file
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
                    lines: [10, 11, 12],
                  },
                  {
                    file: 'src/auth/session.ts',
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
          testFiles: ['src/__tests__/auth.test.ts'],
          implFiles: ['src/auth/login.ts', 'src/auth/session.ts'],
          totalLinesCovered: 23,
        },
      };

      const coverageFile = join(featuresDir, 'user-login.feature.coverage');
      await writeFile(coverageFile, JSON.stringify(coverageData, null, 2));

      // Create the test file (required for validation)
      const testFile = join(testDir, 'src', '__tests__', 'auth.test.ts');
      await mkdir(join(testDir, 'src', '__tests__'), { recursive: true });
      await writeFile(testFile, '// test file');

      // Create the impl file
      const implFile = join(testDir, 'src', 'auth', 'login.ts');
      await mkdir(join(testDir, 'src', 'auth'), { recursive: true });
      await writeFile(implFile, '// impl file');

      // When I add the same impl file with different lines
      const { linkCoverage } = await import('../link-coverage');
      const result = await linkCoverage('user-login', {
        scenario: 'Login with valid credentials',
        testFile: 'src/__tests__/auth.test.ts',
        implFile: 'src/auth/login.ts',
        implLines: '20,21,22',
        cwd: testDir,
      });

      // Then should update existing entry, not add duplicate
      const updatedContent = await readFile(coverageFile, 'utf-8');
      const updatedCoverage: CoverageFile = JSON.parse(updatedContent);

      expect(
        updatedCoverage.scenarios[0].testMappings[0].implMappings
      ).toHaveLength(2);

      // Find the login.ts entry
      const loginEntry =
        updatedCoverage.scenarios[0].testMappings[0].implMappings.find(
          m => m.file === 'src/auth/login.ts'
        );

      expect(loginEntry).toBeDefined();
      expect(loginEntry!.lines).toEqual([20, 21, 22]);

      // And should display update message
      expect(result.message).toContain('Updated implementation mapping');
    });
  });

  describe('Scenario: Append test mappings', () => {
    it('should allow multiple test mappings for same file', async () => {
      // Given I have a test mapping for a file
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
                implMappings: [],
              },
            ],
          },
        ],
        stats: {
          totalScenarios: 1,
          coveredScenarios: 1,
          coveragePercent: 100,
          testFiles: ['src/__tests__/auth.test.ts'],
          implFiles: [],
          totalLinesCovered: 18,
        },
      };

      const coverageFile = join(featuresDir, 'user-login.feature.coverage');
      await writeFile(coverageFile, JSON.stringify(coverageData, null, 2));

      // Create the test file
      const testFile = join(testDir, 'src', '__tests__', 'auth.test.ts');
      await mkdir(join(testDir, 'src', '__tests__'), { recursive: true });
      await writeFile(testFile, '// test file');

      // When I add another mapping with same file but different lines
      const { linkCoverage } = await import('../link-coverage');
      const result = await linkCoverage('user-login', {
        scenario: 'Login with valid credentials',
        testFile: 'src/__tests__/auth.test.ts',
        testLines: '70-85',
        cwd: testDir,
      });

      // Then should append as second mapping
      const updatedContent = await readFile(coverageFile, 'utf-8');
      const updatedCoverage: CoverageFile = JSON.parse(updatedContent);

      expect(updatedCoverage.scenarios[0].testMappings).toHaveLength(2);
      expect(updatedCoverage.scenarios[0].testMappings[0].lines).toBe('45-62');
      expect(updatedCoverage.scenarios[0].testMappings[1].lines).toBe('70-85');

      // And should display append message
      expect(result.message).toContain('Added second test mapping');
    });
  });

  describe('Edge Cases', () => {
    it('should parse impl line ranges (10-15) into array', async () => {
      // Given I have a coverage file
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
                implMappings: [],
              },
            ],
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

      const coverageFile = join(featuresDir, 'test.feature.coverage');
      await writeFile(coverageFile, JSON.stringify(coverageData, null, 2));

      // Create the test file (required for validation)
      const testFile = join(testDir, 'src', '__tests__', 'test.test.ts');
      await mkdir(join(testDir, 'src', '__tests__'), { recursive: true });
      await writeFile(testFile, '// test file');

      const implFile = join(testDir, 'src', 'test.ts');
      await mkdir(join(testDir, 'src'), { recursive: true });
      await writeFile(implFile, '// impl');

      // When I provide impl lines as range
      const { linkCoverage } = await import('../link-coverage');
      await linkCoverage('test', {
        scenario: 'Test scenario',
        testFile: 'src/__tests__/test.test.ts',
        implFile: 'src/test.ts',
        implLines: '10-15',
        cwd: testDir,
      });

      // Then should expand to array
      const updatedContent = await readFile(coverageFile, 'utf-8');
      const updatedCoverage: CoverageFile = JSON.parse(updatedContent);

      expect(
        updatedCoverage.scenarios[0].testMappings[0].implMappings[0].lines
      ).toEqual([10, 11, 12, 13, 14, 15]);
    });

    it('should require test-file when adding impl-only', async () => {
      // Given I have a coverage file
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const coverageData: CoverageFile = {
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

      const coverageFile = join(featuresDir, 'test.feature.coverage');
      await writeFile(coverageFile, JSON.stringify(coverageData, null, 2));

      // When I try to add impl without test-file
      const { linkCoverage } = await import('../link-coverage');

      // Then should error
      await expect(
        linkCoverage('test', {
          scenario: 'Test scenario',
          implFile: 'src/test.ts',
          implLines: '10,11',
          cwd: testDir,
        })
      ).rejects.toThrow('--test-file is required');
    });
  });

  describe('Feature: System-reminder for missing scenarios in link-coverage (UX-001)', () => {
    describe('Scenario: Scenario exists in feature file but not in coverage file', () => {
      it('should emit system-reminder suggesting generate-coverage', async () => {
        // Given: I have a feature file with a scenario "Login with valid credentials"
        const featuresDir = join(testDir, 'spec', 'features');
        await mkdir(featuresDir, { recursive: true });

        const featureContent = `Feature: User Login
  Scenario: Login with valid credentials
    Given I am on the login page
    When I enter valid credentials
    Then I should be logged in`;

        await writeFile(
          join(featuresDir, 'user-login.feature'),
          featureContent
        );

        // And: the coverage file exists but does not contain that scenario
        const coverageData: CoverageFile = {
          scenarios: [
            {
              name: 'Different scenario',
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

        const coverageFile = join(featuresDir, 'user-login.feature.coverage');
        await writeFile(coverageFile, JSON.stringify(coverageData, null, 2));

        // When: I run "fspec link-coverage" for that scenario
        const { linkCoverage } = await import('../link-coverage');

        // Then: the command should fail with "Scenario not found" error
        await expect(
          linkCoverage('user-login', {
            scenario: 'Login with valid credentials',
            testFile: 'src/__tests__/auth.test.ts',
            testLines: '10-20',
            skipValidation: true,
            cwd: testDir,
          })
        ).rejects.toThrow('Scenario not found');

        // And: the output should contain a system-reminder wrapped in <system-reminder> tags
        try {
          await linkCoverage('user-login', {
            scenario: 'Login with valid credentials',
            testFile: 'src/__tests__/auth.test.ts',
            testLines: '10-20',
            skipValidation: true,
            cwd: testDir,
          });
        } catch (error: any) {
          expect(error.message).toContain('<system-reminder>');
          expect(error.message).toContain('</system-reminder>');

          // And: the system-reminder should suggest running "fspec generate-coverage" first
          expect(error.message).toContain('fspec generate-coverage');
          expect(error.message).toContain('coverage file is out of sync');
        }
      });
    });

    describe("Scenario: Scenario doesn't exist in feature file (typo)", () => {
      it('should show normal error without system-reminder', async () => {
        // Given: I have a feature file with scenarios
        const featuresDir = join(testDir, 'spec', 'features');
        await mkdir(featuresDir, { recursive: true });

        const featureContent = `Feature: User Login
  Scenario: Login with valid credentials
    Given I am on the login page
    When I enter valid credentials
    Then I should be logged in`;

        await writeFile(
          join(featuresDir, 'user-login.feature'),
          featureContent
        );

        // And: the coverage file exists
        const coverageData: CoverageFile = {
          scenarios: [
            {
              name: 'Login with valid credentials',
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

        const coverageFile = join(featuresDir, 'user-login.feature.coverage');
        await writeFile(coverageFile, JSON.stringify(coverageData, null, 2));

        // When: I run "fspec link-coverage" with a scenario name that doesn't exist in the feature file
        const { linkCoverage } = await import('../link-coverage');

        // Then: the command should fail with "Scenario not found" error
        await expect(
          linkCoverage('user-login', {
            scenario: 'Login with INVALID credentials', // Typo - doesn't exist
            testFile: 'src/__tests__/auth.test.ts',
            testLines: '10-20',
            skipValidation: true,
            cwd: testDir,
          })
        ).rejects.toThrow('Scenario not found');

        // And: the output should list available scenarios
        try {
          await linkCoverage('user-login', {
            scenario: 'Login with INVALID credentials',
            testFile: 'src/__tests__/auth.test.ts',
            testLines: '10-20',
            skipValidation: true,
            cwd: testDir,
          });
        } catch (error: any) {
          expect(error.message).toContain('Available scenarios');
          expect(error.message).toContain('Login with valid credentials');

          // And: the output should NOT contain a system-reminder
          expect(error.message).not.toContain('<system-reminder>');
        }
      });
    });

    describe('Scenario: Coverage file missing entirely', () => {
      it('should emit system-reminder suggesting generate-coverage', async () => {
        // Given: I have a feature file with a scenario "Login with valid credentials"
        const featuresDir = join(testDir, 'spec', 'features');
        await mkdir(featuresDir, { recursive: true });

        const featureContent = `Feature: User Login
  Scenario: Login with valid credentials
    Given I am on the login page
    When I enter valid credentials
    Then I should be logged in`;

        await writeFile(
          join(featuresDir, 'user-login.feature'),
          featureContent
        );

        // And: the coverage file does not exist

        // When: I run "fspec link-coverage" for that scenario
        const { linkCoverage } = await import('../link-coverage');

        // Then: the command should fail with an error
        await expect(
          linkCoverage('user-login', {
            scenario: 'Login with valid credentials',
            testFile: 'src/__tests__/auth.test.ts',
            testLines: '10-20',
            skipValidation: true,
            cwd: testDir,
          })
        ).rejects.toThrow();

        // And: the output should contain a system-reminder suggesting "fspec generate-coverage"
        try {
          await linkCoverage('user-login', {
            scenario: 'Login with valid credentials',
            testFile: 'src/__tests__/auth.test.ts',
            testLines: '10-20',
            skipValidation: true,
            cwd: testDir,
          });
        } catch (error: any) {
          expect(error.message).toContain('<system-reminder>');
          expect(error.message).toContain('</system-reminder>');
          expect(error.message).toContain('fspec generate-coverage');
        }
      });
    });
  });
});
