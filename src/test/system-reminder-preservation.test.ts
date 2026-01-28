/* eslint-disable @typescript-eslint/no-unused-vars */
// Feature: spec/features/system-reminder-preservation.feature
//
// Tests for System Reminder Preservation - capturing system reminders from fspec TypeScript command execution
// Maps to scenarios in the feature file with @step comments.

import { fspecCallback } from '../utils/fspec-callback';
import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest';

describe('System Reminder Preservation', () => {
  let originalStderrWrite: typeof process.stderr.write;

  beforeEach(() => {
    // Save original stderr.write for restoration
    originalStderrWrite = process.stderr.write;
  });

  afterEach(() => {
    // Restore original stderr.write
    process.stderr.write = originalStderrWrite;
  });

  describe('Scenario: Capture system reminder from console.error during command execution', () => {
    test('should capture console.error output and include system reminders in FspecTool response', () => {
      let capturedStderr = '';

      // @step Given a fspec command outputs system reminders to console.error during execution
      const mockCommand = 'list-work-units'; // Use existing command
      const mockArgs = JSON.stringify({});
      const mockProjectRoot = '/test/path';

      // Mock process.stderr.write to capture output
      vi.spyOn(process.stderr, 'write').mockImplementation((data: unknown) => {
        capturedStderr += data;
        return true;
      });

      // @step When the TypeScript callback executes the command within fspecCallback
      const result = fspecCallback(mockCommand, mockArgs, mockProjectRoot);

      // @step Then the console.error output should be captured
      expect(capturedStderr).toContain('system-reminder');

      // @step And the system reminder should be parsed from the captured stderr
      const parsedResult = JSON.parse(result);

      // @step And the system reminder should be included in the FspecTool response
      expect(parsedResult).toHaveProperty('systemReminders');
      expect(parsedResult.systemReminders).toBeInstanceOf(Array);
      expect(parsedResult.systemReminders.length).toBeGreaterThan(0);

      // Should contain the system reminder from console.error output
      expect(parsedResult.systemReminders[0]).toContain(
        'Work unit listing completed'
      );
    });
  });

  describe('Scenario: Parse result.systemReminder property from command response', () => {
    test('should extract systemReminder from command result and include in response', () => {
      // @step Given a fspec command returns a result with systemReminder property
      const mockCommand = 'create-story';
      const mockArgs = JSON.stringify({ prefix: 'TEST', title: 'Test Story' });
      const mockProjectRoot = '/test/path';

      // Mock a command that would return systemReminder
      const originalConsoleError = console.error;

      console.error = vi.fn(data => {
        originalConsoleError(data);
      });

      // @step When the TypeScript callback processes the command result
      const result = fspecCallback(mockCommand, mockArgs, mockProjectRoot);

      // @step Then the result.systemReminder content should be extracted
      const parsedResult = JSON.parse(result);

      // @step And the system reminder should be included in the FspecTool response for LLM guidance
      expect(parsedResult).toHaveProperty('systemReminders');
      expect(parsedResult.systemReminders).toEqual(
        expect.arrayContaining([
          expect.stringMatching(
            /Work unit.*has no estimate|Example Mapping|update-work-unit-estimate/
          ),
        ])
      );

      console.error = originalConsoleError;
    });
  });

  describe('Scenario: Parse raw <system-reminder> tags from console.error output', () => {
    test('should parse XML system-reminder tags from stderr and include in response', () => {
      // @step Given a fspec command outputs raw <system-reminder> tags to console.error
      const mockCommand = 'update-work-unit-status'; // Use command that outputs to stderr
      const mockArgs = JSON.stringify({
        workUnitId: 'TEST-001',
        status: 'testing',
      });
      const mockProjectRoot = '/test/path';

      // Mock stderr capture to simulate <system-reminder> tag output
      process.stderr.write = vi.fn((data: unknown) => {
        return true;
      });

      // @step When the TypeScript callback captures the stderr output during execution
      const result = fspecCallback(mockCommand, mockArgs, mockProjectRoot);

      // @step Then the <system-reminder> tags should be parsed and extracted
      const parsedResult = JSON.parse(result);

      // @step And the system reminder content should be included in the FspecTool response
      expect(parsedResult).toHaveProperty('systemReminders');
      expect(parsedResult.systemReminders).toBeInstanceOf(Array);
      expect(parsedResult.systemReminders.length).toBeGreaterThan(0);

      // Should contain the parsed content without the XML tags
      expect(parsedResult.systemReminders[0]).toContain(
        'Status updated successfully'
      );
    });
  });

  describe('Scenario: Combine multiple system reminders in tool response', () => {
    test('should capture both result.systemReminder and raw tags in single response', () => {
      // @step Given a fspec command outputs both result.systemReminder and raw <system-reminder> tags
      const mockCommand = 'create-story'; // This command has both patterns
      const mockArgs = JSON.stringify({ prefix: 'TEST', title: 'Test Unit' });
      const mockProjectRoot = '/test/path';

      process.stderr.write = vi.fn((data: unknown) => {
        return true;
      });

      // Simulate multiple reminder sources
      console.error = vi.fn();

      // @step When the TypeScript callback processes the command execution
      const result = fspecCallback(mockCommand, mockArgs, mockProjectRoot);

      // @step Then both system reminder patterns should be captured
      const parsedResult = JSON.parse(result);

      // @step And the system reminders should be combined in the FspecTool response
      expect(parsedResult).toHaveProperty('systemReminders');
      expect(parsedResult.systemReminders).toBeInstanceOf(Array);

      // @step And the LLM should receive all workflow guidance in a single response
      // Should contain reminders from both result.systemReminder and stderr output
      expect(parsedResult.systemReminders.length).toBeGreaterThanOrEqual(2);

      // Check for result.systemReminder content
      expect(
        parsedResult.systemReminders.some((r: string) => r.includes('estimate'))
      ).toBe(true);

      // Check for stderr <system-reminder> content
      expect(
        parsedResult.systemReminders.some((r: string) =>
          r.includes('Example Mapping')
        )
      ).toBe(true);
    });
  });
});
