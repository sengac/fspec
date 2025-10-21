/**
 * Feature: spec/features/system-reminder-formatting-for-ai.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect } from 'vitest';
import type { HookExecutionResult } from '../types.js';
import { formatHookOutput } from '../formatting.js';

describe('Feature: System reminder formatting for AI', () => {
  describe('Scenario: Blocking hook fails with stderr wrapped in system-reminder', () => {
    it('should wrap blocking hook stderr in system-reminder tags', () => {
      // Given I have a blocking hook that fails with exit code 1
      // And the hook outputs "Invalid config" to stderr
      const result: HookExecutionResult = {
        hookName: 'lint',
        success: false,
        exitCode: 1,
        stdout: '',
        stderr: 'Invalid config',
        timedOut: false,
        duration: 100,
      };
      const isBlocking = true;

      // When I format the hook output for display
      const formatted = formatHookOutput(result, isBlocking);

      // Then the stderr should be wrapped in system-reminder tags
      expect(formatted).toContain('<system-reminder>');
      expect(formatted).toContain('</system-reminder>');

      // And the system-reminder should include the hook name
      expect(formatted).toMatch(/Hook:\s*lint/);

      // And the system-reminder should include the exit code
      expect(formatted).toMatch(/Exit code:\s*1/);

      // And the system-reminder should include the stderr content
      expect(formatted).toContain('Invalid config');
    });
  });

  describe('Scenario: Non-blocking hook stderr displayed as-is', () => {
    it('should display non-blocking hook stderr without wrapping', () => {
      // Given I have a non-blocking hook that fails with exit code 1
      // And the hook outputs "Warning: deprecated API" to stderr
      const result: HookExecutionResult = {
        hookName: 'check',
        success: false,
        exitCode: 1,
        stdout: '',
        stderr: 'Warning: deprecated API',
        timedOut: false,
        duration: 100,
      };
      const isBlocking = false;

      // When I format the hook output for display
      const formatted = formatHookOutput(result, isBlocking);

      // Then the stderr should be displayed as-is
      expect(formatted).toContain('Warning: deprecated API');

      // And the stderr should not be wrapped in system-reminder tags
      expect(formatted).not.toContain('<system-reminder>');
      expect(formatted).not.toContain('</system-reminder>');
    });
  });

  describe('Scenario: Blocking hook succeeds with no system-reminder', () => {
    it('should not generate system-reminder for successful hooks', () => {
      // Given I have a blocking hook that succeeds with exit code 0
      // And the hook outputs "All checks passed" to stdout
      const result: HookExecutionResult = {
        hookName: 'validate',
        success: true,
        exitCode: 0,
        stdout: 'All checks passed',
        stderr: '',
        timedOut: false,
        duration: 100,
      };
      const isBlocking = true;

      // When I format the hook output for display
      const formatted = formatHookOutput(result, isBlocking);

      // Then no system-reminder should be generated
      expect(formatted).not.toContain('<system-reminder>');

      // And only stdout should be displayed
      expect(formatted).toContain('All checks passed');
    });
  });

  describe('Scenario: Blocking hook fails with empty stderr produces system-reminder', () => {
    it('should generate system-reminder even when stderr is empty', () => {
      // Given I have a blocking hook that fails with exit code 1
      // And the hook produces no stderr output
      const result: HookExecutionResult = {
        hookName: 'test',
        success: false,
        exitCode: 1,
        stdout: '',
        stderr: '',
        timedOut: false,
        duration: 100,
      };
      const isBlocking = true;

      // When I format the hook output for display
      const formatted = formatHookOutput(result, isBlocking);

      // Then system-reminder should be generated with generic message
      expect(formatted).toContain('<system-reminder>');
      expect(formatted).toContain('Hook: test');
      expect(formatted).toContain('Exit code: 1');
      expect(formatted).toContain('(Hook failed with no error output)');
      expect(formatted).toContain('</system-reminder>');
    });
  });

  describe('Scenario: System-reminder includes hook metadata', () => {
    it('should include hook name, exit code, and stderr in system-reminder', () => {
      // Given I have a blocking hook named "validate" that fails
      // And the hook exits with code 1
      // And the hook outputs "Validation failed: missing field 'name'" to stderr
      const result: HookExecutionResult = {
        hookName: 'validate',
        success: false,
        exitCode: 1,
        stdout: '',
        stderr: "Validation failed: missing field 'name'",
        timedOut: false,
        duration: 100,
      };
      const isBlocking = true;

      // When I format the hook output for display
      const formatted = formatHookOutput(result, isBlocking);

      // Then the system-reminder should contain "Hook: validate"
      expect(formatted).toMatch(/Hook:\s*validate/);

      // And the system-reminder should contain "Exit code: 1"
      expect(formatted).toMatch(/Exit code:\s*1/);

      // And the system-reminder should contain "Validation failed: missing field 'name'"
      expect(formatted).toContain("Validation failed: missing field 'name'");
    });
  });
});
