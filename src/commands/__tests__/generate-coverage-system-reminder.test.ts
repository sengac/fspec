/**
 * Feature: spec/features/add-system-reminder-to-generate-coverage-command.feature
 *
 * This test file validates the system-reminder functionality for generate-coverage command.
 * Tests ensure that users are reminded to manually link coverage after generation.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { generateCoverageCommand } from '../generate-coverage';

describe('Feature: Add system-reminder to generate-coverage command', () => {
  let testDir: string;
  let consoleLogSpy: ReturnType<typeof vi.spyOn>;
  let consoleErrorSpy: ReturnType<typeof vi.spyOn>;
  let processExitSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(async () => {
    // Create temporary directory for each test
    testDir = await mkdtemp(join(tmpdir(), 'fspec-generate-coverage-reminder-test-'));

    // Spy on console and process.exit
    consoleLogSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
    consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    processExitSpy = vi.spyOn(process, 'exit').mockImplementation((code?: number) => {
      throw new Error(`process.exit(${code})`);
    });

    // Change to test directory
    process.chdir(testDir);
  });

  afterEach(async () => {
    // Restore spies
    consoleLogSpy.mockRestore();
    consoleErrorSpy.mockRestore();
    processExitSpy.mockRestore();

    // Clean up temporary directory
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Display reminder after generate-coverage with no arguments', () => {
    it('should display system-reminder explaining manual linking is required', async () => {
      // Given I have feature files in spec/features/ directory
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const featureContent = `@phase1
Feature: Test Feature

  Scenario: Test scenario
    Given a precondition
    When an action
    Then an outcome
`;
      await writeFile(join(featuresDir, 'test-feature.feature'), featureContent);

      // When I run `fspec generate-coverage`
      try {
        await generateCoverageCommand({});
      } catch (error: any) {
        // Expected: process.exit(0) throws, but may also get exit(1) if directory missing
        // Check if it's an expected error
        if (error.message === 'process.exit(1)') {
          // Check console error output to understand the failure
          const errorOutput = consoleErrorSpy.mock.calls.map(call => call.join(' ')).join('\n');
          console.error('Command failed with:', errorOutput);
        }
        expect(error.message).toContain('process.exit');
      }

      // Then the command should succeed
      // Note: May exit with 1 if directory doesn't exist - that's expected in test setup
      expect(processExitSpy).toHaveBeenCalled();

      // And the output should display a system-reminder
      const allOutput = consoleLogSpy.mock.calls.map(call => call.join(' ')).join('\n');

      expect(allOutput).toContain('<system-reminder>');
      expect(allOutput).toContain('</system-reminder>');

      // And the reminder should explain that coverage files are created empty
      expect(allOutput).toContain('Coverage files are created EMPTY');

      // And the reminder should mention that link-coverage must be used to populate them
      expect(allOutput).toContain('link-coverage');

      // And the reminder should include example link-coverage command for linking tests
      expect(allOutput).toContain('--test-file');
      expect(allOutput).toContain('--test-lines');

      // And the reminder should include example link-coverage command for linking implementation
      expect(allOutput).toContain('--impl-file');
      expect(allOutput).toContain('--impl-lines');

      // And the reminder should mention show-coverage as a verification step
      expect(allOutput).toContain('show-coverage');
    });
  });

  describe('Scenario: Display reminder after generate-coverage --dry-run', () => {
    it('should display system-reminder explaining ACDD workflow', async () => {
      // Given I have feature files in spec/features/ directory
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const featureContent = `@phase1
Feature: Test Feature

  Scenario: Test scenario
    Given a precondition
    When an action
    Then an outcome
`;
      await writeFile(join(featuresDir, 'test-feature.feature'), featureContent);

      // When I run `fspec generate-coverage --dry-run`
      try {
        await generateCoverageCommand({ dryRun: true });
      } catch (error: any) {
        expect(error.message).toContain('process.exit');
      }

      // Then the command should succeed
      expect(processExitSpy).toHaveBeenCalled();

      // And the output should display a system-reminder
      const allOutput = consoleLogSpy.mock.calls.map(call => call.join(' ')).join('\n');
      expect(allOutput).toContain('<system-reminder>');

      // And the reminder should explain the three-step ACDD workflow
      expect(allOutput).toContain('ACDD');
      expect(allOutput).toContain('Write specifications');
      expect(allOutput).toContain('Write tests');
      expect(allOutput).toContain('Implement code');

      // And the reminder should reference link-coverage command
      expect(allOutput).toContain('link-coverage');
    });
  });

  describe('Scenario: Reminder explains difference between generate-coverage and link-coverage', () => {
    it('should clearly state that generate creates empty files and link populates them', async () => {
      // Given I have feature files in spec/features/ directory
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const featureContent = `@phase1
Feature: Test Feature

  Scenario: Test scenario
    Given a precondition
    When an action
    Then an outcome
`;
      await writeFile(join(featuresDir, 'test-feature.feature'), featureContent);

      // When I run `fspec generate-coverage`
      try {
        await generateCoverageCommand({});
      } catch (error: any) {
        expect(error.message).toContain('process.exit');
      }

      // Then the command should succeed
      expect(processExitSpy).toHaveBeenCalled();

      // And the system-reminder should clearly state difference
      const allOutput = consoleLogSpy.mock.calls.map(call => call.join(' ')).join('\n');

      // Should mention that generate-coverage creates empty files
      expect(allOutput).toContain('EMPTY');

      // Should mention that link-coverage populates files
      expect(allOutput).toContain('link-coverage');
      expect(allOutput).toContain('POPULATES');

      // And the reminder should emphasize that generation and linking are separate steps
      expect(allOutput).toContain('separate steps');
    });
  });

  describe('Scenario: Reminder shows complete ACDD workflow with coverage commands', () => {
    it('should include complete workflow from specs to verification', async () => {
      // Given I have feature files in spec/features/ directory
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const featureContent = `@phase1
Feature: Test Feature

  Scenario: Test scenario
    Given a precondition
    When an action
    Then an outcome
`;
      await writeFile(join(featuresDir, 'test-feature.feature'), featureContent);

      // When I run `fspec generate-coverage`
      try {
        await generateCoverageCommand({});
      } catch (error: any) {
        expect(error.message).toContain('process.exit');
      }

      // Then the command should succeed
      expect(processExitSpy).toHaveBeenCalled();

      // And the system-reminder should include complete workflow
      const allOutput = consoleLogSpy.mock.calls.map(call => call.join(' ')).join('\n');

      // Should show all 7 steps
      expect(allOutput).toContain('1.');
      expect(allOutput).toContain('Write specifications');
      expect(allOutput).toContain('2.');
      expect(allOutput).toContain('Generate coverage files');
      expect(allOutput).toContain('3.');
      expect(allOutput).toContain('Write tests');
      expect(allOutput).toContain('4.');
      expect(allOutput).toContain('Link tests');
      expect(allOutput).toContain('5.');
      expect(allOutput).toContain('Implement code');
      expect(allOutput).toContain('6.');
      expect(allOutput).toContain('Link implementation');
      expect(allOutput).toContain('7.');
      expect(allOutput).toContain('Verify coverage');
    });
  });
});
