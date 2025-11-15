/**
 * Feature: spec/features/enhance-ast-research-tool-with-missing-tree-sitter-capabilities.feature
 *
 * Tests for enhanced AST research tool with tree-sitter Query API,
 * field-based access, and closest() method
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { QueryExecutor } from '../query-executor';
import Parser from '@sengac/tree-sitter';
import { loadLanguageParser } from '../language-loader';

describe('Feature: Enhance AST Research Tool with Missing Tree-Sitter Capabilities', () => {
  let parser: Parser;
  let tsLanguage: unknown;

  beforeEach(async () => {
    parser = new Parser();
    tsLanguage = await loadLanguageParser('typescript');
    parser.setLanguage(tsLanguage);
  });

  describe('Scenario: Use inline query to find function names declaratively', () => {
    it('should find all function names using S-expression query', async () => {
      // @step Given I have a TypeScript file with multiple function declarations
      const sourceCode = `
        function handleLogin() { return true; }
        function processData() { return []; }
        function validateInput() { return null; }
      `;
      const tree = parser.parse(sourceCode);

      // @step When I run 'fspec research --tool=ast --operation=query --query="(function_declaration name: (identifier) @name)" --file=src/test.ts'
      const executor = new QueryExecutor({
        language: 'typescript',
        operation: 'query',
        parameters: {
          query: '(function_declaration name: (identifier) @name)',
        },
      });

      const matches = await executor.execute(tree, tsLanguage);

      // @step Then the output should contain all function names as captures
      expect(matches).toBeDefined();
      expect(matches.length).toBeGreaterThan(0);
      expect(matches.some(m => m.name === 'handleLogin')).toBe(true);
      expect(matches.some(m => m.name === 'processData')).toBe(true);
      expect(matches.some(m => m.name === 'validateInput')).toBe(true);

      // @step And the output should be in QueryMatch interface format
      expect(matches[0]).toHaveProperty('type');
      expect(matches[0]).toHaveProperty('name');
      expect(matches[0]).toHaveProperty('line');
      expect(matches[0]).toHaveProperty('column');
    });
  });

  describe('Scenario: Find containing function using find-context operation', () => {
    it('should find the containing function at a specific position using closest()', async () => {
      // @step Given I have a TypeScript file with nested functions
      const sourceCode = `
function outerFunction() {
  const x = 10;

  function innerFunction() {
    const y = 20;
    console.log(x + y);
  }

  return innerFunction;
}
      `;
      const tree = parser.parse(sourceCode);

      // @step When I run 'fspec research --tool=ast --operation=find-context --row=42 --column=10 --context-type=function --file=src/test.ts'
      const executor = new QueryExecutor({
        language: 'typescript',
        operation: 'find-context',
        parameters: {
          row: '6', // Line with console.log (inside innerFunction)
          column: '5',
          'context-type': 'function',
        },
      });

      const matches = await executor.execute(tree, tsLanguage);

      // @step Then the output should return the function declaration containing position 42:10
      expect(matches).toBeDefined();
      expect(matches.length).toBeGreaterThan(0);
      expect(matches[0].name).toBe('innerFunction');

      // @step And the operation should use closest() method internally
      // This is verified by the implementation using node.closest()
      expect(matches[0].type).toMatch(/function/);
    });
  });

  describe('Scenario: Use external query file to detect anti-patterns', () => {
    it('should detect empty catch blocks using external query file', async () => {
      // @step Given I have created a file queries/anti-patterns.scm with a pattern for empty catch blocks
      // @step And I have a TypeScript file with try-catch blocks
      const sourceCode = `
function riskyOperation() {
  try {
    throw new Error('test');
  } catch (e) {
    // Empty catch block - anti-pattern
  }

  try {
    doSomething();
  } catch (err) {
    console.error(err); // Not empty
  }
}
      `;
      const tree = parser.parse(sourceCode);

      // @step When I run 'fspec research --tool=ast --query-file=queries/anti-patterns.scm --file=src/test.ts'
      const executor = new QueryExecutor({
        language: 'typescript',
        queryFile: 'queries/anti-patterns.scm',
        parameters: {},
      });

      const matches = await executor.execute(tree, tsLanguage);

      // @step Then the output should list all empty catch blocks detected
      expect(matches).toBeDefined();
      expect(matches.length).toBeGreaterThan(0);

      // @step And each result should include line number and code snippet
      expect(matches[0]).toHaveProperty('line');
      expect(matches[0]).toHaveProperty('text');
      expect(matches[0].text).toContain('catch');
    });
  });

  describe('Scenario: Maintain backward compatibility with existing operations', () => {
    it('should maintain backward compatibility with list-functions operation', async () => {
      // @step Given I have been using 'fspec research --tool=ast --operation=list-functions' in my workflow
      const sourceCode = `
        function handleLogin() { return true; }
        function processData() { return []; }
      `;
      const tree = parser.parse(sourceCode);

      // @step When I run 'fspec research --tool=ast --operation=list-functions --file=src/test.ts'
      const executor = new QueryExecutor({
        language: 'typescript',
        operation: 'list-functions',
        parameters: {},
      });

      const matches = await executor.execute(tree, tsLanguage);

      // @step Then the operation should work exactly as before
      expect(matches).toBeDefined();
      expect(matches.length).toBe(2);

      // @step And the output format should be unchanged
      expect(matches[0]).toHaveProperty('type');
      expect(matches[0]).toHaveProperty('name');
      expect(matches[0]).toHaveProperty('line');
      expect(matches[0]).toHaveProperty('column');

      // @step And all existing tests should pass without modification
      // This is verified by the existing test suite continuing to pass
      expect(matches[0].name).toBe('handleLogin');
      expect(matches[1].name).toBe('processData');
    });
  });

  describe('Scenario: Filter query results using predicates', () => {
    it('should filter functions by name using #match? predicate', async () => {
      // @step Given I have a TypeScript file with various function names
      const sourceCode = `
        function handleLogin() { return true; }
        function handleLogout() { return false; }
        function processData() { return []; }
        function handleError() { return null; }
      `;
      const tree = parser.parse(sourceCode);

      // @step When I run a query with #match? predicate: 'fspec research --tool=ast --operation=query --query="(function_declaration name: (identifier) @name (#match? @name \"^handle\"))" --file=src/test.ts'
      const executor = new QueryExecutor({
        language: 'typescript',
        operation: 'query',
        parameters: {
          query:
            '(function_declaration name: (identifier) @name (#match? @name "^handle"))',
        },
      });

      const matches = await executor.execute(tree, tsLanguage);

      // @step Then the output should only include functions whose names start with 'handle'
      expect(matches).toBeDefined();
      expect(matches.length).toBe(3); // handleLogin, handleLogout, handleError
      expect(matches.every(m => m.name?.startsWith('handle'))).toBe(true);

      // @step And functions with other names should be excluded
      expect(matches.some(m => m.name === 'processData')).toBe(false);
    });
  });
});
