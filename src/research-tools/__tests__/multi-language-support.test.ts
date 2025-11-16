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

  describe('Scenario: Parse Java file and list methods', () => {
    it('should parse Java file and return list of methods', async () => {
      // @step Given a Java file "Example.java" exists with method definitions
      // Fixture file created at test-fixtures/Example.java

      // @step And tree-sitter-java parser is installed
      // This will be verified when implementation is complete

      // @step When I run "fspec research --tool=ast --operation=list-functions --file=Example.java"
      const result = await astTool.execute([
        '--operation',
        'list-functions',
        '--file',
        'test-fixtures/Example.java',
      ]);

      const parsed = JSON.parse(result);

      // @step Then the output should contain a list of Java methods
      expect(parsed.matches).toBeDefined();
      expect(Array.isArray(parsed.matches)).toBe(true);

      // @step And the output should include method names and line numbers
      expect(parsed.matches.length).toBeGreaterThan(0);
      expect(parsed.matches[0]).toHaveProperty('line');
    });
  });

  describe('Scenario: Parse PHP file and list functions', () => {
    it('should parse PHP file and return list of functions', async () => {
      // @step Given a PHP file "example.php" exists with function definitions
      // Fixture file created at test-fixtures/example.php

      // @step And tree-sitter-php parser is installed
      // This will be verified when implementation is complete

      // @step When I run "fspec research --tool=ast --operation=list-functions --file=example.php"
      const result = await astTool.execute([
        '--operation',
        'list-functions',
        '--file',
        'test-fixtures/example.php',
      ]);

      const parsed = JSON.parse(result);

      // @step Then the output should contain a list of PHP functions
      expect(parsed.matches).toBeDefined();
      expect(Array.isArray(parsed.matches)).toBe(true);

      // @step And the output should include function names and line numbers
      expect(parsed.matches.length).toBeGreaterThan(0);
      expect(parsed.matches[0]).toHaveProperty('line');
    });
  });

  describe('Scenario: Parse Ruby file and list methods', () => {
    it('should parse Ruby file and return list of methods', async () => {
      // @step Given a Ruby file "example.rb" exists with method definitions
      // Fixture file created at test-fixtures/example.rb

      // @step And tree-sitter-ruby parser is installed
      // This will be verified when implementation is complete

      // @step When I run "fspec research --tool=ast --operation=list-functions --file=example.rb"
      const result = await astTool.execute([
        '--operation',
        'list-functions',
        '--file',
        'test-fixtures/example.rb',
      ]);

      const parsed = JSON.parse(result);

      // @step Then the output should contain a list of Ruby methods
      expect(parsed.matches).toBeDefined();
      expect(Array.isArray(parsed.matches)).toBe(true);

      // @step And the output should include method names and line numbers
      expect(parsed.matches.length).toBeGreaterThan(0);
      expect(parsed.matches[0]).toHaveProperty('line');
    });
  });

  describe('Scenario: Parse Bash script and list functions', () => {
    it('should parse Bash script and return list of functions', async () => {
      // @step Given a Bash script "script.sh" exists with function definitions
      // Fixture file created at test-fixtures/script.sh

      // @step And tree-sitter-bash parser is installed
      // This will be verified when implementation is complete

      // @step When I run "fspec research --tool=ast --operation=list-functions --file=script.sh"
      const result = await astTool.execute([
        '--operation',
        'list-functions',
        '--file',
        'test-fixtures/script.sh',
      ]);

      const parsed = JSON.parse(result);

      // @step Then the output should contain a list of Bash functions
      expect(parsed.matches).toBeDefined();
      expect(Array.isArray(parsed.matches)).toBe(true);

      // @step And the output should include function names and line numbers
      expect(parsed.matches.length).toBeGreaterThan(0);
      expect(parsed.matches[0]).toHaveProperty('line');
    });
  });

  describe('Scenario: AST tool help documents all 16 languages', () => {
    it('should list all 16 supported languages in help output', async () => {
      // @step When I run "fspec research --tool=ast --help"
      const helpConfig = astTool.getHelpConfig();
      const helpText = JSON.stringify(helpConfig);

      // @step Then the output should list all 16 supported languages
      expect(helpText).toContain('16');
      expect(helpText).toContain('languages');

      // @step And the language list should include JavaScript, TypeScript, Python, Go, Rust, Kotlin, Dart, Swift, C#, C, C++, Java, PHP, Ruby, Bash, JSON
      expect(helpText).toContain('JavaScript');
      expect(helpText).toContain('TypeScript');
      expect(helpText).toContain('Python');
      expect(helpText).toContain('Go');
      expect(helpText).toContain('Rust');
      expect(helpText).toContain('Kotlin');
      expect(helpText).toContain('Dart');
      expect(helpText).toContain('Swift');
      expect(helpText).toContain('C#');
      expect(helpText).toContain('C++');
      expect(helpText).toContain('Java');
      expect(helpText).toContain('PHP');
      expect(helpText).toContain('Ruby');
      expect(helpText).toContain('Bash');
      expect(helpText).toContain('JSON');
    });
  });

  describe('Scenario: Research command lists AST tool with correct capabilities', () => {
    it('should list AST tool with multi-language support', async () => {
      // @step When I run "fspec research"
      // This requires testing the main research command which lists all tools
      // For now, we test that the AST tool exists and has correct metadata

      // @step Then the output should list the AST research tool
      expect(astTool).toBeDefined();
      expect(astTool.name).toBe('ast');

      // @step And the AST tool description should mention multi-language support
      expect(astTool.description).toContain('language');

      // @step And the tool should indicate 15 languages are supported
      expect(astTool.description).toContain('15');
    });
  });

  describe('Scenario: Help file documents all research tools', () => {
    it('should contain documentation for all research tools in src/help.ts', async () => {
      const fs = await import('fs/promises');
      const path = await import('path');

      // @step Given I check the file "src/help.ts"
      const helpFilePath = path.join(process.cwd(), 'src', 'help.ts');
      const helpContent = await fs.readFile(helpFilePath, 'utf-8');

      // @step Then it should contain documentation for all research tools
      expect(helpContent).toContain('research');

      // @step And it should list the AST tool with complete language support information
      expect(helpContent).toContain('ast');
      expect(helpContent).toContain('15');

      // @step And it should include usage examples for research tools
      expect(helpContent).toContain('fspec research');
    });
  });
});
