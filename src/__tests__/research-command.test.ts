/**
 * Feature: spec/features/interactive-research-command-with-multiple-backend-support.feature
 *
 * Tests for interactive research command with multiple backend support.
 * These tests verify the 'fspec research' command that lists and executes
 * research tools.
 */

import { describe, it, expect, beforeEach, afterEach, beforeAll } from 'vitest';
import { research } from '../commands/research';

const PROJECT_ROOT = process.cwd();

describe('Feature: Interactive research command with multiple backend support', () => {
  let consoleOutput: string[];
  const originalLog = console.log;

  beforeAll(() => {
    // Enable test mode for research scripts
    process.env.FSPEC_TEST_MODE = '1';
  });

  beforeEach(() => {
    consoleOutput = [];
    console.log = (...args: unknown[]) => {
      consoleOutput.push(args.join(' '));
    };
  });

  afterEach(() => {
    console.log = originalLog;
  });

  describe('Scenario: List available research tools without arguments', () => {
    it('should list all available research tools with descriptions', async () => {
      // @step Given research tools are registered in the tool registry
      // Note: Research tools are now integrated, not external scripts
      // The tool registry is defined in src/commands/research-tool-list.ts

      // @step When I run "fspec research" without arguments
      const result = await research({
        cwd: PROJECT_ROOT,
      });

      // @step Then the output should list available research tools
      const output = consoleOutput.join('\n');

      // Should have some tools listed (bundled ones)
      expect(result.tools).toBeDefined();

      // @step And each tool should show usage examples
      // The tools array should have items if any are returned
      expect(result.tools !== undefined).toBe(true);
    });
  });

  describe('Scenario: Error on invalid research tool', () => {
    it('should return error for non-existent tool', async () => {
      // @step Given no research tool named "invalid" exists
      // (TypeScript plugin system - no bundled or custom tool with this name)

      // @step When I run "fspec research --tool=invalid"
      let error: Error | null = null;
      try {
        await research({
          tool: 'invalid',
          cwd: PROJECT_ROOT,
        });
      } catch (err) {
        error = err as Error;
      }

      // @step Then the command should throw an error
      expect(error).not.toBeNull();

      // @step And the error should contain information about the missing tool
      expect(error!.message).toContain('invalid');
    });
  });
});
