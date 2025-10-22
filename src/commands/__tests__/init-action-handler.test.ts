/**
 * Feature: spec/features/incomplete-implementation-of-init-009-and-init-010-features.feature
 *
 * This test file validates the acceptance criteria for the init command action handler.
 * Tests map to scenarios: "Switch agents when different agent requested", "Cancel agent switch", "Reinstall same agent"
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { existsSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';

// This will be the new function we'll create
import { executeInit } from '../init';

describe('Feature: Agent switching prompt in fspec init - Action Handler', () => {
  let testDir: string;
  let originalCwd: string;

  beforeEach(async () => {
    // Create temporary test directory
    testDir = join(tmpdir(), `fspec-test-${Date.now()}`);
    await mkdir(testDir, { recursive: true });

    // Save and change working directory
    originalCwd = process.cwd();
    process.chdir(testDir);
  });

  afterEach(async () => {
    // Restore working directory
    process.chdir(originalCwd);

    // Clean up test directory
    if (existsSync(testDir)) {
      await rm(testDir, { recursive: true, force: true });
    }
  });

  describe('Scenario: Switch agents when different agent requested via CLI', () => {
    it('should detect existing agent, prompt for switch, and swap agents when confirmed', async () => {
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

      // Mock prompt to simulate user confirming switch
      const mockPromptSwitch = vi.fn().mockResolvedValue(true);

      // When I run 'fspec init --agent=cursor'
      const result = await executeInit({
        agentIds: ['cursor'],
        promptAgentSwitch: mockPromptSwitch
      });

      // Then the prompt should ask 'Switch from Claude to Cursor?'
      expect(mockPromptSwitch).toHaveBeenCalledWith('claude', 'cursor');

      // Then spec/CLAUDE.md should be removed
      expect(existsSync(claudeMdPath)).toBe(false);

      // Then .claude/commands/fspec.md should be removed
      expect(existsSync(claudeSlashCmd)).toBe(false);

      // Then spec/CURSOR.md should be created
      const cursorMdPath = join(specDir, 'CURSOR.md');
      expect(existsSync(cursorMdPath)).toBe(true);

      // Then .cursor/commands/fspec.md should be created
      const cursorSlashCmd = join(testDir, '.cursor', 'commands', 'fspec.md');
      expect(existsSync(cursorSlashCmd)).toBe(true);

      // Then spec/fspec-config.json should contain agent 'cursor'
      const config = JSON.parse(await readFile(configPath, 'utf-8'));
      expect(config.agent).toBe('cursor');

      // Then the output should show detailed list of installed files
      expect(result.filesInstalled).toContain('spec/CURSOR.md');
      expect(result.filesInstalled).toContain('.cursor/commands/fspec.md');
    });
  });

  describe('Scenario: Cancel agent switch and keep existing setup', () => {
    it('should detect existing agent, prompt for switch, and keep existing when cancelled', async () => {
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

      // Mock prompt to simulate user cancelling switch
      const mockPromptSwitch = vi.fn().mockResolvedValue(false);

      // When I run 'fspec init --agent=cursor'
      // When the prompt asks 'Switch from Claude to Cursor?'
      // When I select 'Cancel'
      const result = await executeInit({
        agentIds: ['cursor'],
        promptAgentSwitch: mockPromptSwitch
      });

      // Then the prompt should have been shown
      expect(mockPromptSwitch).toHaveBeenCalledWith('claude', 'cursor');

      // Then spec/CLAUDE.md should remain unchanged
      expect(existsSync(claudeMdPath)).toBe(true);
      const claudeContent = await readFile(claudeMdPath, 'utf-8');
      expect(claudeContent).toBe('# Claude Config');

      // Then .claude/commands/fspec.md should remain unchanged
      expect(existsSync(claudeSlashCmd)).toBe(true);

      // Then spec/CURSOR.md should not be created
      const cursorMdPath = join(specDir, 'CURSOR.md');
      expect(existsSync(cursorMdPath)).toBe(false);

      // Then spec/fspec-config.json should still contain agent 'claude'
      const config = JSON.parse(await readFile(configPath, 'utf-8'));
      expect(config.agent).toBe('claude');

      // Then the result should indicate cancellation
      expect(result.cancelled).toBe(true);
    });
  });

  describe('Scenario: Reinstall same agent without prompt (idempotent)', () => {
    it('should detect same agent and reinstall without showing prompt', async () => {
      // Given I have fspec initialized with Claude agent
      const specDir = join(testDir, 'spec');
      const claudeDir = join(testDir, '.claude', 'commands');
      await mkdir(specDir, { recursive: true });
      await mkdir(claudeDir, { recursive: true });

      const configPath = join(specDir, 'fspec-config.json');
      await writeFile(configPath, JSON.stringify({ agent: 'claude' }), 'utf-8');

      const claudeMdPath = join(specDir, 'CLAUDE.md');
      await writeFile(claudeMdPath, '# Claude Config OLD', 'utf-8');

      const claudeSlashCmd = join(claudeDir, 'fspec.md');
      await writeFile(claudeSlashCmd, '# fspec command OLD', 'utf-8');

      // Mock prompt (should NOT be called)
      const mockPromptSwitch = vi.fn();

      // When I run 'fspec init --agent=claude'
      const result = await executeInit({
        agentIds: ['claude'],
        promptAgentSwitch: mockPromptSwitch
      });

      // Then no switch prompt should appear
      expect(mockPromptSwitch).not.toHaveBeenCalled();

      // Then spec/CLAUDE.md should be reinstalled (content updated)
      expect(existsSync(claudeMdPath)).toBe(true);
      const claudeContent = await readFile(claudeMdPath, 'utf-8');
      expect(claudeContent).not.toBe('# Claude Config OLD'); // Should be regenerated

      // Then .claude/commands/fspec.md should be reinstalled
      expect(existsSync(claudeSlashCmd)).toBe(true);

      // Then spec/fspec-config.json should still contain agent 'claude'
      const config = JSON.parse(await readFile(configPath, 'utf-8'));
      expect(config.agent).toBe('claude');

      // Then the command should exit successfully
      expect(result.success).toBe(true);
    });
  });

  describe('Scenario: No duplicate writeAgentConfig calls', () => {
    it('should write agent config only once during installation', async () => {
      // Given I have no existing agent installed
      const specDir = join(testDir, 'spec');
      await mkdir(specDir, { recursive: true });

      // Track config writes using a mock
      const configWrites: string[] = [];
      const originalWriteFile = (global as any).writeAgentConfigMock;

      // When I run 'fspec init --agent=claude'
      const result = await executeInit({
        agentIds: ['claude'],
        trackConfigWrites: (agent: string) => configWrites.push(agent)
      });

      // Then agent config should be written exactly once
      expect(configWrites).toHaveLength(1);
      expect(configWrites[0]).toBe('claude');
    });
  });
});
