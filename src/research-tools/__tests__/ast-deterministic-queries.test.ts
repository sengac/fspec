/**
 * Feature: spec/features/refactor-ast-tool-to-use-deterministic-tree-sitter-queries-instead-of-semantic-natural-language-parsing.feature
 *
 * Tests for deterministic tree-sitter query operations replacing semantic natural language parsing
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { tool as astTool } from '../ast';
import type { QueryMatch } from '../../utils/query-executor';
import * as fs from 'fs/promises';
import * as path from 'path';
import { tmpdir } from 'os';

describe('Feature: Refactor AST tool to use deterministic tree-sitter queries', () => {
  let testDir: string;
  let authFile: string;
  let apiFile: string;
  let utilsFile: string;
  let constantsFile: string;
  let indexFile: string;

  beforeEach(async () => {
    // Create temporary test directory
    testDir = await fs.mkdtemp(path.join(tmpdir(), 'ast-test-'));
    authFile = path.join(testDir, 'auth.ts');
    apiFile = path.join(testDir, 'api.ts');
    utilsFile = path.join(testDir, 'utils.ts');
    constantsFile = path.join(testDir, 'constants.ts');
    indexFile = path.join(testDir, 'index.ts');
  });

  afterEach(async () => {
    // Cleanup test directory
    await fs.rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: List all functions using deterministic operation flag', () => {
    it('should list all function types using --operation=list-functions', async () => {
      // @step Given I have a TypeScript file "src/auth.ts" with function declarations, expressions, and arrow functions
      const authCode = `
export function login(username: string, password: string) {
  return { token: 'abc' };
}

export const logout = function() {
  console.log('logged out');
};

export const validateToken = (token: string) => {
  return token.length > 0;
};

export class AuthService {
  authenticate() {
    return true;
  }
}

export function* generateTokens() {
  yield 'token1';
  yield 'token2';
}
`;
      await fs.writeFile(authFile, authCode);

      // @step When I run "fspec research --tool=ast --operation=list-functions --file=src/auth.ts"
      const result = await astTool.execute([
        '--operation',
        'list-functions',
        '--file',
        authFile,
      ]);

      // @step Then the output should be valid JSON
      const parsed = JSON.parse(result);
      expect(parsed).toBeDefined();

      // @step And the JSON should contain an array of function matches
      expect(parsed.matches).toBeInstanceOf(Array);
      expect(parsed.matches.length).toBeGreaterThan(0);

      // @step And each match should include function type (declaration, expression, arrow, method, generator)
      const functionTypes = parsed.matches.map((m: QueryMatch) => m.type);
      expect(functionTypes).toContain('function_declaration');
      expect(functionTypes).toContain('arrow_function');

      // @step And each match should include line numbers and function names
      parsed.matches.forEach((match: QueryMatch) => {
        expect(match).toHaveProperty('line');
        expect(match.line).toBeGreaterThan(0);
      });
    });
  });

  describe('Scenario: Find specific class by name', () => {
    it('should find a class by exact name using --operation=find-class --name=ClassName', async () => {
      // @step Given I have a TypeScript file "src/auth.ts" containing class "AuthController"
      const authCode = `
export class UserService {
  getUser() {}
}

export class AuthController {
  constructor(private service: UserService) {}

  login() {
    return this.service.getUser();
  }
}

export class SessionManager {
  create() {}
}
`;
      await fs.writeFile(authFile, authCode);

      // @step When I run "fspec research --tool=ast --operation=find-class --name=AuthController --file=src/auth.ts"
      const result = await astTool.execute([
        '--operation',
        'find-class',
        '--name',
        'AuthController',
        '--file',
        authFile,
      ]);

      // @step Then the output should be valid JSON
      const parsed = JSON.parse(result);
      expect(parsed).toBeDefined();

      // @step And the JSON should contain the class definition
      expect(parsed.match).toBeDefined();
      expect(parsed.match.name).toBe('AuthController');

      // @step And the class definition should include line numbers
      expect(parsed.match.line).toBeGreaterThan(0);

      // @step And the class definition should include the class body structure
      expect(parsed.match).toHaveProperty('body');
    });
  });

  describe('Scenario: Find functions with parametric predicate filter', () => {
    it('should find functions with parameter count >= threshold', async () => {
      // @step Given I have a TypeScript file "src/api.ts" with functions of varying parameter counts
      const apiCode = `
export function simple() {}

export function withTwo(a: string, b: number) {}

export function withFive(a: string, b: number, c: boolean, d: object, e: string[]) {
  return { a, b, c, d, e };
}

export function withSix(p1: any, p2: any, p3: any, p4: any, p5: any, p6: any) {
  return [p1, p2, p3, p4, p5, p6];
}

export function withEight(
  arg1: string,
  arg2: number,
  arg3: boolean,
  arg4: object,
  arg5: string[],
  arg6: number[],
  arg7: Date,
  arg8: RegExp
) {
  return true;
}
`;
      await fs.writeFile(apiFile, apiCode);

      // @step When I run "fspec research --tool=ast --operation=find-functions --min-params=5 --file=src/api.ts"
      const result = await astTool.execute([
        '--operation',
        'find-functions',
        '--min-params',
        '5',
        '--file',
        apiFile,
      ]);

      // @step Then the output should be valid JSON
      const parsed = JSON.parse(result);
      expect(parsed).toBeDefined();

      // @step And the JSON should contain only functions with 5 or more parameters
      expect(parsed.matches).toBeInstanceOf(Array);
      expect(parsed.matches.length).toBe(3); // withFive, withSix, withEight

      // @step And each match should include the parameter count
      parsed.matches.forEach((match: QueryMatch) => {
        expect(match.paramCount).toBeGreaterThanOrEqual(5);
      });

      // @step And each match should include line numbers
      parsed.matches.forEach((match: QueryMatch) => {
        expect(match.line).toBeGreaterThan(0);
      });
    });
  });

  describe('Scenario: Execute custom tree-sitter query from file', () => {
    it('should load and execute custom .scm query file', async () => {
      // @step Given I have a custom query file "queries/custom-pattern.scm" with tree-sitter S-expression query
      const queriesDir = path.join(testDir, 'queries');
      await fs.mkdir(queriesDir, { recursive: true });
      const customQueryFile = path.join(queriesDir, 'custom-pattern.scm');
      const customQuery = `
(function_declaration
  name: (identifier) @function-name)
`;
      await fs.writeFile(customQueryFile, customQuery);

      // @step And I have a TypeScript file "src/utils.ts"
      const utilsCode = `
export function helperA() {}
export function helperB() {}
export const notAFunction = 123;
`;
      await fs.writeFile(utilsFile, utilsCode);

      // @step When I run "fspec research --tool=ast --query-file=queries/custom-pattern.scm --file=src/utils.ts"
      const result = await astTool.execute([
        '--query-file',
        customQueryFile,
        '--file',
        utilsFile,
      ]);

      // @step Then the tool should load the .scm file content
      // @step And the tool should execute the custom tree-sitter query
      const parsed = JSON.parse(result);
      expect(parsed).toBeDefined();

      // @step And the output should be valid JSON with matches based on the custom query
      expect(parsed.matches).toBeInstanceOf(Array);
      expect(parsed.matches.length).toBeGreaterThan(0);
    });
  });

  describe('Scenario: Find identifiers matching pattern', () => {
    it('should find identifiers matching regex pattern', async () => {
      // @step Given I have a TypeScript file "src/constants.ts" with CONSTANT_CASE and camelCase identifiers
      const constantsCode = `
export const MAX_RETRIES = 3;
export const API_BASE_URL = 'https://example.com';
export const DEFAULT_TIMEOUT = 5000;

export const userName = 'john';
export const userId = 123;

export const ENVIRONMENT_PROD = 'production';
`;
      await fs.writeFile(constantsFile, constantsCode);

      // @step When I run "fspec research --tool=ast --operation=find-identifiers --pattern="^[A-Z][A-Z_]+" --file=src/constants.ts"
      const result = await astTool.execute([
        '--operation',
        'find-identifiers',
        '--pattern',
        '^[A-Z][A-Z_]+',
        '--file',
        constantsFile,
      ]);

      // @step Then the output should be valid JSON
      const parsed = JSON.parse(result);
      expect(parsed).toBeDefined();

      // @step And the JSON should contain only CONSTANT_CASE identifiers
      expect(parsed.matches).toBeInstanceOf(Array);
      const identifierNames = parsed.matches.map((m: QueryMatch) => m.name);
      expect(identifierNames).toContain('MAX_RETRIES');
      expect(identifierNames).toContain('API_BASE_URL');
      expect(identifierNames).toContain('DEFAULT_TIMEOUT');
      expect(identifierNames).toContain('ENVIRONMENT_PROD');
      expect(identifierNames).not.toContain('userName');
      expect(identifierNames).not.toContain('userId');

      // @step And each match should include the identifier name and line number
      parsed.matches.forEach((match: QueryMatch) => {
        expect(match).toHaveProperty('name');
        expect(match).toHaveProperty('line');
        expect(match.line).toBeGreaterThan(0);
      });
    });
  });

  describe('Scenario: Find async functions', () => {
    it('should find only async function declarations', async () => {
      // @step Given I have a TypeScript file "src/api.ts" with both async and sync function declarations
      const apiCode = `
export function syncFunction() {
  return 'sync';
}

export async function asyncFetchUser(id: string) {
  const response = await fetch(\`/users/\${id}\`);
  return response.json();
}

export function anotherSync() {
  return 42;
}

export async function asyncSaveData(data: any) {
  await save(data);
  return true;
}
`;
      await fs.writeFile(apiFile, apiCode);

      // @step When I run "fspec research --tool=ast --operation=find-async-functions --file=src/api.ts"
      const result = await astTool.execute([
        '--operation',
        'find-async-functions',
        '--file',
        apiFile,
      ]);

      // @step Then the output should be valid JSON
      const parsed = JSON.parse(result);
      expect(parsed).toBeDefined();

      // @step And the JSON should contain only async function declarations
      expect(parsed.matches).toBeInstanceOf(Array);
      expect(parsed.matches.length).toBe(2);
      const functionNames = parsed.matches.map((m: QueryMatch) => m.name);
      expect(functionNames).toContain('asyncFetchUser');
      expect(functionNames).toContain('asyncSaveData');
      expect(functionNames).not.toContain('syncFunction');
      expect(functionNames).not.toContain('anotherSync');

      // @step And each match should include the function name and line numbers
      parsed.matches.forEach((match: QueryMatch) => {
        expect(match).toHaveProperty('name');
        expect(match).toHaveProperty('line');
        expect(match.line).toBeGreaterThan(0);
      });
    });
  });

  describe('Scenario: Find exports by type', () => {
    it('should find exports filtered by export type', async () => {
      // @step Given I have a TypeScript file "src/index.ts" with default and named exports
      const indexCode = `
export const namedExport1 = 'value1';
export const namedExport2 = 'value2';

export default class DefaultExport {
  constructor() {}
}

export function namedFunction() {}
`;
      await fs.writeFile(indexFile, indexCode);

      // @step When I run "fspec research --tool=ast --operation=find-exports --export-type=default --file=src/index.ts"
      const result = await astTool.execute([
        '--operation',
        'find-exports',
        '--export-type',
        'default',
        '--file',
        indexFile,
      ]);

      // @step Then the output should be valid JSON
      const parsed = JSON.parse(result);
      expect(parsed).toBeDefined();

      // @step And the JSON should contain only the default export statement
      expect(parsed.match).toBeDefined();
      expect(parsed.match.exportType).toBe('default');

      // @step And the match should include line numbers and exported identifier
      expect(parsed.match).toHaveProperty('line');
      expect(parsed.match.line).toBeGreaterThan(0);
      expect(parsed.match).toHaveProperty('identifier');
    });
  });
});
