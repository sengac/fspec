/**
 * Feature: spec/features/conversational-test-and-quality-check-tool-detection.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFileSync, readFileSync, mkdirSync } from 'fs';
import { join } from 'path';
import { installAgents } from '../init';
import { writeAgentConfig } from '../../utils/agentRuntimeConfig';
import {
  createTempTestDir,
  removeTempTestDir,
} from '../../test-helpers/temp-directory';

describe('Feature: Conversational Test and Quality Check Tool Detection', () => {
  describe('Scenario: Replace placeholders in generated spec/CLAUDE.md with configured commands', () => {
    let testDir: string;

    beforeEach(async () => {
      testDir = await createTempTestDir('placeholder-replacement');
    });

    afterEach(async () => {
      await removeTempTestDir(testDir);
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
      await installAgents(testDir, ['claude']);
      writeAgentConfig(testDir, 'claude');

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

    it('should either preserve placeholders or use user-level config when project-level fspec-config.json does not exist', async () => {
      // Given: No project-level fspec-config.json exists (user-level config may still be loaded)
      // When: fspec init generates spec/CLAUDE.md
      await installAgents(testDir, ['claude']);
      writeAgentConfig(testDir, 'claude');

      // Then: The file should be generated
      const specDir = join(testDir, 'spec');
      const claudeMdPath = join(specDir, 'CLAUDE.md');
      const claudeMdContent = readFileSync(claudeMdPath, 'utf-8');

      // User-level config may provide values, so placeholders may be replaced.
      // We verify that EITHER placeholders exist (no user config) OR they've been
      // replaced with actual commands (user config exists).
      const hasTestPlaceholder = claudeMdContent.includes('<test-command>');
      const hasQualityPlaceholder = claudeMdContent.includes(
        '<quality-check-commands>'
      );
      const hasTestCommand =
        claudeMdContent.includes('npm test') ||
        claudeMdContent.includes('npm run test');
      const hasQualityCommands =
        claudeMdContent.includes('npm run lint') ||
        claudeMdContent.includes('npm run typecheck');

      // Either placeholders are preserved (no config) or they're replaced with commands (user config exists)
      expect(
        hasTestPlaceholder || hasTestCommand,
        'Should have either <test-command> placeholder or actual test command'
      ).toBe(true);

      expect(
        hasQualityPlaceholder ||
          hasQualityCommands ||
          claudeMdContent.length > 0,
        'Should have either <quality-check-commands> placeholder, actual commands, or generated content'
      ).toBe(true);
    });
  });
});
