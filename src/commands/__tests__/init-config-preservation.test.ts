/**
 * Feature: spec/features/fspec-init-config-preservation.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { existsSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { installAgents } from '../init';
import { writeAgentConfig } from '../../utils/agentRuntimeConfig';

describe('Feature: fspec init config preservation', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(tmpdir(), `fspec-test-${Date.now()}`);
    await mkdir(testDir, { recursive: true });
  });

  afterEach(async () => {
    if (existsSync(testDir)) {
      await rm(testDir, { recursive: true, force: true });
    }
  });

  describe('Scenario: Preserve existing tool configuration when switching agents', () => {
    it('should preserve tools configuration when switching from claude to cursor', async () => {
      // Given I have a config file with agent 'claude' and tools configuration
      const specDir = join(testDir, 'spec');
      await mkdir(specDir, { recursive: true });

      const configPath = join(specDir, 'fspec-config.json');
      const initialConfig = {
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
      await writeFile(
        configPath,
        JSON.stringify(initialConfig, null, 2),
        'utf-8'
      );

      // When I run 'fspec init cursor'
      writeAgentConfig(testDir, 'cursor');

      // Then the config file should have agent set to 'cursor'
      const updatedConfig = JSON.parse(await readFile(configPath, 'utf-8'));
      expect(updatedConfig.agent).toBe('cursor');

      // And the tools configuration should be preserved
      expect(updatedConfig.tools).toEqual(initialConfig.tools);
    });
  });

  describe('Scenario: Preserve templates for all installed agents when switching agents', () => {
    it('should preserve claude and cursor templates when switching to aider', async () => {
      // Given I have templates for both 'claude' and 'cursor' agents installed
      const specDir = join(testDir, 'spec');
      const claudeDir = join(testDir, '.claude', 'commands');
      const cursorDir = join(testDir, '.cursor', 'commands');

      await mkdir(specDir, { recursive: true });
      await mkdir(claudeDir, { recursive: true });
      await mkdir(cursorDir, { recursive: true });

      const claudeMdPath = join(specDir, 'CLAUDE.md');
      const cursorMdPath = join(specDir, 'CURSOR.md');
      const claudeSlashCmd = join(claudeDir, 'fspec.md');
      const cursorSlashCmd = join(cursorDir, 'fspec.md');

      await writeFile(claudeMdPath, '# Claude Config', 'utf-8');
      await writeFile(cursorMdPath, '# Cursor Config', 'utf-8');
      await writeFile(claudeSlashCmd, '# fspec command for Claude', 'utf-8');
      await writeFile(cursorSlashCmd, '# fspec command for Cursor', 'utf-8');

      // When I run 'fspec init aider'
      await installAgents(testDir, ['aider'], { shouldSwitch: true });

      // Then the 'claude' templates should still exist
      expect(existsSync(claudeMdPath)).toBe(true);
      expect(existsSync(claudeSlashCmd)).toBe(true);

      // And the 'cursor' templates should still exist
      expect(existsSync(cursorMdPath)).toBe(true);
      expect(existsSync(cursorSlashCmd)).toBe(true);

      // And the 'aider' templates should be created
      const aiderMdPath = join(specDir, 'AIDER.md');
      expect(existsSync(aiderMdPath)).toBe(true);
    });
  });

  describe('Scenario: Switch agents while preserving all existing configuration', () => {
    it('should preserve all config fields when switching from claude to cursor', async () => {
      // Given I have a config file with agent 'claude', test command 'npm test', and quality check commands
      const specDir = join(testDir, 'spec');
      await mkdir(specDir, { recursive: true });

      const configPath = join(specDir, 'fspec-config.json');
      const initialConfig = {
        agent: 'claude',
        tools: {
          test: {
            command: 'npm test',
          },
          qualityCheck: {
            commands: ['npm run lint'],
          },
        },
      };
      await writeFile(
        configPath,
        JSON.stringify(initialConfig, null, 2),
        'utf-8'
      );

      // When I run 'fspec init cursor'
      writeAgentConfig(testDir, 'cursor');

      // Then the config file should have agent set to 'cursor'
      const updatedConfig = JSON.parse(await readFile(configPath, 'utf-8'));
      expect(updatedConfig.agent).toBe('cursor');

      // And the test command should still be 'npm test'
      expect(updatedConfig.tools?.test?.command).toBe('npm test');

      // And the quality check commands should be preserved
      expect(updatedConfig.tools?.qualityCheck?.commands).toEqual([
        'npm run lint',
      ]);
    });
  });
});
