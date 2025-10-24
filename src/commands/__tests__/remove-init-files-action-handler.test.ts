/**
 * Feature: spec/features/incomplete-implementation-of-init-009-and-init-010-features.feature
 *
 * This test file validates the acceptance criteria for the remove-init-files command action handler.
 * Tests map to scenarios: "Remove all files using --no-keep-config flag" and "Keep config using --keep-config flag"
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mkdir, writeFile, rm } from 'fs/promises';
import { existsSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';

// This will be the new function we'll create that the action handler calls
import { executeRemoveInitFiles } from '../remove-init-files';

describe('Feature: Remove initialization files - Action Handler', () => {
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

  describe('Scenario: Remove all files using --no-keep-config flag (non-interactive)', () => {
    it('should remove all files including config without showing prompt', async () => {
      // Given I have fspec initialized with Claude agent
      const specDir = join(testDir, 'spec');
      const claudeDir = join(testDir, '.claude', 'commands');
      await mkdir(specDir, { recursive: true });
      await mkdir(claudeDir, { recursive: true });

      // Given spec/fspec-config.json exists
      const configPath = join(specDir, 'fspec-config.json');
      await writeFile(configPath, JSON.stringify({ agent: 'claude' }), 'utf-8');

      const claudeMdPath = join(specDir, 'CLAUDE.md');
      await writeFile(claudeMdPath, '# Claude Config', 'utf-8');

      const slashCmdPath = join(claudeDir, 'fspec.md');
      await writeFile(slashCmdPath, '# fspec command', 'utf-8');

      // When I run 'fspec remove-init-files --no-keep-config'
      const result = await executeRemoveInitFiles({ keepConfig: false });

      // Then no interactive prompt should appear (tested by not mocking ConfirmPrompt)
      // Then spec/CLAUDE.md should be removed
      expect(existsSync(claudeMdPath)).toBe(false);

      // Then .claude/commands/fspec.md should be removed
      expect(existsSync(slashCmdPath)).toBe(false);

      // Then spec/fspec-config.json should be removed
      expect(existsSync(configPath)).toBe(false);

      // Then the output should show detailed list of removed files
      expect(result.filesRemoved).toContain('spec/CLAUDE.md');
      expect(result.filesRemoved).toContain('.claude/commands/fspec.md');
      expect(result.filesRemoved).toContain('spec/fspec-config.json');
    });
  });

  describe('Scenario: Keep config using --keep-config flag (non-interactive)', () => {
    it('should keep config and remove agent files without showing prompt', async () => {
      // Given I have fspec initialized with Claude agent
      const specDir = join(testDir, 'spec');
      const claudeDir = join(testDir, '.claude', 'commands');
      await mkdir(specDir, { recursive: true });
      await mkdir(claudeDir, { recursive: true });

      // Given spec/fspec-config.json exists
      const configPath = join(specDir, 'fspec-config.json');
      await writeFile(configPath, JSON.stringify({ agent: 'claude' }), 'utf-8');

      const claudeMdPath = join(specDir, 'CLAUDE.md');
      await writeFile(claudeMdPath, '# Claude Config', 'utf-8');

      const slashCmdPath = join(claudeDir, 'fspec.md');
      await writeFile(slashCmdPath, '# fspec command', 'utf-8');

      // When I run 'fspec remove-init-files --keep-config'
      const result = await executeRemoveInitFiles({ keepConfig: true });

      // Then no interactive prompt should appear
      // Then spec/CLAUDE.md should be removed
      expect(existsSync(claudeMdPath)).toBe(false);

      // Then .claude/commands/fspec.md should be removed
      expect(existsSync(slashCmdPath)).toBe(false);

      // Then spec/fspec-config.json should still exist
      expect(existsSync(configPath)).toBe(true);

      // Then the output should show detailed list of removed files
      expect(result.filesRemoved).toContain('spec/CLAUDE.md');
      expect(result.filesRemoved).toContain('.claude/commands/fspec.md');
      expect(result.filesRemoved).not.toContain('spec/fspec-config.json');
    });
  });

  describe('Scenario: Interactive prompt when no flags provided', () => {
    it('should show interactive prompt and respect user choice (Yes = keep config)', async () => {
      // Given I have fspec initialized with Claude agent
      const specDir = join(testDir, 'spec');
      const claudeDir = join(testDir, '.claude', 'commands');
      await mkdir(specDir, { recursive: true });
      await mkdir(claudeDir, { recursive: true });

      const configPath = join(specDir, 'fspec-config.json');
      await writeFile(configPath, JSON.stringify({ agent: 'claude' }), 'utf-8');

      const claudeMdPath = join(specDir, 'CLAUDE.md');
      await writeFile(claudeMdPath, '# Claude Config', 'utf-8');

      const slashCmdPath = join(claudeDir, 'fspec.md');
      await writeFile(slashCmdPath, '# fspec command', 'utf-8');

      // Mock ConfirmPrompt to simulate user selecting "Yes" (keep config)
      // This will need to be implemented when we integrate ConfirmPrompt
      const mockPromptKeepConfig = vi.fn().mockResolvedValue(true);

      // When I run 'fspec remove-init-files' without flags
      const result = await executeRemoveInitFiles({
        promptKeepConfig: mockPromptKeepConfig,
      });

      // Then the interactive prompt should have been shown
      expect(mockPromptKeepConfig).toHaveBeenCalledWith(
        'Keep spec/fspec-config.json?'
      );

      // Then spec/fspec-config.json should still exist (user said Yes)
      expect(existsSync(configPath)).toBe(true);

      // Then agent files should be removed
      expect(existsSync(claudeMdPath)).toBe(false);
      expect(existsSync(slashCmdPath)).toBe(false);
    });

    it('should show interactive prompt and respect user choice (No = remove all)', async () => {
      // Given I have fspec initialized with Claude agent
      const specDir = join(testDir, 'spec');
      const claudeDir = join(testDir, '.claude', 'commands');
      await mkdir(specDir, { recursive: true });
      await mkdir(claudeDir, { recursive: true });

      const configPath = join(specDir, 'fspec-config.json');
      await writeFile(configPath, JSON.stringify({ agent: 'claude' }), 'utf-8');

      const claudeMdPath = join(specDir, 'CLAUDE.md');
      await writeFile(claudeMdPath, '# Claude Config', 'utf-8');

      const slashCmdPath = join(claudeDir, 'fspec.md');
      await writeFile(slashCmdPath, '# fspec command', 'utf-8');

      // Mock ConfirmPrompt to simulate user selecting "No" (remove all)
      const mockPromptKeepConfig = vi.fn().mockResolvedValue(false);

      // When I run 'fspec remove-init-files' without flags
      const result = await executeRemoveInitFiles({
        promptKeepConfig: mockPromptKeepConfig,
      });

      // Then the interactive prompt should have been shown
      expect(mockPromptKeepConfig).toHaveBeenCalledWith(
        'Keep spec/fspec-config.json?'
      );

      // Then all files including config should be removed (user said No)
      expect(existsSync(configPath)).toBe(false);
      expect(existsSync(claudeMdPath)).toBe(false);
      expect(existsSync(slashCmdPath)).toBe(false);
    });
  });
});
