/**
 * Feature: spec/features/audit-coverage-command.feature
 *
 * This test file validates the acceptance criteria for the audit-coverage command.
 * Tests that coverage files are audited for missing files and actionable recommendations are provided.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { join } from 'path';
import { auditCoverage } from '../audit-coverage';
import {
  setupTestDirectory,
  type TestDirectorySetup,
} from '../../test-helpers/universal-test-setup';
import {
  writeJsonTestFile,
  ensureTestDirectory,
  createTestFile,
} from '../../test-helpers/test-file-operations';

describe('Feature: Audit Coverage Command', () => {
  let setup: TestDirectorySetup;
  let specDir: string;
  let featuresDir: string;
  let srcDir: string;
  let testsDir: string;

  beforeEach(async () => {
    setup = await setupTestDirectory('audit-coverage');
    specDir = join(setup.testDir, 'spec');
    featuresDir = join(specDir, 'features');
    srcDir = join(setup.testDir, 'src');
    testsDir = join(srcDir, '__tests__');

    // Create directory structure
    await ensureTestDirectory(featuresDir);
    await ensureTestDirectory(testsDir);
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: Audit coverage with all files present', () => {
    it('should display success message when all files exist', async () => {
      // Given I have a coverage file "user-login.feature.coverage"
      const coverageData = {
        scenarios: [
          {
            name: 'Login with valid credentials',
            testMappings: [
              {
                file: 'src/__tests__/login.test.ts',
                lines: '10-20',
                implMappings: [
                  {
                    file: 'src/auth/login.ts',
                    lines: [25, 30],
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
          testFiles: ['src/__tests__/login.test.ts'],
          implFiles: ['src/auth/login.ts'],
          totalLinesCovered: 5,
        },
      };

      await writeJsonTestFile(
        join(featuresDir, 'user-login.feature.coverage'),
        coverageData
      );

      // And all test files referenced in coverage exist
      await createTestFile(testsDir, 'login.test.ts', '// test content');

      // And all implementation files referenced in coverage exist
      await ensureTestDirectory(join(srcDir, 'auth'));
      await createTestFile(join(srcDir, 'auth'), 'login.ts', '// impl content');

      // When I run `fspec audit-coverage user-login`
      const result = await auditCoverage({
        featureName: 'user-login',
        cwd: setup.testDir,
      });

      // Then the command should display "✅ All files found (2/2)"
      expect(result.output).toContain('✅ All files found (2/2)');

      // And the output should show "All mappings valid"
      expect(result.output).toContain('All mappings valid');

      // And the command should exit with code 0
      expect(result.exitCode).toBe(0);
    });
  });

  describe('Scenario: Audit detects missing test file', () => {
    it('should display error message when test file is missing', async () => {
      // Given I have a coverage file with mapping to "src/__tests__/deleted.test.ts"
      const coverageData = {
        scenarios: [
          {
            name: 'Login with valid credentials',
            testMappings: [
              {
                file: 'src/__tests__/deleted.test.ts',
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
          testFiles: ['src/__tests__/deleted.test.ts'],
          implFiles: [],
          totalLinesCovered: 3,
        },
      };

      await writeJsonTestFile(
        join(featuresDir, 'user-login.feature.coverage'),
        coverageData
      );

      // And the test file "src/__tests__/deleted.test.ts" does not exist
      // (file not created, so it doesn't exist)

      // When I run `fspec audit-coverage user-login`
      const result = await auditCoverage({
        featureName: 'user-login',
        cwd: setup.testDir,
      });

      // Then the output should display "❌ Test file not found: src/__tests__/deleted.test.ts"
      expect(result.output).toContain(
        '❌ Test file not found: src/__tests__/deleted.test.ts'
      );

      // And the output should show recommendation "Remove this mapping or restore the deleted file"
      expect(result.output).toContain(
        'Remove this mapping or restore the deleted file'
      );

      // And the command should exit with code 1
      expect(result.exitCode).toBe(1);
    });
  });

  describe('Scenario: Audit detects missing implementation file', () => {
    it('should display error message when implementation file is missing', async () => {
      // Given I have a coverage file with mapping to "src/auth/deleted.ts"
      const coverageData = {
        scenarios: [
          {
            name: 'Login with valid credentials',
            testMappings: [
              {
                file: 'src/__tests__/placeholder.test.ts',
                lines: '1-5',
                implMappings: [
                  {
                    file: 'src/auth/deleted.ts',
                    lines: [25, 30],
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
          testFiles: ['src/__tests__/placeholder.test.ts'],
          implFiles: ['src/auth/deleted.ts'],
          totalLinesCovered: 2,
        },
      };

      await writeJsonTestFile(
        join(featuresDir, 'user-login.feature.coverage'),
        coverageData
      );

      // And the test file exists but implementation file doesn't
      await createTestFile(testsDir, 'placeholder.test.ts', '// placeholder test');
      // (implementation file "src/auth/deleted.ts" not created, so it doesn't exist)

      // When I run `fspec audit-coverage user-login`
      const result = await auditCoverage({
        featureName: 'user-login',
        cwd: setup.testDir,
      });

      // Then the output should display "❌ Implementation file not found: src/auth/deleted.ts"
      expect(result.output).toContain(
        '❌ Implementation file not found: src/auth/deleted.ts'
      );

      // And the output should show actionable recommendation
      expect(result.output).toMatch(
        /Remove this mapping|restore the deleted file/i
      );

      // And the command should exit with code 1
      expect(result.exitCode).toBe(1);
    });
  });
});
