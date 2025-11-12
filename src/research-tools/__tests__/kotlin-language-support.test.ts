/**
 * Feature: spec/features/add-kotlin-language-support-to-ast-research-tool.feature
 *
 * Tests that AST research tool supports Kotlin language.
 */

import { describe, it, expect } from 'vitest';
import { tool as astTool } from '../ast';

describe('Feature: Add Kotlin language support to AST research tool', () => {
  describe('Scenario: Parse Kotlin file and list functions', () => {
    it('should parse Kotlin file and return list of functions', async () => {
      // @step Given a Kotlin file "MyClass.kt" exists with function definitions
      // Fixture file created at test-fixtures/MyClass.kt

      // @step And tree-sitter-kotlin parser is installed
      // This will be verified when implementation is complete

      // @step When I run "fspec research --tool=ast --operation=list-functions --file=MyClass.kt"
      const result = await astTool.execute([
        '--operation',
        'list-functions',
        '--file',
        'test-fixtures/MyClass.kt',
      ]);

      const parsed = JSON.parse(result);

      // @step Then the output should contain a list of Kotlin functions
      expect(parsed.matches).toBeDefined();
      expect(Array.isArray(parsed.matches)).toBe(true);

      // @step And the output should include function names and line numbers
      expect(parsed.matches.length).toBeGreaterThan(0);
      expect(parsed.matches[0]).toHaveProperty('line');
      expect(parsed.matches[0]).toHaveProperty('column');
    });
  });
});
