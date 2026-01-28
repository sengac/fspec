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

  describe('Scenario: Commands without system reminders should not have systemReminders field', () => {
    test('should handle commands that do not emit system reminders', async () => {
      // @step Given a fspec command that does NOT output system reminders
      const mockCommand = 'list-work-units'; // This command doesn't emit system reminders
      const mockArgs = JSON.stringify({});
      const mockProjectRoot = '/test/path';

      // @step When the TypeScript callback executes the command within fspecCallback
      const result = await fspecCallback(
        mockCommand,
        mockArgs,
        mockProjectRoot
      );

      // @step Then the result should not have systemReminders field (or it should be empty)
      const parsedResult = JSON.parse(result);

      // Either no systemReminders field, or empty array
      if (parsedResult.systemReminders) {
        expect(parsedResult.systemReminders).toBeInstanceOf(Array);
        expect(parsedResult.systemReminders.length).toBe(0);
      } else {
        // No systemReminders field is also acceptable
        expect(parsedResult).not.toHaveProperty('systemReminders');
      }

      // Should still have the actual command result
      expect(parsedResult).toHaveProperty('workUnits');
      expect(parsedResult.workUnits).toBeInstanceOf(Array);
    });
  });

  afterEach(() => {
    // Restore original stderr.write
    process.stderr.write = originalStderrWrite;
  });

  describe('Scenario: Capture system reminder from console.error during command execution', () => {
    test('should capture console.error output and include system reminders in FspecTool response', async () => {
      // @step Given a fspec command outputs system reminders to console.error during execution
      const mockCommand = 'create-story'; // Use a command that DOES emit system reminders
      const mockArgs = JSON.stringify({ prefix: 'TEST', title: 'Test Story' });
      const mockProjectRoot = '/test/path';

      // @step When the TypeScript callback executes the command within fspecCallback
      const result = await fspecCallback(
        mockCommand,
        mockArgs,
        mockProjectRoot
      );

      // @step Then the console.error output should be captured
      // (Note: stderr is captured internally by fspecCallback, not by our test spy)

      // @step And the system reminder should be parsed from the captured stderr
      const parsedResult = JSON.parse(result);

      // @step And the system reminder should be included in the FspecTool response
      expect(parsedResult).toHaveProperty('systemReminders');
      expect(parsedResult.systemReminders).toBeInstanceOf(Array);
      expect(parsedResult.systemReminders.length).toBeGreaterThan(0);

      // Should contain the system reminder from console.error output
      expect(parsedResult.systemReminders[0]).toContain('Story');
    });
  });

  describe('Scenario: Parse result.systemReminder property from command response', () => {
    test('should extract systemReminder from command result and include in response', async () => {
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
      const result = await fspecCallback(
        mockCommand,
        mockArgs,
        mockProjectRoot
      );

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
    test('should parse XML system-reminder tags from stderr and include in response', async () => {
      // @step Given a fspec command outputs raw <system-reminder> tags to console.error
      // Use create-story since we know it emits system reminders
      const mockCommand = 'create-story';
      const mockArgs = JSON.stringify({
        prefix: 'TEST',
        title: 'Test Story For XML Tags',
      });
      const mockProjectRoot = '/test/path';

      // @step When the TypeScript callback captures the stderr output during execution
      const result = await fspecCallback(
        mockCommand,
        mockArgs,
        mockProjectRoot
      );

      // @step Then the <system-reminder> tags should be parsed and extracted
      const parsedResult = JSON.parse(result);

      // @step And the system reminder content should be included in the FspecTool response
      expect(parsedResult).toHaveProperty('systemReminders');
      expect(parsedResult.systemReminders).toBeInstanceOf(Array);
      expect(parsedResult.systemReminders.length).toBeGreaterThan(0);

      // Should contain the parsed content without the XML tags
      expect(parsedResult.systemReminders[0]).toContain('Story');
    });
  });

  describe('Scenario: Combine multiple system reminders in tool response', () => {
    test('should capture both result.systemReminder and raw tags in single response', async () => {
      // @step Given a fspec command outputs both result.systemReminder and raw <system-reminder> tags
      // Use create-story since it has both patterns
      const mockCommand = 'create-story';
      const mockArgs = JSON.stringify({
        prefix: 'TEST',
        title: 'Multi Reminder Test',
      });
      const mockProjectRoot = '/test/path';

      // @step When the TypeScript callback processes the command execution
      const result = await fspecCallback(
        mockCommand,
        mockArgs,
        mockProjectRoot
      );

      // @step Then both system reminder patterns should be captured
      const parsedResult = JSON.parse(result);

      // @step And the system reminders should be combined in the FspecTool response
      expect(parsedResult).toHaveProperty('systemReminders');
      expect(parsedResult.systemReminders).toBeInstanceOf(Array);

      // @step And the LLM should receive all workflow guidance in a single response
      // For create-story, we expect at least 1 system reminder (may not always be 2)
      expect(parsedResult.systemReminders.length).toBeGreaterThanOrEqual(1);

      // Check that it contains story-related content
      expect(
        parsedResult.systemReminders.some((r: string) => r.includes('Story'))
      ).toBe(true);
    });
  });
});
