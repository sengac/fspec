/**
 * Feature: spec/features/link-coverage-crashes-when-coverage-file-stats-are-missing.feature
 *
 * Reproduction test for BUG-091
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, rm, writeFile, readFile } from 'fs/promises';
import { existsSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { linkCoverage } from '../link-coverage';

describe('Feature: link-coverage crashes when coverage file stats are missing', () => {
  let testDir: string;
  let featureFile: string;
  let coverageFile: string;
  let testFile: string;

  beforeEach(async () => {
    testDir = join(tmpdir(), `fspec-bug-repro-${Date.now()}`);
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec', 'features'), { recursive: true });

    featureFile = join(
      testDir,
      'spec',
      'features',
      'test-missing-stats.feature'
    );
    coverageFile = join(
      testDir,
      'spec',
      'features',
      'test-missing-stats.feature.coverage'
    );
    testFile = join(testDir, 'test-missing-stats.test.ts');
  });

  afterEach(async () => {
    if (existsSync(testDir)) {
      await rm(testDir, { recursive: true, force: true });
    }
  });

  it('should handle missing stats object gracefully', async () => {
    // @step Given I have a feature file "test-missing-stats.feature"
    await writeFile(
      featureFile,
      'Feature: Test Missing Stats\n\n  Scenario: Test Scenario\n    Given a step'
    );

    // @step And I have a coverage file "test-missing-stats.feature.coverage" without a stats object
    // Intentionally omitting "stats" object to reproduce the bug
    const coverageContent = {
      scenarios: [
        {
          name: 'Test Scenario',
          testMappings: [],
          implMappings: [],
        },
      ],
    };
    await writeFile(coverageFile, JSON.stringify(coverageContent, null, 2));

    // @step And I have a test file "test-missing-stats.test.ts"
    await writeFile(testFile, '// @step Given a step\nconst x = 1;');

    // @step When I run "fspec link-coverage test-missing-stats.feature --scenario 'Test Scenario' --test-file test-missing-stats.test.ts --test-lines 1-10"
    const result = await linkCoverage('test-missing-stats.feature', {
      scenario: 'Test Scenario',
      testFile: 'test-missing-stats.test.ts',
      testLines: '1-10',
      skipValidation: true, // Skip file existence checks since we're in tmp dir
      cwd: testDir,
    });

    // @step Then the exit code should be 0
    // (implied by promise resolution without throw)
    expect(result.success).toBe(true);

    // @step And the output should contain "Linked test mapping"
    expect(result.message).toContain('Linked test mapping');

    // @step And the coverage file should contain a valid stats object
    const updatedCoverage = JSON.parse(await readFile(coverageFile, 'utf-8'));
    expect(updatedCoverage.stats).toBeDefined();
    expect(updatedCoverage.stats.totalScenarios).toBeDefined();
  });
});
