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

    it('should not throw errors about __dirname or missing template files', async () => {
      // Given: fspec is bundled (no access to .claude/commands/fspec.md or spec/templates/base/AGENT.md)
      // When: I run fspec init
      const result = installAgents(testDir, ['claude']);

      // Then: Command should not reject with filesystem errors
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
});
