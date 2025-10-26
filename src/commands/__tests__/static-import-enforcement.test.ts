/**
 * Feature: spec/features/auto-checkpoints-not-working-lazy-import-fails-in-bundled-dist.feature
 *
 * This test file enforces static imports to prevent Vite bundler optimization bugs.
 * Dynamic imports (await import()) break in bundled distribution.
 */

import { describe, it, expect } from 'vitest';
import { readFile } from 'fs/promises';
import { join } from 'path';

describe('Feature: Auto-checkpoints not working - lazy import fails in bundled dist', () => {
  describe('Scenario: Static imports enforced in update-work-unit-status.ts', () => {
    it('should NOT find any dynamic import patterns (await import)', async () => {
      // @step Given the source file 'src/commands/update-work-unit-status.ts'
      const filePath = join(
        process.cwd(),
        'src/commands/update-work-unit-status.ts'
      );
      const content = await readFile(filePath, 'utf-8');

      // @step When a static analysis test scans the file for dynamic imports
      const hasDynamicImport = content.includes('await import(');

      // @step Then the test should NOT find any 'await import(' patterns
      expect(hasDynamicImport).toBe(false);

      if (hasDynamicImport) {
        // Extract the line for better error message
        const lines = content.split('\n');
        const problematicLines = lines
          .map((line, index) => ({ line, index: index + 1 }))
          .filter((item) => item.line.includes('await import('))
          .map((item) => `  Line ${item.index}: ${item.line.trim()}`)
          .join('\n');

        throw new Error(
          `Dynamic imports found in update-work-unit-status.ts:\n${problematicLines}\n\n` +
            `CRITICAL: Dynamic imports break in Vite bundled distribution!\n` +
            `Use static imports at top of file instead: import * as gitCheckpoint from '../utils/git-checkpoint';`
        );
      }
    });

    it('should verify git-checkpoint is imported statically at top of file', async () => {
      // @step Given the source file 'src/commands/update-work-unit-status.ts'
      const filePath = join(
        process.cwd(),
        'src/commands/update-work-unit-status.ts'
      );
      const content = await readFile(filePath, 'utf-8');

      // @step When a static analysis test scans the file for dynamic imports
      const lines = content.split('\n');

      // Find all import statements (should be at top of file)
      const importLines = lines
        .slice(0, 50) // Check first 50 lines for imports
        .filter((line) => line.trim().startsWith('import '));

      // @step And the test should verify git-checkpoint is imported statically at top of file
      const hasStaticGitCheckpointImport = importLines.some((line) =>
        line.includes('git-checkpoint')
      );

      expect(hasStaticGitCheckpointImport).toBe(true);

      if (!hasStaticGitCheckpointImport) {
        throw new Error(
          `Missing static import of git-checkpoint in update-work-unit-status.ts!\n\n` +
            `Expected: import * as gitCheckpoint from '../utils/git-checkpoint';\n\n` +
            `Found imports:\n${importLines.map((l) => `  ${l.trim()}`).join('\n')}`
        );
      }

      // Verify the import uses namespace import (* as)
      const gitCheckpointImport = importLines.find((line) =>
        line.includes('git-checkpoint')
      );

      expect(gitCheckpointImport).toMatch(/import \* as \w+ from/);

      if (gitCheckpointImport && !gitCheckpointImport.match(/import \* as \w+ from/)) {
        throw new Error(
          `git-checkpoint import should use namespace syntax!\n\n` +
            `Expected: import * as gitCheckpoint from '../utils/git-checkpoint';\n` +
            `Found: ${gitCheckpointImport.trim()}`
        );
      }
    });
  });
});
