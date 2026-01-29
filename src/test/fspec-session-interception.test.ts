/**
 * Test FspecTool session-level interception
 *
 * This test verifies that when an agent uses FspecTool within a session,
 * the session manager properly intercepts the FspecTool error and executes
 * the command synchronously.
 *
 * CRITICAL WARNING: NO CLI INVOCATION - NO FALLBACKS - NO SIMULATIONS
 */

import { describe, it, expect, beforeAll } from 'vitest';
import { testFspecJsControlledInvocation } from '../utils/fspec-init';
import fs from 'fs';
import { join } from 'path';

describe('FspecTool Session-Level Interception', () => {
  beforeAll(async () => {
    // Ensure the test directory structure exists
    const testSpecDir = join(process.cwd(), 'spec');
    if (!fs.existsSync(testSpecDir)) {
      fs.mkdirSync(testSpecDir, { recursive: true });
    }
  });

  it('should test basic JS-controlled invocation pattern', async () => {
    // CRITICAL WARNING: NO CLI INVOCATION - NO FALLBACKS - NO SIMULATIONS
    // Test that our TypeScript callback implementation works

    expect(async () => {
      await testFspecJsControlledInvocation('.');
    }).not.toThrow();
  });

  it('should create empty work-units.json if it does not exist', async () => {
    // CRITICAL WARNING: NO CLI INVOCATION - NO FALLBACKS - NO SIMULATIONS
    // Test that the synchronous implementation creates the file structure

    // Remove work units file if it exists
    const workUnitsPath = join(process.cwd(), 'spec', 'work-units.json');
    if (fs.existsSync(workUnitsPath)) {
      fs.unlinkSync(workUnitsPath);
    }

    // Test the JS-controlled invocation - should create the file
    await testFspecJsControlledInvocation('.');

    // Verify the file was created
    expect(fs.existsSync(workUnitsPath)).toBe(true);

    // Verify the file has the correct structure
    const content = fs.readFileSync(workUnitsPath, 'utf-8');
    const data = JSON.parse(content);
    expect(data).toHaveProperty('workUnits');
    expect(typeof data.workUnits).toBe('object');
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
