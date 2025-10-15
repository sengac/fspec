/**
 * Test suite for: spec/features/automatic-json-file-initialization.feature
 * Scenario: All 48+ commands use ensure utilities
 */

import { describe, it, expect } from 'vitest';
import { readFile } from 'fs/promises';
import { glob } from 'tinyglobby';
import { join } from 'path';

describe('Feature: Automatic JSON File Initialization', () => {
  describe('Scenario: All 48+ commands use ensure utilities', () => {
    it('should ensure all commands use ensure utilities instead of direct file reads', async () => {
      // Given I have analyzed all commands in src/commands/
      const commandFiles = await glob('src/commands/*.ts', {
        absolute: true,
        cwd: join(process.cwd()),
      });

      // Filter out test files and non-command files
      const actualCommands = commandFiles.filter(
        file =>
          !file.includes('__tests__') &&
          !file.includes('.test.ts') &&
          !file.includes('.spec.ts') &&
          !file.endsWith('index.ts')
      );

      // When I check which commands read JSON files
      const violations: Array<{
        file: string;
        issue: string;
        line: number;
      }> = [];

      for (const commandFile of actualCommands) {
        const content = await readFile(commandFile, 'utf-8');
        const lines = content.split('\n');

        // Check if command uses ensure utilities
        const usesEnsureUtils =
          content.includes('ensureWorkUnitsFile') ||
          content.includes('ensureEpicsFile') ||
          content.includes('ensurePrefixesFile') ||
          content.includes('ensureTagsFile') ||
          content.includes('ensureFoundationFile');

        // Check for direct readFile usage on JSON data files
        lines.forEach((line, index) => {
          const lineNum = index + 1;

          // Look for direct readFile calls on work-units.json
          if (
            line.includes('readFile') &&
            line.includes('work-units.json') &&
            !usesEnsureUtils
          ) {
            violations.push({
              file: commandFile,
              issue:
                'Directly reads work-units.json without using ensureWorkUnitsFile',
              line: lineNum,
            });
          }

          // Look for direct readFile calls on epics.json
          if (
            line.includes('readFile') &&
            line.includes('epics.json') &&
            !usesEnsureUtils
          ) {
            violations.push({
              file: commandFile,
              issue: 'Directly reads epics.json without using ensureEpicsFile',
              line: lineNum,
            });
          }

          // Look for direct readFile calls on prefixes.json
          if (
            line.includes('readFile') &&
            line.includes('prefixes.json') &&
            !usesEnsureUtils
          ) {
            violations.push({
              file: commandFile,
              issue:
                'Directly reads prefixes.json without using ensurePrefixesFile',
              line: lineNum,
            });
          }
        });
      }

      // Then ALL commands should import from ensure-files
      // And NO commands should directly readFile work-units.json without ensure
      // And NO commands should directly readFile epics.json without ensure
      // And NO commands should directly readFile prefixes.json without ensure

      if (violations.length > 0) {
        const violationReport = violations
          .map(
            v =>
              `  ${v.file.replace(process.cwd(), '.')}:${v.line} - ${v.issue}`
          )
          .join('\n');

        expect.fail(
          `\n❌ Found ${violations.length} command(s) not using ensure utilities:\n${violationReport}\n\nAll commands MUST use ensureWorkUnitsFile, ensureEpicsFile, or ensurePrefixesFile from ../utils/ensure-files`
        );
      }

      // Verify we have a reasonable number of commands analyzed
      expect(actualCommands.length).toBeGreaterThan(48);

      // If we get here, all commands are using ensure utilities correctly
      expect(violations).toEqual([]);
    });
  });
});
