/**
 * Feature: spec/features/conversational-test-and-quality-check-tool-detection.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdirSync, writeFileSync, readFileSync, rmSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { execSync } from 'child_process';

describe('Feature: Conversational Test and Quality Check Tool Detection', () => {
  describe('Scenario: Replace placeholders in generated spec/CLAUDE.md with configured commands', () => {
    let testDir: string;

    beforeEach(() => {
      testDir = join(tmpdir(), `fspec-test-${Date.now()}`);
      mkdirSync(testDir, { recursive: true });
    });

    afterEach(() => {
      if (testDir) {
        rmSync(testDir, { recursive: true, force: true });
      }
    });

    it('should replace <test-command> and <quality-check-commands> placeholders with configured commands', async () => {
      // @step Given slash command section generators (src/utils/slashCommandSections/*.ts) return content with <test-command> and <quality-check-commands> placeholders
      // Slash command sections already contain placeholders (verified in documentation-validation.test.ts)

      // @step And spec/fspec-config.json has tools.test.command = 'npm test' configured
      const specDir = join(testDir, 'spec');
      mkdirSync(specDir, { recursive: true });

      const config = {
        agent: 'claude',
        tools: {
          test: {
            command: 'npm test',
          },
          qualityCheck: {
            commands: ['npm run lint', 'npm run typecheck'],
          },
        },
      };

      writeFileSync(
        join(specDir, 'fspec-config.json'),
        JSON.stringify(config, null, 2),
        'utf-8'
      );

      // @step When fspec init command calls slash command section generators and assembles the output
      // @step And placeholder replacement logic reads spec/fspec-config.json to get configured commands
      try {
        execSync(
          `node ${join(process.cwd(), 'dist/index.js')} init --agent claude`,
          {
            cwd: testDir,
            stdio: 'pipe',
          }
        );
      } catch (_error) {
        // Init might fail, but we're checking the file content
      }

      // @step Then the generated spec/CLAUDE.md file should have all <test-command> placeholders replaced with 'npm test'
      const claudeMdPath = join(specDir, 'CLAUDE.md');
      const claudeMdContent = readFileSync(claudeMdPath, 'utf-8');

      // Check that <test-command> placeholders were replaced
      const testCommandPlaceholders = claudeMdContent.match(/<test-command>/g);
      expect(
        testCommandPlaceholders,
        'Found <test-command> placeholders that should have been replaced with "npm test"'
      ).toBeNull();

      // Verify actual command appears in the content
      expect(
        claudeMdContent,
        'Generated spec/CLAUDE.md should contain "npm test" from config'
      ).toContain('npm test');

      // @step And the generated spec/CLAUDE.md file should have all <quality-check-commands> placeholders replaced with configured quality commands
      const qualityCommandPlaceholders = claudeMdContent.match(
        /<quality-check-commands>/g
      );
      expect(
        qualityCommandPlaceholders,
        'Found <quality-check-commands> placeholders that should have been replaced'
      ).toBeNull();

      // Verify chained quality commands appear (npm run lint && npm run typecheck)
      expect(
        claudeMdContent,
        'Generated spec/CLAUDE.md should contain chained quality commands from config'
      ).toContain('npm run lint && npm run typecheck');
    });

    it('should preserve placeholders when fspec-config.json does not exist', async () => {
      // Given: No fspec-config.json exists
      // When: fspec init generates spec/CLAUDE.md
      try {
        execSync(
          `node ${join(process.cwd(), 'dist/index.js')} init --agent claude`,
          {
            cwd: testDir,
            stdio: 'pipe',
          }
        );
      } catch (_error) {
        // Init might fail, but we're checking the file content
      }

      // Then: Placeholders should remain unchanged (fallback behavior)
      const specDir = join(testDir, 'spec');
      const claudeMdPath = join(specDir, 'CLAUDE.md');
      const claudeMdContent = readFileSync(claudeMdPath, 'utf-8');

      // Placeholders should still exist when no config is present
      expect(
        claudeMdContent,
        'Should preserve <test-command> placeholder when no config exists'
      ).toContain('<test-command>');

      expect(
        claudeMdContent,
        'Should preserve <quality-check-commands> placeholder when no config exists'
      ).toContain('<quality-check-commands>');
    });
  });
});
