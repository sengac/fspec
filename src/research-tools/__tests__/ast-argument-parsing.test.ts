/**
 * Feature: spec/features/ast-research-tool-fails-with-file-is-required-error-even-when-file-flag-is-provided.feature
 *
 * Tests for AST tool argument parsing to handle both --flag=value and --flag value formats.
 */

import { describe, it, expect } from 'vitest';
import { tool as astTool } from '../ast';

describe('Feature: AST research tool fails with --file is required error even when --file flag is provided', () => {
  describe('Scenario: Parse --file argument in equals format', () => {
    it('should successfully parse --file=value format', async () => {
      // @step Given I have the AST research tool
      const tool = astTool;

      // @step When I run 'fspec research --tool=ast --operation=list-functions --file=src/git/diff.ts'
      // Note: This simulates what the CLI would pass after filtering out --tool
      const result = await tool.execute([
        '--operation=list-functions',
        '--file=src/git/diff.ts',
      ]);

      // @step Then the tool should successfully parse the --file argument
      expect(result).toBeDefined();

      // @step And the tool should return JSON with matches
      const parsed = JSON.parse(result);
      expect(parsed).toHaveProperty('matches');
      expect(Array.isArray(parsed.matches)).toBe(true);
    });
  });

  describe('Scenario: Parse --file argument in space-separated format', () => {
    it('should successfully parse --file value format', async () => {
      // @step Given I have the AST research tool
      const tool = astTool;

      // @step When I run 'fspec research --tool=ast --operation list-functions --file src/git/diff.ts'
      const result = await tool.execute([
        '--operation',
        'list-functions',
        '--file',
        'src/git/diff.ts',
      ]);

      // @step Then the tool should successfully parse the --file argument
      expect(result).toBeDefined();

      // @step And the tool should return JSON with matches
      const parsed = JSON.parse(result);
      expect(parsed).toHaveProperty('matches');
      expect(Array.isArray(parsed.matches)).toBe(true);
    });
  });
});
