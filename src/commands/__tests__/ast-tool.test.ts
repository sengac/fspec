/**
 * Feature: spec/features/language-agnostic-ast-tool-for-ai-analysis.feature
 *
 * Tests for language-agnostic AST tool using tree-sitter (RES-014).
 * Tests AST parsing, querying, and pattern detection across multiple languages.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import fs from 'fs/promises';
import path from 'path';
import os from 'os';
import { parseFile } from '../../utils/ast-parser.js';

describe('Feature: Language-Agnostic AST Tool for AI Analysis', () => {
  let tempDir: string;

  beforeEach(async () => {
    // Create temp directory for test files
    tempDir = await fs.mkdtemp(path.join(os.tmpdir(), 'fspec-ast-test-'));
  });

  afterEach(async () => {
    // Clean up temp directory
    if (tempDir) {
      await fs.rm(tempDir, { recursive: true, force: true });
    }
  });

  describe('Scenario: Find all function definitions across codebase', () => {
    it('should find all functions with file paths and line numbers', async () => {
      // @step Given I have a codebase with multiple programming languages
      const jsFile = path.join(tempDir, 'test.js');
      await fs.writeFile(
        jsFile,
        'function foo() {}\nfunction bar() {}',
        'utf-8'
      );

      // @step When I run "fspec research --tool=ast --query="function definitions""
      const result = await parseFile(jsFile, 'function definitions');

      // @step Then I should see a list of all functions in the codebase
      expect(result).toBeDefined();
      expect(result.matches).toBeDefined();
      expect(result.matches!.length).toBeGreaterThan(0);

      // @step And each result should include file path and line number
      for (const match of result.matches!) {
        expect(match.filePath).toBeDefined();
        expect(match.lineNumber).toBeDefined();
      }
    });
  });

  describe('Scenario: Find specific class definition by name', () => {
    it('should find class definition with file path and line range', async () => {
      // @step Given I have a TypeScript file with class AuthService
      const tsFile = path.join(tempDir, 'service.ts');
      const classCode =
        'class AuthService {\n  constructor() {}\n  login() {}\n}';
      await fs.writeFile(tsFile, classCode, 'utf-8');

      // @step When I query for "class AuthService"
      const result = await parseFile(tsFile, 'class AuthService');

      // @step Then the AST tool should find AuthService class definition
      expect(result).toBeDefined();
      expect(result.className).toBe('AuthService');

      // @step And the result should include file path src/auth/service.ts
      expect(result.filePath).toContain('service.ts');

      // @step And the result should include line range 42-150
      expect(result.startLine).toBeDefined();
      expect(result.endLine).toBeDefined();
    });
  });

  describe('Scenario: Search for import statements with JSON output', () => {
    it('should return JSON with import paths, line numbers, and symbols', async () => {
      // @step Given I have TypeScript files with import statements
      const tsFile = path.join(tempDir, 'imports.ts');
      const importCode =
        'import { foo, bar } from "./module";\nimport baz from "./other";';
      await fs.writeFile(tsFile, importCode, 'utf-8');

      // @step When I search for "import statements" across TypeScript files
      const result = await parseFile(tsFile, 'import statements');

      // @step Then I should receive JSON output with import paths
      expect(result).toBeDefined();
      expect(result.imports).toBeDefined();
      expect(result.imports!.length).toBeGreaterThan(0);

      // @step And the JSON should include line numbers for each import
      for (const imp of result.imports!) {
        expect(imp.lineNumber).toBeDefined();
      }

      // @step And the JSON should include imported symbols
      for (const imp of result.imports!) {
        expect(imp.symbols).toBeDefined();
      }
    });
  });

  describe('Scenario: Parse Python file for class definitions and methods', () => {
    it('should find classes and methods using tree-sitter Python grammar', async () => {
      // @step Given I have a Python file with class definitions
      const pyFile = path.join(tempDir, 'test.py');
      const pythonCode =
        'class MyClass:\n    def method1(self):\n        pass\n    def method2(self):\n        pass';
      await fs.writeFile(pyFile, pythonCode, 'utf-8');

      // @step When the AST tool parses the Python file
      const result = await parseFile(pyFile, 'class definitions');

      // @step Then it should find all class definitions
      expect(result).toBeDefined();
      expect(result.classes).toBeDefined();
      expect(result.classes!.length).toBeGreaterThan(0);

      // @step And it should find all methods within each class
      for (const cls of result.classes!) {
        expect(cls.methods).toBeDefined();
        expect(cls.methods.length).toBeGreaterThan(0);
      }

      // @step And it should use tree-sitter Python grammar
      expect(result.grammar).toBe('python');
    });
  });

  describe('Scenario: Find export default patterns across JavaScript and TypeScript', () => {
    it('should find all default exports in JS and TS files', async () => {
      // @step Given I have JavaScript and TypeScript files with export default statements
      const jsFile = path.join(tempDir, 'module.js');
      const tsFile = path.join(tempDir, 'component.ts');
      await fs.writeFile(jsFile, 'export default function foo() {}', 'utf-8');
      await fs.writeFile(tsFile, 'export default class Bar {}', 'utf-8');

      // @step When I query for "export default" pattern
      const jsResult = await parseFile(jsFile, 'export default');
      const tsResult = await parseFile(tsFile, 'export default');

      // Combine results
      const result = {
        exports: [...(jsResult.exports || []), ...(tsResult.exports || [])],
      };

      // @step Then the AST tool should find all default exports
      expect(result).toBeDefined();
      expect(result.exports).toBeDefined();
      expect(result.exports.length).toBeGreaterThan(0);

      // @step And the results should include both JavaScript and TypeScript files
      const fileExtensions = result.exports.map((exp: any) =>
        path.extname(exp.filePath)
      );
      expect(fileExtensions).toContain('.js');
      expect(fileExtensions).toContain('.ts');
    });
  });
});
