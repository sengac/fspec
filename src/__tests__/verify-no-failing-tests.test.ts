/**
 * Meta-test to verify no tests are failing in the entire test suite
 */

import { describe, it, expect } from 'vitest';
import { execSync } from 'child_process';

describe('Meta: Verify no failing tests', () => {
  it('should have zero failing tests in the entire test suite', () => {
    // Run npm test and capture output
    let exitCode = 0;
    let output = '';

    try {
      output = execSync('npm test', {
        encoding: 'utf8',
        cwd: process.cwd(),
        stdio: 'pipe',
      });
    } catch (error: any) {
      exitCode = error.status || 1;
      output = error.stdout + error.stderr;
    }

    // Parse vitest output for test results
    const failedMatch = output.match(/Test Files\s+(\d+)\s+failed/);
    const failedTests = failedMatch ? parseInt(failedMatch[1]) : 0;

    // Expect zero failed tests
    expect(failedTests).toBe(0);
    expect(exitCode).toBe(0);
  }, 300000); // 5 minute timeout for full test suite
});
