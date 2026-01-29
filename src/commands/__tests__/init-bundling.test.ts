/**
 * Feature: spec/features/wire-up-multi-agent-support-to-fspec-init-command.feature
 *
 * This test file validates the acceptance criteria for bundling and global install scenarios.
 * Tests verify that fspec init works without filesystem dependencies after bundling.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { join } from 'path';
import { mkdtemp, rm, readFile } from 'fs/promises';
import { tmpdir } from 'os';
import { installAgents } from '../init';
import { getSlashCommandTemplate } from '../../utils/slashCommandTemplate';
import {
  setupTestDirectory,
  type TestDirectorySetup,
} from '../../test-helpers/universal-test-setup';

describe('Feature: Wire up multi-agent support to fspec init command', () => {
  let setup: TestDirectorySetup;

  beforeEach(async () => {
    setup = await setupTestDirectory('init-bundling');
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: Works after global npm install', () => {
    it('should execute successfully without requiring local template files', async () => {
      // Given: I have installed fspec globally (simulated by running from dist/)
      // When: I run "fspec init" in a fresh project directory
      await installAgents(setup.testDir, ['claude']);

      // Then: The command should execute successfully
      // And: spec/CLAUDE.md should be created with correct content
      const specClaudeMdPath = join(setup.testDir, 'spec', 'CLAUDE.md');
      const claudeMdContent = await readFile(specClaudeMdPath, 'utf-8');

      expect(claudeMdContent).toContain(
        'Project Management and Specification Guidelines'
      );
      expect(claudeMdContent).toContain('fspec');

      // And: No errors about missing files or templates should occur
      // (if this test runs without throwing, we passed)
    });

    it('should execute using embedded templates', async () => {
      // When: I run fspec init
      const result = installAgents(setup.testDir, ['claude']);

      // Then: Templates are loaded from embedded TypeScript modules
      await expect(result).resolves.not.toThrow();
    });
  });

  describe('Scenario: Agent-specific customization', () => {
    it('should generate Cursor-specific content when --agent=cursor is used', async () => {
      // Given: I run "fspec init --agent=cursor"
      await installAgents(setup.testDir, ['cursor']);

      // When: The installation completes
      // Then: spec/CURSOR.md should contain comprehensive Project Management Guidelines
      const specCursorPath = join(setup.testDir, 'spec', 'CURSOR.md');
      const specCursorContent = await readFile(specCursorPath, 'utf-8');

      const lineCount = specCursorContent.split('\n').length;
      expect(lineCount).toBeGreaterThan(2000); // Comprehensive Project Management Guidelines, not a stub
      expect(specCursorContent).toContain(
        'Project Management and Specification Guidelines'
      ); // Contains guidelines
    });

    it('should generate Claude-specific content when --agent=claude is used', async () => {
      // Given: I run "fspec init --agent=claude"
      await installAgents(setup.testDir, ['claude']);

      // When: The installation completes
      // Then: spec/CLAUDE.md should contain comprehensive Project Management Guidelines
      const specClaudePath = join(setup.testDir, 'spec', 'CLAUDE.md');
      const specClaudeContent = await readFile(specClaudePath, 'utf-8');

      const lineCount = specClaudeContent.split('\n').length;
      expect(lineCount).toBeGreaterThan(2000); // Comprehensive Project Management Guidelines, not a stub
      expect(specClaudeContent).toContain(
        'Project Management and Specification Guidelines'
      ); // Contains guidelines
    });
  });

  describe('Scenario: Embedded templates work independently', () => {
    it('should work in any directory using embedded templates', async () => {
      const emptyDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));

      try {
        // When: I call getSlashCommandTemplate() in any directory
        const template = getSlashCommandTemplate();

        // Then: Template is minimal (header + two commands only)
        expect(template.length).toBeLessThan(700); // Minimal template with clear instructions
        expect(template).toContain('fspec --sync-version');
        expect(template).toContain('fspec bootstrap');

        // And: fspec init works
        await expect(
          installAgents(emptyDir, ['claude'])
        ).resolves.not.toThrow();
      } finally {
        await rm(emptyDir, { recursive: true, force: true });
      }
    });

    it('should return complete embedded template', async () => {
      // When: I call getSlashCommandTemplate()
      const template = getSlashCommandTemplate();

      // Then: Template contains minimal header content from embedded modules
      const lineCount = template.split('\n').length;
      expect(lineCount).toBeLessThanOrEqual(25); // Minimal template with clear instructions
    });

    it('should generate consistent templates for all agents', async () => {
      // When: I run fspec init for multiple agents
      await installAgents(setup.testDir, ['claude', 'cursor']);

      const claudeSlashCmd = join(
        setup.testDir,
        '.claude',
        'commands',
        'fspec.md'
      );
      const cursorSlashCmd = join(
        setup.testDir,
        '.cursor',
        'commands',
        'fspec.md'
      );

      const claudeCmdContent = await readFile(claudeSlashCmd, 'utf-8');
      const cursorCmdContent = await readFile(cursorSlashCmd, 'utf-8');

      // Then: Both contain minimal embedded template content (header + two commands)
      expect(claudeCmdContent.length).toBeLessThan(700);
      expect(cursorCmdContent.length).toBeLessThan(700);

      expect(claudeCmdContent).toContain('fspec');
      expect(cursorCmdContent).toContain('fspec');
    });
  });

  describe('Scenario: Slash command files use correct format without YAML frontmatter', () => {
    it('should generate markdown files without YAML frontmatter', async () => {
      // Given: I run "fspec init --agent=claude"
      await installAgents(setup.testDir, ['claude']);

      // When: The slash command file is generated
      const slashCmdPath = join(
        setup.testDir,
        '.claude',
        'commands',
        'fspec.md'
      );
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
      await installAgents(setup.testDir, ['gemini']);

      // When: The slash command file is generated
      const slashCmdPath = join(
        setup.testDir,
        '.gemini',
        'commands',
        'fspec.toml'
      );
      const slashCmdContent = await readFile(slashCmdPath, 'utf-8');

      // Then: TOML files (Gemini, Qwen) should have [command] section with metadata
      expect(slashCmdContent).toContain('[command]');
      expect(slashCmdContent).toContain(
        'name = "fspec - Load Project Context"'
      );
      expect(slashCmdContent).toContain(
        'description = "Load fspec workflow and ACDD methodology"'
      );

      // And: TOML files should NOT have YAML frontmatter
      expect(slashCmdContent).not.toMatch(/^---\s*\n/);
    });
  });
});
