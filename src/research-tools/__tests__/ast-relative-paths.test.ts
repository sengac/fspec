// Feature: spec/features/ast-research-tool-fails-with-relative-file-paths.feature

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { tool as astTool } from '../ast';
import { join } from 'path';
import { writeFile, mkdir, rm } from 'fs/promises';
import { tmpdir } from 'os';

describe('Feature: AST research tool fails with relative file paths', () => {
  let tmpDir: string;
  let originalCwd: string;

  beforeEach(async () => {
    // Create temp directory
    tmpDir = join(tmpdir(), `ast-test-${Date.now()}`);
    await mkdir(tmpDir, { recursive: true });

    // Create subdirectory structure
    await mkdir(join(tmpDir, 'src'), { recursive: true });

    // Create test file
    const testContent = `
function testFunction() {
  return 'hello';
}

export function anotherFunction() {
  return 'world';
}
`;
    await writeFile(join(tmpDir, 'src', 'index.ts'), testContent);

    // Change to temp directory
    originalCwd = process.cwd();
    process.chdir(tmpDir);
  });

  afterEach(async () => {
    // Restore original directory
    process.chdir(originalCwd);

    // Cleanup
    await rm(tmpDir, { recursive: true, force: true });
  });

  describe('Scenario: AST research tool resolves relative file paths', () => {
    it('should resolve relative paths from current working directory', async () => {
      // @step Given I am in the project root directory
      // (already set up in beforeEach - process.chdir(tmpDir))

      // @step And the file "src/index.ts" exists
      // (already created in beforeEach)

      // @step When I run "fspec research --tool=ast --operation=list-functions --file=src/index.ts"
      const result = await astTool.execute([
        '--operation',
        'list-functions',
        '--file',
        'src/index.ts',
      ]);

      // @step Then the command should succeed
      expect(result).toBeDefined();

      // @step And the output should contain function names from the file
      const parsed = JSON.parse(result);
      expect(parsed.matches).toBeDefined();
      expect(parsed.matches.length).toBeGreaterThan(0);
      expect(JSON.stringify(parsed)).toContain('testFunction');

      // @step And no "ENOENT" or "file not found" errors should occur
      // (if there were errors, the above would have thrown)
    });
  });

  describe('Scenario: AST research tool works with absolute file paths', () => {
    it('should work with absolute file paths', async () => {
      // @step Given I am in the project root directory
      // (already set up in beforeEach)

      // @step And the file "src/index.ts" exists
      // (already created in beforeEach)

      const absolutePath = join(tmpDir, 'src', 'index.ts');

      // @step When I run "fspec research --tool=ast --operation=list-functions --file=/full/path/src/index.ts"
      const result = await astTool.execute([
        '--operation',
        'list-functions',
        '--file',
        absolutePath,
      ]);

      // @step Then the command should succeed
      expect(result).toBeDefined();

      // @step And the output should contain function names from the file
      const parsed = JSON.parse(result);
      expect(parsed.matches).toBeDefined();
      expect(parsed.matches.length).toBeGreaterThan(0);

      // @step And the behavior should be identical to using relative paths
      const relativeResult = await astTool.execute([
        '--operation',
        'list-functions',
        '--file',
        'src/index.ts',
      ]);
      expect(result).toEqual(relativeResult);
    });
  });
});
