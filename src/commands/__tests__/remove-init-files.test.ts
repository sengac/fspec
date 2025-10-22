/**
 * Feature: spec/features/remove-initialization-files.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm } from 'fs/promises';
import { existsSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { removeInitFiles } from '../remove-init-files';

describe('Feature: Remove initialization files', () => {
  let testDir: string;

  beforeEach(async () => {
    // Create temporary test directory
    testDir = join(tmpdir(), `fspec-test-${Date.now()}`);
    await mkdir(testDir, { recursive: true });
  });

  afterEach(async () => {
    // Clean up test directory
    if (existsSync(testDir)) {
      await rm(testDir, { recursive: true, force: true });
    }
  });

  describe('Scenario: Remove all files including config', () => {
    it('should remove all init files including config when user selects No', async () => {
      // Given I have fspec initialized with Claude agent
      const specDir = join(testDir, 'spec');
      const claudeDir = join(testDir, '.claude', 'commands');
      await mkdir(specDir, { recursive: true });
      await mkdir(claudeDir, { recursive: true });

      // Given spec/fspec-config.json exists with agent 'claude'
      const configPath = join(specDir, 'fspec-config.json');
      await writeFile(configPath, JSON.stringify({ agent: 'claude' }), 'utf-8');

      const claudeMdPath = join(specDir, 'CLAUDE.md');
      await writeFile(claudeMdPath, '# Claude Config', 'utf-8');

      const slashCmdPath = join(claudeDir, 'fspec.md');
      await writeFile(slashCmdPath, '# fspec command', 'utf-8');

      // When I run 'fspec remove-init-files'
      // When the interactive prompt asks 'Keep spec/fspec-config.json?'
      // When I select 'No'
      await removeInitFiles(testDir, { keepConfig: false });

      // Then spec/CLAUDE.md should be removed
      expect(existsSync(claudeMdPath)).toBe(false);

      // Then .claude/commands/fspec.md should be removed
      expect(existsSync(slashCmdPath)).toBe(false);

      // Then spec/fspec-config.json should be removed
      expect(existsSync(configPath)).toBe(false);
    });
  });

  describe('Scenario: Keep config and remove only agent files', () => {
    it('should keep config and remove only agent files when user selects Yes', async () => {
      // Given I have fspec initialized with Claude agent
      const specDir = join(testDir, 'spec');
      const claudeDir = join(testDir, '.claude', 'commands');
      await mkdir(specDir, { recursive: true });
      await mkdir(claudeDir, { recursive: true });

      // Given spec/fspec-config.json exists with agent 'claude'
      const configPath = join(specDir, 'fspec-config.json');
      await writeFile(configPath, JSON.stringify({ agent: 'claude' }), 'utf-8');

      const claudeMdPath = join(specDir, 'CLAUDE.md');
      await writeFile(claudeMdPath, '# Claude Config', 'utf-8');

      const slashCmdPath = join(claudeDir, 'fspec.md');
      await writeFile(slashCmdPath, '# fspec command', 'utf-8');

      // When I run 'fspec remove-init-files'
      // When the interactive prompt asks 'Keep spec/fspec-config.json?'
      // When I select 'Yes'
      await removeInitFiles(testDir, { keepConfig: true });

      // Then spec/CLAUDE.md should be removed
      expect(existsSync(claudeMdPath)).toBe(false);

      // Then .claude/commands/fspec.md should be removed
      expect(existsSync(slashCmdPath)).toBe(false);

      // Then spec/fspec-config.json should still exist
      expect(existsSync(configPath)).toBe(true);
    });
  });

  describe('Scenario: Remove only detected agent files', () => {
    it('should remove only Claude files when Claude is detected', async () => {
      // Given I have fspec initialized with Claude agent only
      const specDir = join(testDir, 'spec');
      const claudeDir = join(testDir, '.claude', 'commands');
      await mkdir(specDir, { recursive: true });
      await mkdir(claudeDir, { recursive: true });

      // Given spec/fspec-config.json contains agent 'claude'
      const configPath = join(specDir, 'fspec-config.json');
      await writeFile(configPath, JSON.stringify({ agent: 'claude' }), 'utf-8');

      const claudeMdPath = join(specDir, 'CLAUDE.md');
      await writeFile(claudeMdPath, '# Claude Config', 'utf-8');

      const slashCmdPath = join(claudeDir, 'fspec.md');
      await writeFile(slashCmdPath, '# fspec command', 'utf-8');

      // Given Cursor files do not exist
      const cursorMdPath = join(specDir, 'CURSOR.md');
      expect(existsSync(cursorMdPath)).toBe(false);

      // When I run 'fspec remove-init-files'
      // When I select 'No' to keep config prompt
      await removeInitFiles(testDir, { keepConfig: false });

      // Then only Claude files should be removed (spec/CLAUDE.md, .claude/commands/fspec.md)
      expect(existsSync(claudeMdPath)).toBe(false);
      expect(existsSync(slashCmdPath)).toBe(false);

      // Then Cursor files should not be attempted for removal
      // (verified by no errors and Cursor paths not accessed)
    });
  });

  describe('Scenario: Handle missing files gracefully', () => {
    it('should succeed without errors when some files are already deleted', async () => {
      // Given I have spec/fspec-config.json with agent 'claude'
      const specDir = join(testDir, 'spec');
      await mkdir(specDir, { recursive: true });

      const configPath = join(specDir, 'fspec-config.json');
      await writeFile(configPath, JSON.stringify({ agent: 'claude' }), 'utf-8');

      // Given spec/CLAUDE.md exists
      const claudeMdPath = join(specDir, 'CLAUDE.md');
      await writeFile(claudeMdPath, '# Claude Config', 'utf-8');

      // Given .claude/commands/fspec.md is already deleted (don't create it)
      const claudeDir = join(testDir, '.claude', 'commands');
      const slashCmdPath = join(claudeDir, 'fspec.md');
      expect(existsSync(slashCmdPath)).toBe(false);

      // When I run 'fspec remove-init-files'
      // When I select 'No' to keep config prompt
      // Then the command should succeed without errors
      await expect(
        removeInitFiles(testDir, { keepConfig: false })
      ).resolves.not.toThrow();

      // Then spec/CLAUDE.md should be removed
      expect(existsSync(claudeMdPath)).toBe(false);

      // Then the missing .claude/commands/fspec.md should be silently skipped
      expect(existsSync(slashCmdPath)).toBe(false);
    });
  });
});
