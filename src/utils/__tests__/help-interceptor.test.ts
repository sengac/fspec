/**
 * Feature: spec/features/command-help-system.feature
 * Tests for process-level help interceptor system
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

describe('Feature: Scalable help system for all commands', () => {
  let originalArgv: string[];
  let originalExit: typeof process.exit;
  let exitCode: number | undefined;

  beforeEach(() => {
    originalArgv = process.argv;
    originalExit = process.exit;

    // Mock process.exit to capture exit code
    exitCode = undefined;
    process.exit = vi.fn((code?: number) => {
      exitCode = code;
      throw new Error(`process.exit(${code})`);
    }) as never;
  });

  afterEach(() => {
    process.argv = originalArgv;
    process.exit = originalExit;
  });

  describe('Scenario: Display custom help when help config exists', () => {
    it('should load and display custom help for commands with help configs', async () => {
      // Given I have a command "remove-tag-from-scenario" with help config
      process.argv = ['node', 'fspec', 'remove-tag-from-scenario', '--help'];

      // When handleCustomHelp is called
      const { handleCustomHelp } = await import('../help-interceptor');
      let helpDisplayed = false;

      try {
        helpDisplayed = await handleCustomHelp();
      } catch (error: any) {
        // process.exit throws in test environment
        if (error.message.includes('process.exit')) {
          helpDisplayed = true;
        }
      }

      // Then the help should be displayed
      expect(helpDisplayed).toBe(true);

      // And process should exit with code 0
      expect(exitCode).toBe(0);
    });

    it('should show WHEN TO USE section in custom help', async () => {
      // Given I have a command with custom help config
      const { displayHelpAndExit } = await import('../help-formatter');
      const mockHelp = {
        name: 'test-command',
        description: 'Test command',
        whenToUse: 'Use this when testing',
      };

      // When displayHelpAndExit is called
      const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

      try {
        displayHelpAndExit(mockHelp);
      } catch {
        // Expected - process.exit throws
      }

      // Then WHEN TO USE section should be displayed
      const output = consoleSpy.mock.calls.map(call => call.join(' ')).join('\n');
      expect(output).toContain('WHEN TO USE');
      expect(output).toContain('Use this when testing');

      consoleSpy.mockRestore();
    });
  });

  describe('Scenario: Fall back to Commander.js help when no custom help exists', () => {
    it('should return false when command has no custom help config', async () => {
      // Given I have a command without help config
      process.argv = ['node', 'fspec', 'some-unknown-command', '--help'];

      // When handleCustomHelp is called
      const { handleCustomHelp } = await import('../help-interceptor');
      const result = await handleCustomHelp();

      // Then it should return false (let Commander.js handle it)
      expect(result).toBe(false);
    });

    it('should return false when command is not in registry', async () => {
      // Given a command not registered in help-registry.ts
      process.argv = ['node', 'fspec', 'unregistered-command', '--help'];

      // When handleCustomHelp checks the registry
      const { handleCustomHelp } = await import('../help-interceptor');
      const result = await handleCustomHelp();

      // Then it should fall back to Commander.js
      expect(result).toBe(false);
    });
  });

  describe('Scenario: Handle help flag in any position', () => {
    it('should detect --help before command name', async () => {
      // When --help appears before command
      process.argv = ['node', 'fspec', '--help', 'list-work-units'];

      const { handleCustomHelp } = await import('../help-interceptor');

      try {
        await handleCustomHelp();
      } catch {
        // Expected - process.exit
      }

      // Then help should be detected
      expect(exitCode).toBe(0);
    });

    it('should detect --help after command name', async () => {
      // When --help appears after command
      process.argv = ['node', 'fspec', 'list-work-units', '--help'];

      const { handleCustomHelp } = await import('../help-interceptor');

      try {
        await handleCustomHelp();
      } catch {
        // Expected - process.exit
      }

      // Then help should be detected
      expect(exitCode).toBe(0);
    });

    it('should detect --help after other arguments', async () => {
      // When --help appears after other flags
      process.argv = ['node', 'fspec', 'list-work-units', '--status=backlog', '--help'];

      const { handleCustomHelp } = await import('../help-interceptor');

      try {
        await handleCustomHelp();
      } catch {
        // Expected - process.exit
      }

      // Then help should be detected
      expect(exitCode).toBe(0);
    });

    it('should detect -h short flag', async () => {
      // When -h short flag is used
      process.argv = ['node', 'fspec', 'list-work-units', '-h'];

      const { handleCustomHelp } = await import('../help-interceptor');

      try {
        await handleCustomHelp();
      } catch {
        // Expected - process.exit
      }

      // Then help should be detected
      expect(exitCode).toBe(0);
    });
  });

  describe('Scenario: Handle main program help without command', () => {
    it('should display custom main help with command help note', async () => {
      // Given "fspec --help" without specific command
      process.argv = ['node', 'fspec', '--help'];

      // When handleCustomHelp is called
      const { handleCustomHelp } = await import('../help-interceptor');

      try {
        const result = await handleCustomHelp();
        // Should return true (custom help was displayed)
        expect(result).toBe(true);
      } catch (error: any) {
        // process.exit throws in test environment - this is expected
        if (error.message.includes('process.exit')) {
          // Help was displayed and process exited successfully
          expect(exitCode).toBe(0);
        } else {
          throw error;
        }
      }
    });
  });

  describe('Scenario: Gracefully handle missing help config file', () => {
    it('should fall back gracefully when help file import fails', async () => {
      // Given command is in registry but file is missing
      process.argv = ['node', 'fspec', 'broken-command', '--help'];

      // Simulate missing file by having registry entry but no actual file
      const { handleCustomHelp } = await import('../help-interceptor');

      // When handleCustomHelp tries to load the config
      const result = await handleCustomHelp();

      // Then it should fall back gracefully
      expect(result).toBe(false);

      // And no error should be thrown to user
      expect(exitCode).toBeUndefined();
    });
  });

  describe('Help Registry', () => {
    it('should contain all commands with custom help configs', async () => {
      const { commandsWithCustomHelp } = await import('../../commands/help-registry');

      // The registry should include existing help configs
      expect(commandsWithCustomHelp.has('create-epic')).toBe(true);
      expect(commandsWithCustomHelp.has('list-work-units')).toBe(true);
      expect(commandsWithCustomHelp.has('add-dependency')).toBe(true);
      expect(commandsWithCustomHelp.has('update-work-unit-status')).toBe(true);
      expect(commandsWithCustomHelp.has('add-question')).toBe(true);
    });

    it('should not contain commands without help configs', async () => {
      const { commandsWithCustomHelp } = await import('../../commands/help-registry');

      // Random commands without help should not be in registry
      expect(commandsWithCustomHelp.has('nonexistent-command')).toBe(false);
    });
  });

  describe('Integration with existing help configs', () => {
    it('should work with existing create-epic-help config', async () => {
      const { default: createEpicHelp } = await import('../../commands/create-epic-help');

      expect(createEpicHelp.name).toBe('create-epic');
      expect(createEpicHelp.description).toBeTruthy();
      expect(createEpicHelp.usage).toBeTruthy();
      expect(createEpicHelp.examples).toBeTruthy();
    });

    it('should work with existing list-work-units-help config', async () => {
      const { default: listWorkUnitsHelp } = await import('../../commands/list-work-units-help');

      expect(listWorkUnitsHelp.name).toBe('list-work-units');
      expect(listWorkUnitsHelp.whenToUse).toBeTruthy();
    });
  });
});
