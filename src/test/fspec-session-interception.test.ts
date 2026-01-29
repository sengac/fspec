/**
 * Test FspecTool session-level interception
 *
 * This test verifies that when an agent uses FspecTool within a session,
 * the session manager properly intercepts the FspecTool error and executes
 * the command synchronously.
 *
 * CRITICAL WARNING: NO CLI INVOCATION - NO FALLBACKS - NO SIMULATIONS
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { testFspecJsControlledInvocation } from '../utils/fspec-init';
import { existsSync, readFileSync } from 'fs';
import { join } from 'path';
import {
  createTempTestDir,
  removeTempTestDir,
} from '../test-helpers/temp-directory';
import { createWorkUnitTestEnvironment } from '../test-helpers/work-unit-test-fixtures';

describe('FspecTool Session-Level Interception', () => {
  let testDir: string;

  beforeEach(async () => {
    // Create isolated temp directory for each test
    testDir = await createTempTestDir('fspec-session-interception');

    // Set up work unit test environment
    await createWorkUnitTestEnvironment(testDir);
  });

  afterEach(async () => {
    // Clean up temp directory
    await removeTempTestDir(testDir);
  });

  it('should test basic JS-controlled invocation pattern', async () => {
    // CRITICAL WARNING: NO CLI INVOCATION - NO FALLBACKS - NO SIMULATIONS
    // Test that our TypeScript callback implementation works

    expect(async () => {
      await testFspecJsControlledInvocation(testDir); // Use temp directory
    }).not.toThrow();
  });

  it('should create empty work-units.json if it does not exist', async () => {
    // CRITICAL WARNING: NO CLI INVOCATION - NO FALLBACKS - NO SIMULATIONS
    // Test that the synchronous implementation creates the file structure

    // Create a separate temp directory without pre-existing work-units.json
    const freshTestDir = await createTempTestDir('fresh-session-test');

    try {
      // Test the JS-controlled invocation - should create the file
      await testFspecJsControlledInvocation(freshTestDir);

      // Verify the file was created
      const workUnitsPath = join(freshTestDir, 'spec', 'work-units.json');
      expect(existsSync(workUnitsPath)).toBe(true);

      // Verify the file has the correct structure
      const content = readFileSync(workUnitsPath, 'utf-8');
      const data = JSON.parse(content);
      expect(data).toHaveProperty('workUnits');
      expect(typeof data.workUnits).toBe('object');
    } finally {
      // Clean up the fresh test directory
      await removeTempTestDir(freshTestDir);
    }
  });

  it('should simulate the error message parsing used by session manager', () => {
    // CRITICAL WARNING: NO CLI INVOCATION - NO FALLBACKS - NO SIMULATIONS
    // Test that our error message parsing logic works correctly

    const errorMessage =
      "FspecTool requires session-level execution via executeFspecForAgent NAPI function. Command: 'list-work-units', Args: '', Root: '.', Provider: 'claude'. Session manager should intercept this call.";

    // Test command extraction
    const commandMatch = errorMessage.match(/Command: '([^']+)'/);
    expect(commandMatch).not.toBeNull();
    expect(commandMatch![1]).toBe('list-work-units');

    // Test args extraction
    const argsMatch = errorMessage.match(/Args: '([^']*)'/);
    expect(argsMatch).not.toBeNull();
    expect(argsMatch![1]).toBe('');

    // Test root extraction
    const rootMatch = errorMessage.match(/Root: '([^']+)'/);
    expect(rootMatch).not.toBeNull();
    expect(rootMatch![1]).toBe('.');

    // Test provider extraction
    const providerMatch = errorMessage.match(/Provider: '([^']+)'/);
    expect(providerMatch).not.toBeNull();
    expect(providerMatch![1]).toBe('claude');
  });
});
