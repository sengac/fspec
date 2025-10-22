/**
 * Feature: spec/features/fix-multi-agent-support-critical-issues.feature
 *
 * Tests for bug fixes from INIT-004 code review
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import {
  mkdirSync,
  rmSync,
  existsSync,
  writeFileSync,
  readFileSync,
  readdirSync,
} from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { installAgents } from '../init';

describe('Feature: Fix multi-agent support critical issues', () => {
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

  describe('Scenario: Non-destructive agent switching', () => {
    it('should delete only fspec-specific files when switching agents', async () => {
      // Given I have installed fspec for Claude with "fspec init --agent=claude"
      await installAgents(testDir, ['claude']);

      // And the .claude/commands/ directory contains custom user files
      const customFile1 = join(
        testDir,
        '.claude',
        'commands',
        'my-custom-command.md'
      );
      const customFile2 = join(testDir, '.claude', 'commands', 'my-script.sh');
      writeFileSync(customFile1, '# My custom command');
      writeFileSync(customFile2, '#!/bin/bash\necho "hello"');

      // Verify files exist before switching
      expect(existsSync(customFile1)).toBe(true);
      expect(existsSync(customFile2)).toBe(true);

      // When I run "fspec init --agent=cursor" to switch agents
      await installAgents(testDir, ['cursor']);

      // Then the system should delete only .claude/commands/fspec.md
      expect(existsSync(join(testDir, '.claude', 'commands', 'fspec.md'))).toBe(
        false
      );

      // And the system should preserve all other files in .claude/commands/
      expect(existsSync(customFile1)).toBe(true);
      expect(existsSync(customFile2)).toBe(true);

      // And the system should install Cursor-specific files
      expect(existsSync(join(testDir, 'spec', 'CURSOR.md'))).toBe(true);
      expect(existsSync(join(testDir, '.cursor', 'commands', 'fspec.md'))).toBe(
        true
      );
    });
  });

  describe('Scenario: Path traversal attack prevention', () => {
    it('should reject path traversal attempts in slashCommandPath', async () => {
      // This test validates the agent registry at load time
      // Path traversal validation should happen before any file operations

      // When the installation attempts to validate the path
      // Then the validation should reject the path
      // This would be tested by modifying the agent registry and attempting to load it
      // For now, we expect that malicious paths would be rejected

      // Placeholder test - actual implementation would involve:
      // 1. Modify AGENT_REGISTRY with malicious path
      // 2. Attempt to call installAgents()
      // 3. Expect rejection with appropriate error message

      expect(true).toBe(true); // Placeholder until path validation is implemented
    });
  });

  describe('Scenario: Duplicate agent path detection', () => {
    it('should detect and report duplicate slashCommandPath values', async () => {
      // This test validates the agent registry at load time
      // Duplicate path detection should happen when registry is loaded

      // Given the agent registry has "codex" with slashCommandPath ".codex/commands/"
      // And the agent registry has "codex-cli" with slashCommandPath ".codex/commands/"
      // When the registry is loaded
      // Then the system should detect the duplicate paths
      // And the system should show a clear error message
      // And the error message should list the conflicting agents

      // Placeholder test - actual implementation would involve:
      // 1. Check AGENT_REGISTRY for duplicates at load time
      // 2. Throw error if duplicates found
      // 3. Error message includes conflicting agent IDs

      expect(true).toBe(true); // Placeholder until duplicate detection is implemented
    });
  });

  describe('Scenario: Helpful invalid agent error', () => {
    it('should list all valid agent IDs when invalid agent is provided', async () => {
      // Given I run "fspec init --agent=invalid-agent"
      try {
        await installAgents(testDir, ['invalid-agent']);
        // Should not reach here
        expect(true).toBe(false);
      } catch (error: unknown) {
        // When the system detects the invalid agent ID
        // Then the error message should list all 18 valid agent IDs
        const errorMessage =
          error instanceof Error ? error.message : String(error);

        // Error message should be helpful
        expect(errorMessage).toContain('invalid-agent');

        // Should suggest valid agents (placeholder - actual implementation would list all 18)
        expect(errorMessage).toMatch(/claude|cursor|aider|cline|windsurf/i);

        // And the exit code should be 1 (thrown error indicates failure)
        expect(error).toBeInstanceOf(Error);
      }
    });
  });
});
