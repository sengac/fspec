/**
 * Feature: spec/features/fix-codex-cli-file-paths-in-fspec-init.feature
 *
 * Tests for Codex CLI file path corrections
 */

import { describe, it, expect } from 'vitest';
import { getAgentById } from '../agentRegistry';
import fs from 'fs';
import path from 'path';

describe('Feature: Fix Codex CLI file paths in fspec init', () => {
  describe('Scenario: Codex CLI agent registry uses correct documentation path', () => {
    it('should have docTemplate set to AGENTS.md for codex-cli', () => {
      // Given the agentRegistry.ts file exists
      // When I check the codex-cli agent configuration
      const codexCliConfig = getAgentById('codex-cli');

      // Then the docTemplate should be "AGENTS.md"
      expect(codexCliConfig).toBeDefined();
      expect(codexCliConfig?.docTemplate).toBe('AGENTS.md');

      // And the docTemplate should not be "CODEX-CLI.md"
      expect(codexCliConfig?.docTemplate).not.toBe('CODEX-CLI.md');
    });
  });

  describe('Scenario: Codex CLI agent registry uses correct prompt file path', () => {
    it('should have promptPath set to .codex/prompts/fspec.md for codex-cli', () => {
      // Given the agentRegistry.ts file exists
      // When I check the codex-cli agent configuration
      const codexCliConfig = getAgentById('codex-cli');

      // Then the promptPath should be ".codex/prompts/fspec.md"
      expect(codexCliConfig).toBeDefined();
      expect(codexCliConfig?.slashCommandPath).toBe('.codex/prompts/');

      // And the promptPath should not be ".codex/commands/fspec.md"
      expect(codexCliConfig?.slashCommandPath).not.toBe('.codex/commands/');
    });
  });

  describe('Scenario: Gitignore excludes correct Codex CLI prompt path', () => {
    it('should contain .codex/prompts/fspec.md in .gitignore', () => {
      // Given the .gitignore file exists
      const gitignorePath = path.join(process.cwd(), '.gitignore');
      const gitignoreContent = fs.readFileSync(gitignorePath, 'utf-8');

      // When I check the gitignore entries
      // Then it should contain ".codex/prompts/fspec.md"
      expect(gitignoreContent).toContain('.codex/prompts/fspec.md');

      // And it should not contain ".codex/commands/fspec.md"
      expect(gitignoreContent).not.toContain('.codex/commands/fspec.md');
    });
  });
});
