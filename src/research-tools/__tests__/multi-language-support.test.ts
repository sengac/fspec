/**
 * Feature: spec/features/add-multi-language-support-to-ast-research-tool-kotlin-dart-python-go-rust-swift-c-c-c.feature
 *
 * Tests that AST research tool supports multiple programming languages.
 */

import { describe, it, expect } from 'vitest';
import { tool as astTool } from '../ast';

describe('Feature: Add multi-language support to AST research tool', () => {
  describe('Scenario: Parse Python file and list functions', () => {
    it('should parse Python file and return list of functions', async () => {
      // @step Given a Python file "main.py" exists with function definitions
      // Fixture file created at test-fixtures/main.py

      // @step And tree-sitter-python parser is installed
      // This will be verified when implementation is complete

      // @step When I run "fspec research --tool=ast --operation=list-functions --file=main.py"
      const result = await astTool.execute([
        '--operation',
        'list-functions',
        '--file',
        'test-fixtures/main.py',
      ]);

      const parsed = JSON.parse(result);

      // @step Then the output should contain a list of Python functions
      expect(parsed.matches).toBeDefined();
      expect(Array.isArray(parsed.matches)).toBe(true);

      // @step And the output should include function names and line numbers
      expect(parsed.matches.length).toBeGreaterThan(0);
      expect(parsed.matches[0]).toHaveProperty('line');
      expect(parsed.matches[0]).toHaveProperty('column');
    });
  });

  describe('Scenario: Parse Go file and list functions', () => {
    it('should parse Go file and return list of functions', async () => {
      // @step Given a Go file "main.go" exists with function definitions
      // @step And tree-sitter-go parser is installed
      // @step When I run "fspec research --tool=ast --operation=list-functions --file=main.go"
      const result = await astTool.execute([
        '--operation',
        'list-functions',
        '--file',
        'test-fixtures/main.go',
      ]);

      const parsed = JSON.parse(result);

      // @step Then the output should contain a list of Go functions
      expect(parsed.matches).toBeDefined();
      expect(Array.isArray(parsed.matches)).toBe(true);

      // @step And the output should include function names and line numbers
      expect(parsed.matches.length).toBeGreaterThan(0);
      expect(parsed.matches[0]).toHaveProperty('line');
    });
  });

  describe('Scenario: Parse Rust file and list functions', () => {
    it('should parse Rust file and return list of functions', async () => {
      // @step Given a Rust file "main.rs" exists with function definitions
      // @step And tree-sitter-rust parser is installed
      // @step When I run "fspec research --tool=ast --operation=list-functions --file=main.rs"
      const result = await astTool.execute([
        '--operation',
        'list-functions',
        '--file',
        'test-fixtures/main.rs',
      ]);

      const parsed = JSON.parse(result);

      // @step Then the output should contain a list of Rust functions
      expect(parsed.matches).toBeDefined();
      expect(Array.isArray(parsed.matches)).toBe(true);

      // @step And the output should include function names and line numbers
      expect(parsed.matches.length).toBeGreaterThan(0);
      expect(parsed.matches[0]).toHaveProperty('line');
    });
  });

  describe('Scenario: Parse Swift file and list functions', () => {
    it('should parse Swift file and return list of functions', async () => {
      // @step Given a Swift file "App.swift" exists with function definitions
      // @step And tree-sitter-swift parser is installed
      // @step When I run "fspec research --tool=ast --operation=list-functions --file=App.swift"
      const result = await astTool.execute([
        '--operation',
        'list-functions',
        '--file',
        'test-fixtures/App.swift',
      ]);

      const parsed = JSON.parse(result);

      // @step Then the output should contain a list of Swift functions
      expect(parsed.matches).toBeDefined();
      expect(Array.isArray(parsed.matches)).toBe(true);

      // @step And the output should include function names and line numbers
      expect(parsed.matches.length).toBeGreaterThan(0);
      expect(parsed.matches[0]).toHaveProperty('line');
    });
  });

  describe('Scenario: Parse C# file and list methods', () => {
    it('should parse C# file and return list of methods', async () => {
      // @step Given a C# file "Program.cs" exists with method definitions
      // @step And tree-sitter-c-sharp parser is installed
      // @step When I run "fspec research --tool=ast --operation=list-functions --file=Program.cs"
      const result = await astTool.execute([
        '--operation',
        'list-functions',
        '--file',
        'test-fixtures/Program.cs',
      ]);

      const parsed = JSON.parse(result);

      // @step Then the output should contain a list of C# methods
      expect(parsed.matches).toBeDefined();
      expect(Array.isArray(parsed.matches)).toBe(true);

      // @step And the output should include method names and line numbers
      expect(parsed.matches.length).toBeGreaterThan(0);
      expect(parsed.matches[0]).toHaveProperty('line');
    });
  });

  describe('Scenario: Parse C file and list functions', () => {
    it('should parse C file and return list of functions', async () => {
      // @step Given a C file "main.c" exists with function definitions
      // @step And tree-sitter-c parser is installed
      // @step When I run "fspec research --tool=ast --operation=list-functions --file=main.c"
      const result = await astTool.execute([
        '--operation',
        'list-functions',
        '--file',
        'test-fixtures/main.c',
      ]);

      const parsed = JSON.parse(result);

      // @step Then the output should contain a list of C functions
      expect(parsed.matches).toBeDefined();
      expect(Array.isArray(parsed.matches)).toBe(true);

      // @step And the output should include function names and line numbers
      expect(parsed.matches.length).toBeGreaterThan(0);
      expect(parsed.matches[0]).toHaveProperty('line');
    });
  });

  describe('Scenario: Parse C++ file and list functions', () => {
    it('should parse C++ file and return list of functions', async () => {
      // @step Given a C++ file "main.cpp" exists with function definitions
      // @step And tree-sitter-cpp parser is installed
      // @step When I run "fspec research --tool=ast --operation=list-functions --file=main.cpp"
      const result = await astTool.execute([
        '--operation',
        'list-functions',
        '--file',
        'test-fixtures/main.cpp',
      ]);

      const parsed = JSON.parse(result);

      // @step Then the output should contain a list of C++ functions
      expect(parsed.matches).toBeDefined();
      expect(Array.isArray(parsed.matches)).toBe(true);

      // @step And the output should include function names and line numbers
      expect(parsed.matches.length).toBeGreaterThan(0);
      expect(parsed.matches[0]).toHaveProperty('line');
    });
  });
});
