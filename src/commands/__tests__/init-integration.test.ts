/**
 * Feature: spec/features/init-008-and-bug-030-implementation-not-integrated-into-actual-codebase.feature
 *
 * Integration tests verifying utility functions are actually called by commands.
 * These tests MUST FAIL in RED phase before integration.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { existsSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { execSync } from 'child_process';

describe('Feature: INIT-008 and BUG-030 implementation integration', () => {
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

  describe('Scenario: fspec init calls writeAgentConfig() to create runtime config file', () => {
    it('should create spec/fspec-config.json with detected agent after init', async () => {
      // Given a new project directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // When fspec init completes with --agent claude flag
      try {
        execSync(
          `node ${join(process.cwd(), 'dist/index.js')} init --agent claude`,
          {
            cwd: testDir,
            stdio: 'pipe',
          }
        );
      } catch (error) {
        // Init might fail in test environment, but we're checking if config was created
      }

      // Then spec/fspec-config.json should be created
      const configPath = join(testDir, 'spec', 'fspec-config.json');
      const configExists = existsSync(configPath);

      // THIS WILL FAIL IN RED PHASE - config file not created yet
      expect(configExists).toBe(true);

      if (configExists) {
        const config = JSON.parse(await readFile(configPath, 'utf-8'));
        expect(config.agent).toBe('claude');
      }
    });
  });

  describe('Scenario: init.ts calls getActivationMessage() to show agent-specific instructions', () => {
    it('should show Claude-specific activation message in init output', async () => {
      // Given a new project directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // When fspec init completes with --agent claude flag
      let output = '';
      try {
        output = execSync(
          `node ${join(process.cwd(), 'dist/index.js')} init --agent claude`,
          {
            cwd: testDir,
            encoding: 'utf-8',
          }
        );
      } catch (error: any) {
        output = error.stdout || '';
      }

      // Then the output should contain Claude-specific activation message
      // THIS WILL FAIL IN RED PHASE - still shows generic message
      expect(output).toContain('Run /fspec in Claude Code to activate');
      expect(output).not.toContain('Run /fspec in your AI agent to activate');
    });

    it('should show Cursor-specific activation message for Cursor agent', async () => {
      // Given a new project directory
      await mkdir(join(testDir, 'spec'), { recursive: true });

      // When fspec init completes with --agent cursor flag
      let output = '';
      try {
        output = execSync(
          `node ${join(process.cwd(), 'dist/index.js')} init --agent cursor`,
          {
            cwd: testDir,
            encoding: 'utf-8',
          }
        );
      } catch (error: any) {
        output = error.stdout || '';
      }

      // Then the output should contain Cursor-specific activation message
      // THIS WILL FAIL IN RED PHASE - still shows generic message
      expect(output).toContain('Open .cursor/commands/ in Cursor to activate');
      expect(output).not.toContain('Run /fspec in your AI agent to activate');
    });
  });
});
