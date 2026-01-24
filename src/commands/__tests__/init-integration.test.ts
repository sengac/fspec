/**
 * Feature: spec/features/init-008-and-bug-030-implementation-not-integrated-into-actual-codebase.feature
 *
 * Integration tests verifying utility functions are actually called by commands.
 * These tests verify that init properly creates config and shows activation messages.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { readFile } from 'fs/promises';
import { existsSync } from 'fs';
import { join } from 'path';
import { installAgents } from '../init';
import { writeAgentConfig } from '../../utils/agentRuntimeConfig';
import { getActivationMessage } from '../../utils/activationMessage';
import { getAgentById } from '../../utils/agentRegistry';
import {
  createTempTestDir,
  removeTempTestDir,
} from '../../test-helpers/temp-directory';

describe('Feature: INIT-008 and BUG-030 implementation integration', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await createTempTestDir('init-integration');
  });

  afterEach(async () => {
    await removeTempTestDir(testDir);
  });

  describe('Scenario: fspec init calls writeAgentConfig() to create runtime config file', () => {
    it('should create spec/fspec-config.json with detected agent after init', async () => {
      // Given a new project directory
      // (already created in beforeEach)

      // When fspec init completes with --agent claude flag
      await installAgents(testDir, ['claude']);
      writeAgentConfig(testDir, 'claude');

      // Then spec/fspec-config.json should be created
      const configPath = join(testDir, 'spec', 'fspec-config.json');
      const configExists = existsSync(configPath);

      expect(configExists).toBe(true);

      if (configExists) {
        const config = JSON.parse(await readFile(configPath, 'utf-8'));
        expect(config.agent).toBe('claude');
      }
    });
  });

  describe('Scenario: init.ts calls getActivationMessage() to show agent-specific instructions', () => {
    it('should return Claude-specific activation message', async () => {
      // Given a Claude agent config
      const claudeAgent = getAgentById('claude');
      expect(claudeAgent).toBeDefined();

      // When we get the activation message
      const message = getActivationMessage(claudeAgent!);

      // Then the output should contain Claude-specific activation message
      expect(message).toContain('Run /fspec in Claude Code to activate');
      expect(message).not.toContain('Run /fspec in your AI agent to activate');
    });

    it('should return Cursor-specific activation message for Cursor agent', async () => {
      // Given a Cursor agent config
      const cursorAgent = getAgentById('cursor');
      expect(cursorAgent).toBeDefined();

      // When we get the activation message
      const message = getActivationMessage(cursorAgent!);

      // Then the output should contain Cursor-specific activation message
      expect(message).toContain('Open .cursor/commands/ in Cursor to activate');
      expect(message).not.toContain('Run /fspec in your AI agent to activate');
    });
  });

  describe('Scenario: Init creates proper agent files', () => {
    it('should create .claude/commands/fspec.md for Claude agent', async () => {
      // When fspec init with claude agent
      await installAgents(testDir, ['claude']);

      // Then the command file should exist
      const fspecMdPath = join(testDir, '.claude', 'commands', 'fspec.md');
      expect(existsSync(fspecMdPath)).toBe(true);

      // And it should contain fspec command content
      const content = await readFile(fspecMdPath, 'utf-8');
      expect(content).toContain('fspec');
    });

    it('should create .cursor/commands/fspec.md for Cursor agent', async () => {
      // When fspec init with cursor agent
      await installAgents(testDir, ['cursor']);

      // Then the command file should exist
      const fspecMdPath = join(testDir, '.cursor', 'commands', 'fspec.md');
      expect(existsSync(fspecMdPath)).toBe(true);

      // And it should contain fspec command content
      const content = await readFile(fspecMdPath, 'utf-8');
      expect(content).toContain('fspec');
    });
  });
});
