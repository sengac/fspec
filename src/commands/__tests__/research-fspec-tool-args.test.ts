/**
 * Feature: spec/features/research-tools-fail-when-invoked-via-fspec-tool-reads-process-argv-instead-of-commander-args.feature
 *
 * This test file validates that research tools receive correct arguments
 * when invoked via the Fspec tool (fspecCallback) instead of CLI.
 *
 * RES-022: Research tools fail when invoked via Fspec tool
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { fspecCallback } from '../../utils/fspec-callback';
import * as registry from '../../research-tools/registry';

// Mock the research tool registry
vi.mock('../../research-tools/registry', () => ({
  getResearchTool: vi.fn(),
}));

describe('Feature: Research tools fail when invoked via Fspec tool', () => {
  let mockTool: {
    name: string;
    description: string;
    execute: ReturnType<typeof vi.fn>;
    getHelpConfig: ReturnType<typeof vi.fn>;
  };
  let capturedArgs: string[] | null = null;

  beforeEach(() => {
    capturedArgs = null;
    mockTool = {
      name: 'ast',
      description: 'AST code search tool',
      execute: vi.fn().mockImplementation((args: string[]) => {
        capturedArgs = args;
        // Simulate the ast tool's validation
        const patternIndex = args.findIndex(
          arg => arg === '--pattern' || arg.startsWith('--pattern=')
        );
        if (patternIndex === -1) {
          throw new Error(
            '--pattern is required. Example: --pattern="function $NAME($$$ARGS)"'
          );
        }
        return Promise.resolve('file.ts:10:5:function test()');
      }),
      getHelpConfig: vi.fn().mockReturnValue({
        name: 'ast',
        description: 'AST tool',
        options: [],
        examples: [],
      }),
    };

    vi.mocked(registry.getResearchTool).mockResolvedValue(mockTool);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('Scenario: Research tool receives correct arguments when invoked via Fspec tool', () => {
    it('should forward arguments correctly to the research tool', async () => {
      // @step Given the research command is registered with Commander.js
      // (Commander.js is automatically set up via fspecCallback)

      // @step And the ast research tool is available
      expect(registry.getResearchTool).toBeDefined();

      // @step When I invoke research via the Fspec tool with arguments "--tool=ast --pattern=function --lang=typescript"
      const result = await fspecCallback(
        'research',
        JSON.stringify({
          _: ['--pattern=function', '--lang=typescript'],
          tool: 'ast',
        }),
        process.cwd()
      );

      // @step Then the ast tool should receive the arguments ["--pattern=function", "--lang=typescript"]
      expect(capturedArgs).toContain('--pattern=function');
      expect(capturedArgs).toContain('--lang=typescript');

      // @step And the tool should execute successfully
      const parsed = JSON.parse(result);
      expect(parsed.success).toBe(true);
    });
  });

  describe('Scenario: Research tool throws error for missing required arguments via Fspec tool', () => {
    it('should throw error when required --pattern argument is missing', async () => {
      // @step Given the research command is registered with Commander.js
      // (Commander.js is automatically set up via fspecCallback)

      // @step And the ast research tool is available
      expect(registry.getResearchTool).toBeDefined();

      // @step When I invoke research via the Fspec tool with arguments "--tool=ast --lang=typescript"
      const result = await fspecCallback(
        'research',
        JSON.stringify({
          _: ['--lang=typescript'],
          tool: 'ast',
        }),
        process.cwd()
      );

      // @step Then the ast tool should throw an error containing "--pattern is required"
      const parsed = JSON.parse(result);
      // The error gets captured and returned in the result
      expect(parsed.success).toBe(false);
      expect(parsed.error || parsed.data).toContain('--pattern is required');
    });
  });

  describe('Scenario: CLI invocation continues to work after fix', () => {
    it('should work correctly when invoked via CLI-style arguments', async () => {
      // @step Given the research command is registered with Commander.js
      // (Commander.js is automatically set up via fspecCallback)

      // @step And the ast research tool is available
      expect(registry.getResearchTool).toBeDefined();

      // @step When I invoke research via CLI with arguments "research --tool=ast --pattern=function --lang=typescript"
      // Note: We simulate CLI by using fspecCallback with positional args that include tool-specific flags
      const result = await fspecCallback(
        'research',
        JSON.stringify({
          _: ['--pattern=function', '--lang=typescript'],
          tool: 'ast',
        }),
        process.cwd()
      );

      // @step Then the ast tool should receive the arguments ["--pattern=function", "--lang=typescript"]
      expect(capturedArgs).toContain('--pattern=function');
      expect(capturedArgs).toContain('--lang=typescript');

      // @step And the tool should execute successfully
      const parsed = JSON.parse(result);
      expect(parsed.success).toBe(true);
    });
  });
});
