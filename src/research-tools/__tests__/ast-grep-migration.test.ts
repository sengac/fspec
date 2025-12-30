// Feature: spec/features/migrate-fspec-research-ast-tool-from-tree-sitter-to-codelet-ast-grep.feature
//
// Tests for ast research tool CLI integration

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFileSync, mkdirSync, rmSync, readFileSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { tool } from '../ast';

describe('AST Research Tool', () => {
  let testDir: string;

  beforeEach(() => {
    testDir = join(tmpdir(), `ast-test-${Date.now()}`);
    mkdirSync(testDir, { recursive: true });
  });

  afterEach(() => {
    rmSync(testDir, { recursive: true, force: true });
  });

  // Scenario: Search for functions using pattern matching
  it('should search for functions using pattern matching', async () => {
    // @step Given a TypeScript file with function declarations
    const filePath = join(testDir, 'test.ts');
    writeFileSync(
      filePath,
      `
function hello() { return "world"; }
function greet(name: string) { return \`Hello, \${name}\`; }
const arrow = () => {};
    `
    );

    // @step When I run ast search with pattern 'function $NAME($$$ARGS)' and language 'typescript'
    const output = await tool.execute([
      '--pattern=function $NAME($$$ARGS) { $$$BODY }',
      '--lang=typescript',
      `--path=${filePath}`,
    ]);

    // @step Then the output contains matches in file:line:column:text format
    expect(output).toContain(':'); // file:line:column format
    expect(output).toContain('function');
    // Verify at least 2 matches (hello and greet)
    const lines = output
      .trim()
      .split('\n')
      .filter(l => l.includes('function'));
    expect(lines.length).toBeGreaterThanOrEqual(2);
  });

  // Scenario: Refactor moves matched code to new file
  it('should refactor and move matched code to new file', async () => {
    // @step Given a source file containing 'const SafeTextInput' component
    const sourcePath = join(testDir, 'source.tsx');
    const targetPath = join(testDir, 'SafeTextInput.tsx');

    writeFileSync(
      sourcePath,
      `import React from 'react';

const OtherComponent = () => <div>Other</div>;

const SafeTextInput = () => {
    return <input type="text" />;
};

export default OtherComponent;
`
    );

    // @step When I run ast refactor with pattern 'const SafeTextInput' from source to target file
    await tool.execute([
      '--refactor',
      '--pattern=const SafeTextInput',
      '--lang=tsx',
      `--source=${sourcePath}`,
      `--target=${targetPath}`,
    ]);

    // @step Then the matched code is removed from the source file
    const updatedSource = readFileSync(sourcePath, 'utf-8');
    expect(updatedSource).not.toContain('const SafeTextInput');
    expect(updatedSource).toContain('const OtherComponent');

    // @step And the matched code is written to the target file
    const targetContent = readFileSync(targetPath, 'utf-8');
    expect(targetContent).toContain('const SafeTextInput');
  });

  // Scenario: Refactor errors when pattern matches multiple nodes
  it('should error when refactor pattern matches multiple nodes', async () => {
    // @step Given a source file with multiple const declarations
    const sourcePath = join(testDir, 'multi.tsx');
    const targetPath = join(testDir, 'target.tsx');

    writeFileSync(
      sourcePath,
      `const Alpha = () => <div>A</div>;
const Beta = () => <div>B</div>;
const Gamma = () => <div>C</div>;
const Delta = () => <div>D</div>;
const Epsilon = () => <div>E</div>;
`
    );

    // @step When I run ast refactor with pattern 'const $NAME' that matches 5 nodes
    let errorMessage = '';
    try {
      await tool.execute([
        '--refactor',
        '--pattern=const $NAME',
        '--lang=tsx',
        `--source=${sourcePath}`,
        `--target=${targetPath}`,
      ]);
    } catch (error: unknown) {
      errorMessage = error instanceof Error ? error.message : String(error);
    }

    // @step Then an error is returned stating 'Pattern matched 5 nodes. Refactor requires exactly 1 match.'
    expect(errorMessage).toContain('Pattern matched');
    expect(errorMessage).toContain('Refactor requires exactly 1 match');

    // @step And the error lists all match locations
    expect(errorMessage).toMatch(/Alpha|Beta|Gamma|line/i);
  });

  // Scenario: NAPI exports astGrepSearch function
  it('should have astGrepSearch available as NAPI export', async () => {
    // @step Given the codelet-napi module is loaded
    const { astGrepSearch } = await import('@sengac/codelet-napi');
    expect(astGrepSearch).toBeDefined();
    expect(typeof astGrepSearch).toBe('function');

    // @step When I call astGrepSearch with pattern, language, and paths
    const filePath = join(testDir, 'test.ts');
    writeFileSync(filePath, 'function test() { return 42; }');

    const results = await astGrepSearch('function $NAME', 'typescript', [
      filePath,
    ]);

    // @step Then the function returns an array of match results
    expect(Array.isArray(results)).toBe(true);
    expect(results.length).toBeGreaterThan(0);
  });

  // Scenario: NAPI exports astGrepRefactor function
  it('should have astGrepRefactor available as NAPI export', async () => {
    // @step Given the codelet-napi module is loaded
    const { astGrepRefactor } = await import('@sengac/codelet-napi');
    expect(astGrepRefactor).toBeDefined();
    expect(typeof astGrepRefactor).toBe('function');

    // @step When I call astGrepRefactor with pattern, language, source file, and target file
    const sourcePath = join(testDir, 'source.ts');
    const targetPath = join(testDir, 'target.ts');
    writeFileSync(
      sourcePath,
      'function hello() { return "hello"; }\nfunction other() {}'
    );

    await astGrepRefactor(
      'function hello',
      'typescript',
      sourcePath,
      targetPath
    );

    // @step Then the function moves the matched code from source to target
    const sourceContent = readFileSync(sourcePath, 'utf-8');
    const targetContent = readFileSync(targetPath, 'utf-8');

    expect(sourceContent).not.toContain('function hello');
    expect(targetContent).toContain('function hello');
  });

  // Scenario: Tree-sitter dependencies are removed
  it('should have no tree-sitter dependencies in package.json', () => {
    // @step Given the package.json file
    const packageJson = readFileSync(
      join(process.cwd(), 'package.json'),
      'utf-8'
    );

    // @step When the migration is complete
    // (verified by this test running after migration)

    // @step Then no @sengac/tree-sitter packages are listed as dependencies
    expect(packageJson).not.toContain('@sengac/tree-sitter');
  });
});
