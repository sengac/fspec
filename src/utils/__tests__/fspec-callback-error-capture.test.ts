/**
 * Test fspec-callback with the actual Commander.js integration
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { fspecCallback } from '../fspec-callback';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

describe('fspec-callback error capture', () => {
  let testDir: string;

  beforeAll(() => {
    // Create a real temp directory with fspec structure but no work units
    testDir = fs.mkdtempSync(path.join(os.tmpdir(), 'fspec-error-capture-'));
    const specDir = path.join(testDir, 'spec');
    fs.mkdirSync(specDir, { recursive: true });

    // Create empty work-units.json
    const workUnitsData = {
      workUnits: {},
      states: {
        backlog: [],
        specifying: [],
        testing: [],
        implementing: [],
        validating: [],
        done: [],
        blocked: [],
      },
    };

    fs.writeFileSync(
      path.join(specDir, 'work-units.json'),
      JSON.stringify(workUnitsData, null, 2)
    );
  });

  afterAll(() => {
    if (testDir && fs.existsSync(testDir)) {
      fs.rmSync(testDir, { recursive: true, force: true });
    }
  });

  it('should capture command error about non-existent work unit', async () => {
    // The add-rule command validates that the work unit exists before proceeding
    // This test verifies that we capture meaningful error messages (not just "Exit code 1")
    const resultJson = await fspecCallback(
      'add-rule',
      JSON.stringify({ _: ['NONEXISTENT-001', 'Test rule'] }),
      testDir
    );

    const result = JSON.parse(resultJson);

    expect(result.success).toBe(false);
    // The error should NOT be just "Exit code 1" - it should have the actual message
    expect(result.error).not.toBe('Exit code 1');
    expect(result.error).toContain('does not exist');
  });

  it('should capture actual command error for show-work-unit (which supports --format)', async () => {
    // show-work-unit supports --format json
    // Test with a non-existent work unit to trigger an error
    const resultJson = await fspecCallback(
      'show-work-unit',
      JSON.stringify({ _: ['NONEXISTENT-999'] }),
      testDir
    );

    const result = JSON.parse(resultJson);

    expect(result.success).toBe(false);
    // The error should contain meaningful info
    expect(result.error).not.toBe('Exit code 1');
    expect(result.error).toContain('does not exist');
  });
});
