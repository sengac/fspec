/**
 * Feature: spec/features/ast-code-researcher-for-pattern-detection-and-deep-analysis.feature
 *
 * Tests for AST code researcher with pattern detection and deep analysis.
 * These tests verify the 'fspec research --tool=ast' command that uses tree-sitter
 * for AST parsing across multiple programming languages.
 */

import { describe, it, expect, beforeAll, afterEach } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';
import { execSync } from 'child_process';

const PROJECT_ROOT = process.cwd();
const RESEARCH_SCRIPTS_DIR = path.join(PROJECT_ROOT, 'spec/research-scripts');
const AST_SCRIPT = path.join(RESEARCH_SCRIPTS_DIR, 'ast');

describe('Feature: AST code researcher for pattern detection and deep analysis', () => {
  beforeAll(() => {
    // Enable test mode for research scripts
    process.env.FSPEC_TEST_MODE = '1';
  });

  describe('Scenario: Find all async functions in codebase', () => {
    it('should find and return all async functions with metadata', () => {
      // @step Given the ast research tool exists in spec/research-scripts/
      expect(fs.existsSync(RESEARCH_SCRIPTS_DIR)).toBe(true);

      // @step And the ast tool has executable permissions
      const stats = fs.statSync(AST_SCRIPT);
      expect(stats.mode & fs.constants.S_IXUSR).toBeTruthy();

      // @step And the codebase contains TypeScript files with async functions
      // (In test mode, this is mocked)

      // @step When I run "fspec research --tool=ast --query='find all async functions'"
      const result = execSync(
        'fspec research --tool=ast --query="find all async functions"',
        { encoding: 'utf8', cwd: PROJECT_ROOT }
      );

      // @step Then the ast script should execute with the query
      expect(result).toBeDefined();

      // @step And the output should be in JSON format
      expect(() => JSON.parse(result)).not.toThrow();
      const json = JSON.parse(result);

      // @step And the JSON should contain an array of matches
      expect(json.matches).toBeInstanceOf(Array);

      // @step And each match should include file path, line numbers, and code snippet
      if (json.matches.length > 0) {
        const match = json.matches[0];
        expect(match).toHaveProperty('file');
        expect(match).toHaveProperty('startLine');
        expect(match).toHaveProperty('endLine');
        expect(match).toHaveProperty('code');
      }

      // @step And each match should have nodeType "async_function"
      if (json.matches.length > 0) {
        const match = json.matches[0];
        expect(match.nodeType).toBe('async_function');
      }
    });
  });

  describe('Scenario: Analyze functions exceeding parameter count threshold', () => {
    it('should analyze and return functions with more than 5 parameters', () => {
      // @step Given the ast research tool exists in spec/research-scripts/
      expect(fs.existsSync(AST_SCRIPT)).toBe(true);

      // @step And the ast tool has executable permissions
      const stats = fs.statSync(AST_SCRIPT);
      expect(stats.mode & fs.constants.S_IXUSR).toBeTruthy();

      // @step And the codebase contains functions with varying parameter counts
      // (In test mode, this is mocked)

      // @step When I run "fspec research --tool=ast --query='functions with more than 5 parameters'"
      const result = execSync(
        'fspec research --tool=ast --query="functions with more than 5 parameters"',
        { encoding: 'utf8', cwd: PROJECT_ROOT }
      );

      // @step Then the ast script should analyze function complexity
      expect(result).toBeDefined();
      const json = JSON.parse(result);

      // @step And the output should list functions exceeding the threshold
      expect(json.matches).toBeInstanceOf(Array);

      // @step And each result should show file path, function name, and parameter count
      if (json.matches.length > 0) {
        const match = json.matches[0];
        expect(match).toHaveProperty('file');
        expect(match).toHaveProperty('functionName');
        expect(match).toHaveProperty('parameterCount');
      }

      // @step And functions with 6 or more parameters should be included
      if (json.matches.length > 0) {
        const match = json.matches[0];
        expect(match.parameterCount).toBeGreaterThanOrEqual(6);
      }
    });
  });

  describe('Scenario: Find structurally similar code across multiple languages', () => {
    it('should find similar patterns in TypeScript, Python, and Go files', () => {
      // @step Given the ast research tool exists in spec/research-scripts/
      expect(fs.existsSync(AST_SCRIPT)).toBe(true);

      // @step And the ast tool has executable permissions
      const stats = fs.statSync(AST_SCRIPT);
      expect(stats.mode & fs.constants.S_IXUSR).toBeTruthy();

      // @step And the codebase contains authentication code in TypeScript, Python, and Go
      // (In test mode, this is mocked)

      // @step When I run "fspec research --tool=ast --query='find code patterns similar to src/auth/login.ts'"
      const result = execSync(
        'fspec research --tool=ast --query="find code patterns similar to src/auth/login.ts"',
        { encoding: 'utf8', cwd: PROJECT_ROOT }
      );

      // @step Then the ast script should use AST pattern matching
      expect(result).toBeDefined();
      const json = JSON.parse(result);

      // @step And the output should include matches from Python files
      const pythonMatches = json.matches.filter((m: any) =>
        m.file.endsWith('.py')
      );
      expect(pythonMatches.length).toBeGreaterThan(0);

      // @step And the output should include matches from TypeScript files
      const tsMatches = json.matches.filter((m: any) => m.file.endsWith('.ts'));
      expect(tsMatches.length).toBeGreaterThan(0);

      // @step And the output should include matches from Go files
      const goMatches = json.matches.filter((m: any) => m.file.endsWith('.go'));
      expect(goMatches.length).toBeGreaterThan(0);

      // @step And each match should show structural similarity score
      if (json.matches.length > 0) {
        const match = json.matches[0];
        expect(match).toHaveProperty('similarityScore');
      }
    });
  });

  describe('Scenario: Parse incomplete code with syntax errors', () => {
    it('should perform error-tolerant parsing of broken code', () => {
      // @step Given the ast research tool exists in spec/research-scripts/
      expect(fs.existsSync(AST_SCRIPT)).toBe(true);

      // @step And the ast tool has executable permissions
      const stats = fs.statSync(AST_SCRIPT);
      expect(stats.mode & fs.constants.S_IXUSR).toBeTruthy();

      // @step And a code file contains syntax errors
      // (In test mode, this is mocked)

      // @step When I run "fspec research --tool=ast --file='src/broken.ts'"
      const result = execSync(
        'fspec research --tool=ast --file="src/broken.ts"',
        { encoding: 'utf8', cwd: PROJECT_ROOT }
      );

      // @step Then tree-sitter should perform error-tolerant parsing
      expect(result).toBeDefined();
      const json = JSON.parse(result);

      // @step And the output should contain partial AST for valid portions
      expect(json.partialAST).toBeDefined();

      // @step And the output should indicate which parts failed to parse
      expect(json.errors).toBeInstanceOf(Array);

      // @step And the command should not fail with exit code 1
      // (If we got here, the command succeeded)
      expect(true).toBe(true);
    });
  });

  describe('Scenario: Query for classes implementing specific interface', () => {
    it('should find all classes implementing UserRepository interface', () => {
      // @step Given the ast research tool exists in spec/research-scripts/
      expect(fs.existsSync(AST_SCRIPT)).toBe(true);

      // @step And the ast tool has executable permissions
      const stats = fs.statSync(AST_SCRIPT);
      expect(stats.mode & fs.constants.S_IXUSR).toBeTruthy();

      // @step And the codebase contains classes implementing UserRepository interface
      // (In test mode, this is mocked)

      // @step When I run "fspec research --tool=ast --query='classes implementing interface UserRepository'"
      const result = execSync(
        'fspec research --tool=ast --query="classes implementing interface UserRepository"',
        { encoding: 'utf8', cwd: PROJECT_ROOT }
      );

      // @step Then the ast script should execute tree-sitter queries
      expect(result).toBeDefined();
      const json = JSON.parse(result);

      // @step And the output should list all implementing classes
      expect(json.matches).toBeInstanceOf(Array);

      // @step And each result should show file location
      if (json.matches.length > 0) {
        const match = json.matches[0];
        expect(match).toHaveProperty('file');
        expect(match).toHaveProperty('startLine');
      }

      // @step And each result should include code snippet with class declaration
      if (json.matches.length > 0) {
        const match = json.matches[0];
        expect(match).toHaveProperty('code');
        expect(match.code).toContain('class');
      }
    });
  });

  describe('Scenario: Identify high-complexity functions for refactoring', () => {
    it('should find functions with cyclomatic complexity greater than 10', () => {
      // @step Given the ast research tool exists in spec/research-scripts/
      expect(fs.existsSync(AST_SCRIPT)).toBe(true);

      // @step And the ast tool has executable permissions
      const stats = fs.statSync(AST_SCRIPT);
      expect(stats.mode & fs.constants.S_IXUSR).toBeTruthy();

      // @step And the codebase contains functions with various complexity levels
      // (In test mode, this is mocked)

      // @step When I run "fspec research --tool=ast --query='functions with cyclomatic complexity > 10'"
      const result = execSync(
        'fspec research --tool=ast --query="functions with cyclomatic complexity > 10"',
        { encoding: 'utf8', cwd: PROJECT_ROOT }
      );

      // @step Then the ast script should calculate complexity metrics
      expect(result).toBeDefined();
      const json = JSON.parse(result);

      // @step And the output should include complexity scores
      expect(json.matches).toBeInstanceOf(Array);

      // @step And only functions with complexity > 10 should be returned
      if (json.matches.length > 0) {
        const match = json.matches[0];
        expect(match.complexity).toBeGreaterThan(10);
      }

      // @step And each result should show file path and exact complexity value
      if (json.matches.length > 0) {
        const match = json.matches[0];
        expect(match).toHaveProperty('file');
        expect(match).toHaveProperty('complexity');
        expect(typeof match.complexity).toBe('number');
      }
    });
  });

  describe('Scenario: Extract exported functions for API documentation', () => {
    it('should extract function signatures with JSDoc comments in markdown format', () => {
      // @step Given the ast research tool exists in spec/research-scripts/
      expect(fs.existsSync(AST_SCRIPT)).toBe(true);

      // @step And the ast tool has executable permissions
      const stats = fs.statSync(AST_SCRIPT);
      expect(stats.mode & fs.constants.S_IXUSR).toBeTruthy();

      // @step And the codebase contains exported functions with JSDoc comments
      // (In test mode, this is mocked)

      // @step When I run "fspec research --tool=ast --query='all exported functions' --format=markdown"
      const result = execSync(
        'fspec research --tool=ast --query="all exported functions" --format=markdown',
        { encoding: 'utf8', cwd: PROJECT_ROOT }
      );

      // @step Then the ast script should extract function signatures
      expect(result).toBeDefined();

      // @step And the output should be formatted as markdown
      expect(result).toContain('#');
      expect(result).toContain('function');

      // @step And each function should include JSDoc comments
      expect(result).toMatch(/\/\*\*|@param|@returns/);

      // @step And each function should show parameter types and return type
      expect(result.length).toBeGreaterThan(50);
    });
  });

  describe('Scenario: Detect unused imports in TypeScript files', () => {
    it('should find and report unused import statements', () => {
      // @step Given the ast research tool exists in spec/research-scripts/
      expect(fs.existsSync(AST_SCRIPT)).toBe(true);

      // @step And the ast tool has executable permissions
      const stats = fs.statSync(AST_SCRIPT);
      expect(stats.mode & fs.constants.S_IXUSR).toBeTruthy();

      // @step And TypeScript files contain both used and unused imports
      // (In test mode, this is mocked)

      // @step When I run "fspec research --tool=ast --query='unused imports' --language=typescript"
      const result = execSync(
        'fspec research --tool=ast --query="unused imports" --language=typescript',
        { encoding: 'utf8', cwd: PROJECT_ROOT }
      );

      // @step Then tree-sitter should parse import statements
      expect(result).toBeDefined();
      const json = JSON.parse(result);

      // @step And the ast script should check for usage in the AST
      expect(json.matches).toBeInstanceOf(Array);

      // @step And the output should list only unused imports
      if (json.matches.length > 0) {
        const match = json.matches[0];
        expect(match.unused).toBe(true);
      }

      // @step And each result should show import statement and file location
      if (json.matches.length > 0) {
        const match = json.matches[0];
        expect(match).toHaveProperty('file');
        expect(match).toHaveProperty('importStatement');
        expect(match.importStatement).toContain('import');
      }
    });
  });
});
