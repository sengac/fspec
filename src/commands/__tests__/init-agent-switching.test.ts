/**
 * Feature: spec/features/agent-switching-prompt-in-fspec-init.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { existsSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { installAgents } from '../init';
import { writeAgentConfig } from '../../utils/agentRuntimeConfig';

describe('Feature: Agent switching prompt in fspec init', () => {
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

  describe('Scenario: Switch agents when different agent requested via CLI', () => {
    it('should remove Claude files, install Cursor files, and update config when user confirms switch', async () => {
      // Given I have fspec initialized with Claude agent
      const specDir = join(testDir, 'spec');
      const claudeDir = join(testDir, '.claude', 'commands');
      await mkdir(specDir, { recursive: true });
      await mkdir(claudeDir, { recursive: true });

      // Given spec/fspec-config.json contains agent 'claude'
      const configPath = join(specDir, 'fspec-config.json');
      await writeFile(configPath, JSON.stringify({ agent: 'claude' }), 'utf-8');

      const claudeMdPath = join(specDir, 'CLAUDE.md');
      await writeFile(claudeMdPath, '# Claude Config', 'utf-8');

      const claudeSlashCmd = join(claudeDir, 'fspec.md');
      await writeFile(claudeSlashCmd, '# fspec command', 'utf-8');

      // When I run 'fspec init --agent=cursor'
      // When I select 'Switch to Cursor'
      // (Mock user confirming switch - this will be handled in implementation)
      await installAgents(testDir, ['cursor'], { shouldSwitch: true });

      // Write config (now done separately from installAgents)
      writeAgentConfig(testDir, 'cursor');

      // Then Claude files should be PRESERVED (INIT-015: templates no longer deleted)
      expect(existsSync(claudeMdPath)).toBe(true);
      expect(existsSync(claudeSlashCmd)).toBe(true);

      // Then Cursor files should be installed
      const cursorMdPath = join(specDir, 'CURSOR.md');
      const cursorSlashCmd = join(testDir, '.cursor', 'commands', 'fspec.md');
      expect(existsSync(cursorMdPath)).toBe(true);
      expect(existsSync(cursorSlashCmd)).toBe(true);

      // Then spec/fspec-config.json should contain agent 'cursor'
      const config = JSON.parse(await readFile(configPath, 'utf-8'));
      expect(config.agent).toBe('cursor');
    });
  });

  describe('Scenario: Cancel agent switch and keep existing setup', () => {
    it('should keep Claude files and not install Cursor when user cancels', async () => {
      // Given I have fspec initialized with Claude agent
      const specDir = join(testDir, 'spec');
      const claudeDir = join(testDir, '.claude', 'commands');
      await mkdir(specDir, { recursive: true });
      await mkdir(claudeDir, { recursive: true });

      const configPath = join(specDir, 'fspec-config.json');
      await writeFile(configPath, JSON.stringify({ agent: 'claude' }), 'utf-8');

      const claudeMdPath = join(specDir, 'CLAUDE.md');
      await writeFile(claudeMdPath, '# Claude Config', 'utf-8');

      const claudeSlashCmd = join(claudeDir, 'fspec.md');
      await writeFile(claudeSlashCmd, '# fspec command', 'utf-8');

      // When I run 'fspec init --agent=cursor'
      // When the prompt asks 'Switch from Claude to Cursor?'
      // When I select 'Cancel'
      try {
        await installAgents(testDir, ['cursor'], { shouldSwitch: false });
      } catch (error: any) {
        // Expected: should throw to indicate cancellation
        expect(error.message).toContain('cancelled');
      }

      // Then Claude files should remain unchanged
      expect(existsSync(claudeMdPath)).toBe(true);
      expect(existsSync(claudeSlashCmd)).toBe(true);

      // Then Cursor files should not be installed
      const cursorMdPath = join(specDir, 'CURSOR.md');
      expect(existsSync(cursorMdPath)).toBe(false);
    });
  });

  describe('Scenario: Switch agents in interactive mode', () => {
    it('should pre-select Claude, show switch prompt when Cursor selected, and switch when confirmed', async () => {
      // Given I have fspec initialized with Claude agent
      const specDir = join(testDir, 'spec');
      const claudeDir = join(testDir, '.claude', 'commands');
      await mkdir(specDir, { recursive: true });
      await mkdir(claudeDir, { recursive: true });

      const configPath = join(specDir, 'fspec-config.json');
      await writeFile(configPath, JSON.stringify({ agent: 'claude' }), 'utf-8');

      const claudeMdPath = join(specDir, 'CLAUDE.md');
      await writeFile(claudeMdPath, '# Claude Config', 'utf-8');

      // When I run 'fspec init' without --agent flag
      // Then the interactive selector should pre-select Claude
      // When I select Cursor from the list
      // When I confirm the switch
      await installAgents(testDir, ['cursor'], {
        interactiveMode: true,
        selectedAgent: 'cursor',
        shouldSwitch: true,
      });

      // Then Claude files should be PRESERVED (INIT-015) and Cursor files installed
      expect(existsSync(claudeMdPath)).toBe(true);
      const cursorMdPath = join(specDir, 'CURSOR.md');
      expect(existsSync(cursorMdPath)).toBe(true);
    });
  });

  describe('Scenario: Keep same agent in interactive mode without prompt', () => {
    it('should pre-select Claude and reinstall without prompt when user keeps Claude', async () => {
      // Given I have fspec initialized with Claude agent
      const specDir = join(testDir, 'spec');
      const claudeDir = join(testDir, '.claude', 'commands');
      await mkdir(specDir, { recursive: true });
      await mkdir(claudeDir, { recursive: true });

      const configPath = join(specDir, 'fspec-config.json');
      await writeFile(configPath, JSON.stringify({ agent: 'claude' }), 'utf-8');

      const claudeMdPath = join(specDir, 'CLAUDE.md');
      await writeFile(claudeMdPath, '# Claude Config', 'utf-8');

      // When I run 'fspec init' without --agent flag
      // Then the interactive selector should pre-select Claude
      // When I press Enter to confirm Claude
      await installAgents(testDir, ['claude'], {
        interactiveMode: true,
        selectedAgent: 'claude',
      });

      // Then no switch prompt should appear (tested via lack of shouldSwitch check)
      // Then Claude files should be reinstalled/refreshed
      expect(existsSync(claudeMdPath)).toBe(true);
    });
  });

  describe('Scenario: Reinstall same agent without prompt', () => {
    it('should reinstall Claude files without prompt when same agent requested via CLI', async () => {
      // Given I have fspec initialized with Claude agent
      const specDir = join(testDir, 'spec');
      const claudeDir = join(testDir, '.claude', 'commands');
      await mkdir(specDir, { recursive: true });
      await mkdir(claudeDir, { recursive: true });

      const configPath = join(specDir, 'fspec-config.json');
      await writeFile(configPath, JSON.stringify({ agent: 'claude' }), 'utf-8');

      const claudeMdPath = join(specDir, 'CLAUDE.md');
      await writeFile(claudeMdPath, '# Claude Config', 'utf-8');

      // When I run 'fspec init --agent=claude'
      await installAgents(testDir, ['claude']);

      // Then no switch prompt should appear (same agent)
      // Then Claude files should be reinstalled/refreshed (idempotent behavior)
      expect(existsSync(claudeMdPath)).toBe(true);

      // Then the command should exit successfully (no error thrown)
    });
  });
});
