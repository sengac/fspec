/**
 * Feature: spec/features/ast-tool-returns-stub-instead-of-actual-parsing.feature
 *
 * Tests that AST research tool uses actual tree-sitter parsing instead of returning stub.
 */

import { describe, it, expect } from 'vitest';
import { tool as astTool } from '../ast';

describe('Feature: AST tool returns stub instead of actual parsing', () => {
  describe('Scenario: AST tool executes actual tree-sitter parsing instead of returning stub', () => {
    it('should use tree-sitter parser and return actual matches', async () => {
      // @step Given the AST parser implementation exists in src/utils/query-executor.ts
      // @step And tree-sitter is installed with JavaScript, TypeScript, Python, and Go parsers
      // (Verified by imports in ast.ts)

      // @step When I run "fspec research --tool=ast --operation=list-functions --file=src/research-tools/ast.ts"
      const result = await astTool.execute([
        '--operation',
        'list-functions',
        '--file',
        'src/research-tools/ast.ts',
      ]);

      // @step Then the tool should use tree-sitter Query executor
      // @step And the tool should return actual AST matches using tree-sitter
      const parsed = JSON.parse(result);

      // @step And the output should NOT contain "stub-implementation"
      expect(result).not.toContain('stub-implementation');
      expect(parsed.status).not.toBe('stub-implementation');

      // @step And the output should contain function matches from the codebase
      expect(parsed).toBeDefined();
      expect(parsed.matches).toBeDefined();
      expect(parsed.matches.length).toBeGreaterThan(0);
    });
  });
});
