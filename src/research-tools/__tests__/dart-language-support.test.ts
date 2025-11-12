/**
 * Feature: spec/features/add-kotlin-and-dart-language-support-to-ast-research-tool.feature
 *
 * Tests that AST research tool supports Dart language.
 */

import { describe, it, expect } from 'vitest';
import { tool as astTool } from '../ast';

describe('Feature: Add Dart language support to AST research tool', () => {
  describe('Scenario: Parse Dart file and list functions', () => {
    it('should parse Dart file and return list of functions', async () => {
      // @step Given a Dart file "MyWidget.dart" exists with function definitions
      // Fixture file created at test-fixtures/MyWidget.dart

      // @step And tree-sitter-dart parser is installed
      // This will be verified when implementation is complete

      // @step When I run "fspec research --tool=ast --operation=list-functions --file=MyWidget.dart"
      const result = await astTool.execute([
        '--operation',
        'list-functions',
        '--file',
        'test-fixtures/MyWidget.dart',
      ]);

      const parsed = JSON.parse(result);

      // @step Then the output should contain a list of Dart functions
      expect(parsed.matches).toBeDefined();
      expect(Array.isArray(parsed.matches)).toBe(true);

      // @step And the output should include function names and line numbers
      expect(parsed.matches.length).toBeGreaterThan(0);
      expect(parsed.matches[0]).toHaveProperty('line');
      expect(parsed.matches[0]).toHaveProperty('column');
    });
  });
});
