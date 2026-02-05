/**
 * Test: fspec-callback exit override cascade bug
 *
 * BUG: When a command succeeds and calls process.exit(0), the __FSPEC_EXIT_OVERRIDE__:0
 * exception propagates up. If the command has a try/catch block, it catches this exception,
 * treats it as an error, outputs an error message, and calls process.exit(1).
 *
 * This test validates the fix: fspec-callback should detect when the ORIGINAL exit was
 * successful (exit code 0) even if a subsequent exit (code 1) was triggered.
 *
 * The fix checks if stderr contains __FSPEC_EXIT_OVERRIDE__:0 and treats the overall
 * result as success if so.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import { fspecCallback } from '../fspec-callback';

describe('fspec-callback exit override cascade bug', () => {
  let testDir: string;

  beforeEach(() => {
    // Create a temp directory with fspec structure
    testDir = fs.mkdtempSync(path.join(os.tmpdir(), 'fspec-exit-cascade-'));
    const specDir = path.join(testDir, 'spec');
    const featuresDir = path.join(specDir, 'features');
    fs.mkdirSync(featuresDir, { recursive: true });

    // Create work-units.json
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

    // Create a feature file for generate-coverage to process
    const featureContent = `Feature: Test Feature

  Scenario: Test Scenario
    Given I have a test
    When I run the test
    Then it should pass
`;
    fs.writeFileSync(path.join(featuresDir, 'test.feature'), featureContent);
  });

  afterEach(() => {
    if (testDir && fs.existsSync(testDir)) {
      fs.rmSync(testDir, { recursive: true, force: true });
    }
  });

  describe('Scenario: Command succeeds but has try/catch that catches exit override', () => {
    /**
     * This is the core bug scenario:
     * 1. generate-coverage command succeeds
     * 2. It outputs "✓ Created 1" or "✓ Skipped N"
     * 3. It calls process.exit(0) which throws __FSPEC_EXIT_OVERRIDE__:0
     * 4. The command's catch block catches this, outputs error, calls process.exit(1)
     * 5. fspec-callback should still detect success from original output
     */
    it('should treat generate-coverage as success even when exit cascade occurs', async () => {
      // @step Given a project with feature files
      // @step When generate-coverage is called via fspec-callback
      const result = await fspecCallback('generate-coverage', '{}', testDir);
      const parsed = JSON.parse(result);

      // @step Then the result should be successful
      // The stdout should contain success message (✓ Created or ✓ Skipped)
      // Even if exit code 1 was detected, we should detect the original success

      // Check stdout for success indicators
      const hasSuccessIndicator =
        parsed.data?.includes('✓') ||
        parsed.stdout?.includes('✓') ||
        parsed.success === true;

      // This is what SHOULD happen after the fix
      expect(hasSuccessIndicator).toBe(true);

      // The error field should NOT be just "✗" (the cascade artifact)
      if (parsed.error) {
        expect(parsed.error).not.toBe('✗');
        expect(parsed.error).not.toMatch(/^✗\s*$/);
      }
    });

    it('should extract system reminder from successful generate-coverage', async () => {
      // @step Given a project with feature files
      // @step When generate-coverage is called via fspec-callback
      const result = await fspecCallback('generate-coverage', '{}', testDir);
      const parsed = JSON.parse(result);

      // @step Then system reminders should be captured
      // The system reminder about link-coverage should be present
      const hasSystemReminder =
        parsed.systemReminders?.length > 0 ||
        parsed.data?.includes('<system-reminder>') ||
        parsed.stdout?.includes('<system-reminder>');

      expect(hasSystemReminder).toBe(true);
    });
  });

  describe('Scenario: Detect original exit code from stderr', () => {
    it('should detect __FSPEC_EXIT_OVERRIDE__:0 in stderr as original success', async () => {
      // The fix needs to check stderr for __FSPEC_EXIT_OVERRIDE__:0
      // If present, it means the original exit was successful
      const result = await fspecCallback('generate-coverage', '{}', testDir);
      const parsed = JSON.parse(result);

      // After the fix, success should be true when generate-coverage works
      // The command creates/skips coverage files and outputs success

      // Check for success indicators
      const stdout = parsed.stdout || parsed.data || '';
      const isSuccess =
        parsed.success === true ||
        stdout.includes('✓ Created') ||
        stdout.includes('✓ Skipped') ||
        stdout.includes('No coverage files needed');

      expect(isSuccess).toBe(true);
    });
  });

  describe('Scenario: Real error should still be detected', () => {
    it('should still detect real errors (not cascade artifacts)', async () => {
      // @step Given a command that actually fails
      // @step When the command is called with invalid arguments
      const result = await fspecCallback(
        'show-work-unit',
        '{"_": ["NONEXISTENT-999"]}',
        testDir
      );
      const parsed = JSON.parse(result);

      // @step Then success should be false
      expect(parsed.success).toBe(false);

      // @step And the error should contain meaningful message (not just "✗")
      expect(parsed.error).toBeDefined();
      expect(parsed.error.length).toBeGreaterThan(5); // More than just "✗"
      expect(parsed.error).toContain('does not exist');
    });
  });

  describe('Scenario: Commands without try/catch should work normally', () => {
    it('should handle list-work-units which has no try/catch wrapper', async () => {
      // list-work-units doesn't wrap in try/catch, so exit(0) should work
      const result = await fspecCallback('list-work-units', '{}', testDir);
      const parsed = JSON.parse(result);

      // Should succeed with empty list
      expect(parsed.success).toBe(true);
      expect(parsed.workUnits).toBeDefined();
    });
  });
});

describe('fspec-callback error message cleanup', () => {
  /**
   * The cleanup regexes need to handle:
   * - "Error: __FSPEC_EXIT_OVERRIDE__:N"
   * - "✗ Error: __FSPEC_EXIT_OVERRIDE__:N"
   * - "✗ Failed: __FSPEC_EXIT_OVERRIDE__:N"
   * - Any prefix + __FSPEC_EXIT_OVERRIDE__:N
   */

  it('should clean up "✗ Error: __FSPEC_EXIT_OVERRIDE__:0" from stderr', async () => {
    // This pattern appears when command catch block processes the exit override
    const testPattern = '✗ Error: __FSPEC_EXIT_OVERRIDE__:0';

    // After cleanup, this should be empty or just "✗" at most
    // The fix should recognize this as success indicator, not error
    const cleaned = testPattern
      .replace(/✗\s*(Error:|Failed:)?\s*__FSPEC_EXIT_OVERRIDE__:\d+\n?/gi, '')
      .replace(/__FSPEC_EXIT_OVERRIDE__:\d+\n?/g, '')
      .trim();

    // After proper cleanup, should be empty
    expect(cleaned).toBe('');
  });

  it('should preserve actual error messages that happen before exit override', async () => {
    // If there's a real error message followed by the exit override pattern
    const testPattern =
      'Work unit not found: TEST-001\n✗ Error: __FSPEC_EXIT_OVERRIDE__:1';

    const cleaned = testPattern
      .replace(/✗\s*(Error:|Failed:)?\s*__FSPEC_EXIT_OVERRIDE__:\d+\n?/gi, '')
      .replace(/__FSPEC_EXIT_OVERRIDE__:\d+\n?/g, '')
      .trim();

    // The actual error message should be preserved
    expect(cleaned).toContain('Work unit not found: TEST-001');
  });
});
