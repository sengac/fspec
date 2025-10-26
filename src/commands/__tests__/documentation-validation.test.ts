/**
 * Feature: spec/features/conversational-test-and-quality-check-tool-detection.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync, statSync } from 'fs';
import { join } from 'path';

describe('Feature: Conversational Test and Quality Check Tool Detection', () => {
  describe('Scenario: Validate all documentation uses dynamic command placeholders not hardcoded npm', () => {
    it('should pass when all examples use <test-command> or <quality-check-commands> placeholders', () => {
      // @step Given fspec has help files, slash command sections, project management sections, and CLAUDE.md documentation
      const filesToCheck = [
        // Help files
        ...findFiles('src/commands', /-help\.ts$/),

        // Slash command sections
        ...findFiles('src/utils/slashCommandSections', /\.ts$/),

        // Project management sections
        ...findFiles('src/utils/projectManagementSections', /\.ts$/),

        // System reminder utilities
        'src/utils/system-reminder.ts',
      ];

      // NOTE: docs/, README.md, CLAUDE.md, and spec/CLAUDE.md are EXCLUDED
      // - docs/ and README.md can have npm examples when discussing fspec project
      // - CLAUDE.md is fspec-specific development guide (can have npm examples for fspec development)
      // - spec/CLAUDE.md is auto-generated from slash command sections

      // @step When validation test scans all documentation files for hardcoded npm test, npm run build, npm check patterns
      const hardcodedPatterns = [
        /npm test(?![^\n]*<test-command>)/g, // 'npm test' not followed by <test-command> comment
        /npm run build/g,
        /npm run lint/g,
        /npm run typecheck/g,
        /npm run format/g,
        /npm check/g,
        /"npm test"/g,
        /"npm run/g,
        /'npm test'/g,
        /'npm run/g,
        /`npm test`/g,
        /`npm run/g,
      ];

      const violations: Array<{ file: string; line: number; match: string }> =
        [];

      for (const file of filesToCheck) {
        try {
          const content = readFileSync(file, 'utf-8');
          const lines = content.split('\n');

          lines.forEach((line, index) => {
            // Skip test files themselves
            if (file.includes('__tests__')) {
              return;
            }

            // Skip package.json scripts section
            if (file.includes('package.json')) {
              return;
            }

            for (const pattern of hardcodedPatterns) {
              const matches = line.match(pattern);
              if (matches) {
                violations.push({
                  file,
                  line: index + 1,
                  match: matches[0],
                });
              }
            }
          });
        } catch (error: unknown) {
          // File doesn't exist or can't be read - skip it
          if (
            error instanceof Error &&
            'code' in error &&
            error.code !== 'ENOENT'
          ) {
            throw error;
          }
        }
      }

      // @step Then test should pass when all examples use <test-command> or <quality-check-commands> placeholders
      // @step And test should fail if any npm test, npm run, or npm check hardcoded patterns found
      if (violations.length > 0) {
        const violationReport = violations
          .map(v => `  ${v.file}:${v.line} - "${v.match}"`)
          .join('\n');

        expect.fail(
          `Found ${violations.length} hardcoded npm command(s):\n${violationReport}\n\n` +
            `ALL documentation MUST use dynamic placeholders:\n` +
            `  - Use <test-command> instead of "npm test"\n` +
            `  - Use <quality-check-commands> instead of "npm run lint", "npm run typecheck", etc.\n` +
            `  - System-reminders will fill these in based on fspec configure-tools configuration`
        );
      }

      expect(violations).toHaveLength(0);
    });
  });
});

/**
 * Recursively find files matching a pattern
 */
function findFiles(dir: string, pattern: RegExp): string[] {
  const results: string[] = [];

  try {
    const entries = readdirSync(dir);

    for (const entry of entries) {
      const fullPath = join(dir, entry);
      const stat = statSync(fullPath);

      if (stat.isDirectory()) {
        results.push(...findFiles(fullPath, pattern));
      } else if (pattern.test(entry)) {
        results.push(fullPath);
      }
    }
  } catch (error: unknown) {
    // Directory doesn't exist - return empty array
    if (error instanceof Error && 'code' in error && error.code !== 'ENOENT') {
      throw error;
    }
  }

  return results;
}
