/**
 * Feature: spec/features/wire-up-multi-agent-support-to-fspec-init-command.feature
 *
 * This test file validates the acceptance criteria for bundling and global install scenarios.
 * Tests verify that fspec init works without filesystem dependencies after bundling.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { join } from 'path';
import { mkdtemp, rm, readFile, readdir } from 'fs/promises';
import { tmpdir } from 'os';
import { installAgents } from '../init';
import { getSlashCommandTemplate } from '../../utils/slashCommandTemplate';

describe('Feature: Wire up multi-agent support to fspec init command', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-init-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Works after global npm install', () => {
    it('should execute successfully without requiring local template files', async () => {
      // Given: I have installed fspec globally (simulated by running from dist/)
      // When: I run "fspec init" in a fresh project directory
      await installAgents(testDir, ['claude']);

      // Then: The command should execute successfully
      // And: Agent-specific files should be created with correct content
      const claudeMdPath = join(testDir, 'CLAUDE.md');
      const claudeMdContent = await readFile(claudeMdPath, 'utf-8');

      expect(claudeMdContent).toContain('Claude Code');
      expect(claudeMdContent).toContain('fspec');

      // And: No errors about missing files or templates should occur
      // (if this test runs without throwing, we passed)
    });

    it('should execute using embedded templates', async () => {
      // When: I run fspec init
      const result = installAgents(testDir, ['claude']);

      // Then: Templates are loaded from embedded TypeScript modules
      await expect(result).resolves.not.toThrow();
    });
  });

  describe('Scenario: Agent-specific customization', () => {
    it('should generate Cursor-specific content when --agent=cursor is used', async () => {
      // Given: I run "fspec init --agent=cursor"
      await installAgents(testDir, ['cursor']);

      // When: The installation completes
      // Then: Generated files should contain "Cursor" not "Claude Code"
      const cursorMdPath = join(testDir, 'CURSOR.md');
      const cursorMdContent = await readFile(cursorMdPath, 'utf-8');

      expect(cursorMdContent).toContain('Cursor');
      expect(cursorMdContent).not.toContain('Claude Code');

      // And: The slash command path should be ".cursor/commands/"
      expect(cursorMdContent).toContain('.cursor/commands/');

      // And: The documentation should reference "CURSOR.md"
      const specCursorPath = join(testDir, 'spec', 'CURSOR.md');
      const specCursorContent = await readFile(specCursorPath, 'utf-8');

      expect(specCursorContent).toContain('Cursor');
      expect(specCursorContent).toContain('spec/CURSOR.md');
    });

    it('should generate Claude-specific content when --agent=claude is used', async () => {
      // Given: I run "fspec init --agent=claude"
      await installAgents(testDir, ['claude']);

      // When: The installation completes
      // Then: Generated files should contain "Claude Code"
      const claudeMdPath = join(testDir, 'CLAUDE.md');
      const claudeMdContent = await readFile(claudeMdPath, 'utf-8');

      expect(claudeMdContent).toContain('Claude Code');
      expect(claudeMdContent).not.toContain('Cursor');

      // And: The slash command path should be ".claude/commands/"
      expect(claudeMdContent).toContain('.claude/commands/');

      // And: The documentation should reference "CLAUDE.md"
      const specClaudePath = join(testDir, 'spec', 'CLAUDE.md');
      const specClaudeContent = await readFile(specClaudePath, 'utf-8');

      expect(specClaudeContent).toContain('Claude Code');
      expect(specClaudeContent).toContain('spec/CLAUDE.md');
    });
  });

  describe('Scenario: Embedded templates work independently', () => {
    it('should work in any directory using embedded templates', async () => {
      const emptyDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));

      try {
        // When: I call getSlashCommandTemplate() in any directory
        const template = getSlashCommandTemplate();

        // Then: Template is loaded from embedded TypeScript modules
        expect(template.length).toBeGreaterThan(1000);
        expect(template).toContain('fspec');

        // And: fspec init works
        await expect(installAgents(emptyDir, ['claude'])).resolves.not.toThrow();
      } finally {
        await rm(emptyDir, { recursive: true, force: true });
      }
    });

    it('should return complete embedded template', async () => {
      // When: I call getSlashCommandTemplate()
      const template = getSlashCommandTemplate();

      // Then: Template contains complete content from embedded modules
      const lineCount = template.split('\n').length;
      expect(lineCount).toBeGreaterThan(100);
    });

    it('should generate consistent templates for all agents', async () => {
      // When: I run fspec init for multiple agents
      await installAgents(testDir, ['claude', 'cursor']);

      const claudeSlashCmd = join(testDir, '.claude', 'commands', 'fspec.md');
      const cursorSlashCmd = join(testDir, '.cursor', 'commands', 'fspec.md');

      const claudeCmdContent = await readFile(claudeSlashCmd, 'utf-8');
      const cursorCmdContent = await readFile(cursorSlashCmd, 'utf-8');

      // Then: Both contain complete embedded template content
      expect(claudeCmdContent.length).toBeGreaterThan(1000);
      expect(cursorCmdContent.length).toBeGreaterThan(1000);

      expect(claudeCmdContent).toContain('fspec');
      expect(cursorCmdContent).toContain('fspec');
    });
  });

  describe('Scenario: Slash command files use correct format without YAML frontmatter', () => {
    it('should generate markdown files without YAML frontmatter', async () => {
      // Given: I run "fspec init --agent=claude"
      await installAgents(testDir, ['claude']);

      // When: The slash command file is generated
      const slashCmdPath = join(testDir, '.claude', 'commands', 'fspec.md');
      const slashCmdContent = await readFile(slashCmdPath, 'utf-8');

      // Then: The markdown file should NOT contain YAML frontmatter (---)
      const firstLines = slashCmdContent.split('\n').slice(0, 10).join('\n');
      expect(slashCmdContent).not.toMatch(/^---\s*\n/);
      expect(firstLines).not.toContain('name: fspec');
      expect(firstLines).not.toContain('description: Load fspec');
      expect(firstLines).not.toContain('category: Project');
      expect(firstLines).not.toContain('tags: [fspec');

      // And: The file should start with "# fspec Command"
      expect(slashCmdContent).toMatch(/^# fspec Command/);
    });

    it('should generate TOML files with [command] section metadata', async () => {
      // Given: I run "fspec init --agent=gemini"
      await installAgents(testDir, ['gemini']);

      // When: The slash command file is generated
      const slashCmdPath = join(testDir, '.gemini', 'commands', 'fspec.toml');
      const slashCmdContent = await readFile(slashCmdPath, 'utf-8');

      // Then: TOML files (Gemini, Qwen) should have [command] section with metadata
      expect(slashCmdContent).toContain('[command]');
      expect(slashCmdContent).toContain('name = "fspec - Load Project Context"');
      expect(slashCmdContent).toContain('description = "Load fspec workflow and ACDD methodology"');

      // And: TOML files should NOT have YAML frontmatter
      expect(slashCmdContent).not.toMatch(/^---\s*\n/);
    });
  });
});
