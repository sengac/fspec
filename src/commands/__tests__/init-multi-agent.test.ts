/**
 * Feature: spec/features/support-multiple-ai-agents-beyond-claude.feature
 *
 * Tests for Multi-Agent Init Command
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdirSync, rmSync, existsSync, readFileSync, writeFileSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { installAgents, installAgentFiles } from '../init';

describe('Feature: Support multiple AI agents beyond Claude', () => {
  let testDir: string;

  beforeEach(() => {
    testDir = join(tmpdir(), `fspec-test-${Date.now()}`);
    mkdirSync(testDir, { recursive: true });
  });

  afterEach(() => {
    if (existsSync(testDir)) {
      rmSync(testDir, { recursive: true, force: true });
    }
  });

  describe('Scenario: Install Aider-specific documentation', () => {
    it('should create spec/AIDER.md when installing for Aider', async () => {
      // Given I am in a project directory
      // When I run "fspec init --agent=aider"
      await installAgents(testDir, ['aider']);

      // Then a file "spec/AIDER.md" should be created
      const aiderDocPath = join(testDir, 'spec', 'AIDER.md');
      expect(existsSync(aiderDocPath)).toBe(true);

      // And the file should contain comprehensive Project Management Guidelines
      const content = readFileSync(aiderDocPath, 'utf-8');
      const lineCount = content.split('\n').length;
      expect(lineCount).toBeGreaterThan(2000); // Comprehensive Project Management Guidelines, not a stub

      // And system-reminder tags should not appear (Aider doesn't support them)
      expect(content).not.toContain('<system-reminder>');

      // And meta-cognitive prompts should be removed (Aider is CLI-only)
      expect(content).not.toContain('ultrathink');
    });
  });

  describe('Scenario: Install Cursor slash command', () => {
    it('should create .cursor/commands/fspec.md when installing for Cursor', async () => {
      // When I run "fspec init --agent=cursor"
      await installAgents(testDir, ['cursor']);

      // Then a file ".cursor/commands/fspec.md" should be created
      const slashCommandPath = join(testDir, '.cursor', 'commands', 'fspec.md');
      expect(existsSync(slashCommandPath)).toBe(true);

      // And the file should contain comprehensive ACDD workflow documentation
      const content = readFileSync(slashCommandPath, 'utf-8');
      expect(content).toContain('ACDD');
      expect(content).toContain('Example Mapping');

      // And the file should be plain Markdown WITHOUT YAML frontmatter
      expect(content).toMatch(/^# fspec Command/);
    });
  });

  describe('Scenario: Install comprehensive slash command documentation', () => {
    it('should create comprehensive fspec.md with workflow documentation', async () => {
      await installAgents(testDir, ['cursor']);

      const slashCommandPath = join(testDir, '.cursor', 'commands', 'fspec.md');
      const content = readFileSync(slashCommandPath, 'utf-8');

      // Then the file should be at least 1000 lines long
      const lines = content.split('\n');
      expect(lines.length).toBeGreaterThanOrEqual(1000);

      // And the file should contain ACDD workflow documentation
      expect(content).toContain('ACDD');
      expect(content).toContain('Acceptance Criteria Driven Development');

      // And the file should contain Example Mapping documentation
      expect(content).toContain('Example Mapping');

      // And the file should contain coverage tracking documentation
      expect(content).toContain('coverage');

      // And a file "spec/CURSOR.md" should be created
      const cursorDocPath = join(testDir, 'spec', 'CURSOR.md');
      expect(existsSync(cursorDocPath)).toBe(true);
    });
  });

  describe('Scenario: Install multiple agents simultaneously', () => {
    it('should install both Cursor and Aider when specified', async () => {
      // When I run "fspec init --agent=cursor --agent=aider"
      await installAgents(testDir, ['cursor', 'aider']);

      // Then a file "spec/CURSOR.md" should be created
      expect(existsSync(join(testDir, 'spec', 'CURSOR.md'))).toBe(true);

      // And a file "spec/AIDER.md" should be created
      expect(existsSync(join(testDir, 'spec', 'AIDER.md'))).toBe(true);

      // And a file ".cursor/commands/fspec.md" should be created
      expect(existsSync(join(testDir, '.cursor', 'commands', 'fspec.md'))).toBe(
        true
      );

      // And both documentation files should contain comprehensive content
      const cursorContent = readFileSync(
        join(testDir, 'spec', 'CURSOR.md'),
        'utf-8'
      );
      const aiderContent = readFileSync(
        join(testDir, 'spec', 'AIDER.md'),
        'utf-8'
      );

      // Both should be comprehensive Project Management Guidelines (not stubs)
      expect(cursorContent.split('\n').length).toBeGreaterThan(2000);
      expect(aiderContent.split('\n').length).toBeGreaterThan(2000);

      // Both should contain Project Management Guidelines content
      expect(cursorContent).toContain('Project Management and Specification Guidelines');
      expect(aiderContent).toContain('Project Management and Specification Guidelines');
    });
  });

  describe('Scenario: Agent switching (idempotent behavior)', () => {
    it('should remove Claude files and create Cursor files when switching', async () => {
      // Given I have previously run "fspec init --agent=claude"
      await installAgents(testDir, ['claude']);

      // And files exist
      const claudeMd = join(testDir, 'CLAUDE.md');
      const specClaudeMd = join(testDir, 'spec', 'CLAUDE.md');
      const claudeSlashCmd = join(testDir, '.claude', 'commands', 'fspec.md');

      expect(existsSync(claudeMd)).toBe(true);
      expect(existsSync(specClaudeMd)).toBe(true);
      expect(existsSync(claudeSlashCmd)).toBe(true);

      // When I run "fspec init --agent=cursor"
      await installAgents(testDir, ['cursor']);

      // Then Claude-specific files should be removed
      expect(existsSync(claudeMd)).toBe(false);
      expect(existsSync(specClaudeMd)).toBe(false);
      expect(existsSync(claudeSlashCmd)).toBe(false);

      // And Cursor-specific files should be created
      expect(existsSync(join(testDir, 'CURSOR.md'))).toBe(true);
      expect(existsSync(join(testDir, 'spec', 'CURSOR.md'))).toBe(true);
      expect(existsSync(join(testDir, '.cursor', 'commands', 'fspec.md'))).toBe(
        true
      );
    });
  });

  describe('Scenario: Non-destructive installation', () => {
    it('should preserve user files when installing fspec files', async () => {
      // Given a file ".cursor/commands/my-custom-command.md" exists
      const customFile = join(
        testDir,
        '.cursor',
        'commands',
        'my-custom-command.md'
      );
      mkdirSync(join(testDir, '.cursor', 'commands'), { recursive: true });
      writeFileSync(
        customFile,
        '# My Custom Command\n\nThis is my custom file.'
      );

      // When I run "fspec init --agent=cursor"
      await installAgents(testDir, ['cursor']);

      // Then a file ".cursor/commands/fspec.md" should be created
      expect(existsSync(join(testDir, '.cursor', 'commands', 'fspec.md'))).toBe(
        true
      );

      // And the file ".cursor/commands/my-custom-command.md" should still exist
      expect(existsSync(customFile)).toBe(true);

      // And no user files should be deleted or modified
      const customContent = readFileSync(customFile, 'utf-8');
      expect(customContent).toContain('My Custom Command');
      expect(customContent).toContain('This is my custom file.');
    });
  });

  describe('Scenario: Invalid agent ID error', () => {
    it('should exit with code 1 and show error for invalid agent', async () => {
      // When I run "fspec init --agent=invalid-agent"
      await expect(installAgents(testDir, ['invalid-agent'])).rejects.toThrow();

      // Then no files should be created
      expect(existsSync(join(testDir, 'spec'))).toBe(false);
    });
  });

  describe('Scenario: Generate TOML format for Gemini CLI', () => {
    it('should create .gemini/commands/fspec.toml with TOML format', async () => {
      // When I run "fspec init --agent=gemini"
      await installAgents(testDir, ['gemini']);

      // Then a file ".gemini/commands/fspec.toml" should be created
      const tomlPath = join(testDir, '.gemini', 'commands', 'fspec.toml');
      expect(existsSync(tomlPath)).toBe(true);

      // And the file should use TOML format
      const content = readFileSync(tomlPath, 'utf-8');
      expect(content).toMatch(/\[command\]/);
      expect(content).toMatch(/name\s*=/);

      // And the file should contain workflow documentation
      expect(content).toContain('ACDD');
    });
  });

  describe('Scenario: Install root stub for auto-loading agents', () => {
    it('should create root stub file with pointer to full documentation', async () => {
      // When I run "fspec init --agent=cursor"
      await installAgents(testDir, ['cursor']);

      // Then a file "CURSOR.md" should be created in the project root
      const rootStub = join(testDir, 'CURSOR.md');
      expect(existsSync(rootStub)).toBe(true);

      // And the file should be a short pointer to "spec/CURSOR.md"
      const content = readFileSync(rootStub, 'utf-8');
      expect(content).toContain('spec/CURSOR.md');

      // And the file should contain a quick start guide
      expect(content.length).toBeLessThan(1000); // Short stub file
      expect(content).toMatch(/quick|start|guide/i);
    });
  });
});
