/**
 * Feature: spec/features/refactor-link-coverage-command-to-reduce-file-size-and-complexity.feature
 *
 * Refactoring verification tests
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, rm, writeFile, readFile } from 'fs/promises';
import { existsSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { linkCoverage } from '../link-coverage';

describe('Feature: Refactor link-coverage command to reduce file size and complexity', () => {
  let testDir: string;
  let featureFile: string;
  let coverageFile: string;
  let testFile: string;

  beforeEach(async () => {
    testDir = join(tmpdir(), `fspec-refac-test-${Date.now()}`);
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec', 'features'), { recursive: true });

    featureFile = join(testDir, 'spec', 'features', 'test-refac.feature');
    coverageFile = join(
      testDir,
      'spec',
      'features',
      'test-refac.feature.coverage'
    );
    testFile = join(testDir, 'test-refac.test.ts');
  });

  afterEach(async () => {
    if (existsSync(testDir)) {
      await rm(testDir, { recursive: true, force: true });
    }
  });

  it('should verify link-coverage functionality after refactoring', async () => {
    // @step Given the link-coverage command has been refactored
    // (Implicit - we are running the current code)
    await writeFile(
      featureFile,
      'Feature: Test Refac\n\n  Scenario: Test Scenario\n    Given a step'
    );

    const coverageContent = {
      scenarios: [
        {
          name: 'Test Scenario',
          testMappings: [],
          implMappings: [],
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
    await writeFile(coverageFile, JSON.stringify(coverageContent, null, 2));
    await writeFile(testFile, '// @step Given a step\nconst x = 1;');

    // @step When I run "fspec link-coverage test.feature --scenario 'Test Scenario' --test-file test.ts --test-lines 1-10"
    const result = await linkCoverage('test-refac.feature', {
      scenario: 'Test Scenario',
      testFile: 'test-refac.test.ts',
      testLines: '1-10',
      skipValidation: true,
      cwd: testDir,
    });

    // @step Then the exit code should be 0
    expect(result.success).toBe(true);

    // @step And the output should contain "Linked test mapping"
    expect(result.message).toContain('Linked test mapping');

    // @step And the coverage file should be updated correctly
    const updatedCoverage = JSON.parse(await readFile(coverageFile, 'utf-8'));
    expect(updatedCoverage.scenarios[0].testMappings).toHaveLength(1);
    expect(updatedCoverage.scenarios[0].testMappings[0].file).toBe(
      'test-refac.test.ts'
    );
  });

  it('should verify file size reduction', async () => {
    // @step Given the link-coverage command has been refactored
    // (Implicit)

    // @step When I check the file size of "src/commands/link-coverage.ts"
    const content = await readFile(
      join(process.cwd(), 'src/commands/link-coverage.ts'),
      'utf-8'
    );
    const lineCount = content.split('\n').length;

    // @step Then the file size should be less than 300 lines
    // This test is expected to FAIL initially until refactoring is complete
    expect(lineCount).toBeLessThan(300);
  });
});
