/**
 * Feature: spec/features/shared-configuration-management-utilities-for-user-and-project-config-files.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { homedir } from 'os';
import { loadConfig, writeConfig } from '../config';

describe('Feature: Shared configuration management utilities', () => {
  let testDir: string;
  let originalHome: string;

  beforeEach(async () => {
    // Create temporary test directory
    testDir = join(tmpdir(), `fspec-test-${Date.now()}`);
    await mkdir(testDir, { recursive: true });

    // Mock home directory for testing
    originalHome = process.env.HOME || '';
    process.env.HOME = testDir;
  });

  afterEach(async () => {
    // Restore original home
    process.env.HOME = originalHome;

    // Cleanup test directory
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Load config when no config files exist', () => {
    it('should return empty object when no config files exist', async () => {
      // Given neither user-level nor project-level config files exist
      const cwd = join(testDir, 'project');
      await mkdir(cwd, { recursive: true });

      // When I call loadConfig()
      const config = await loadConfig(cwd);

      // Then it should return an empty object {}
      expect(config).toEqual({});

      // And no errors should be thrown (implicit - test would fail if error thrown)
    });
  });

  describe('Scenario: Load config from user-level only', () => {
    it('should load config from user-level when project-level does not exist', async () => {
      // Given user-level config at ~/.fspec/fspec-config.json contains {"timeout": 60}
      const userConfigDir = join(testDir, '.fspec');
      await mkdir(userConfigDir, { recursive: true });
      await writeFile(
        join(userConfigDir, 'fspec-config.json'),
        JSON.stringify({ timeout: 60 })
      );

      // And project-level config does not exist
      const cwd = join(testDir, 'project');
      await mkdir(cwd, { recursive: true });

      // When I call loadConfig()
      const config = await loadConfig(cwd);

      // Then it should return {"timeout": 60}
      expect(config).toEqual({ timeout: 60 });
    });
  });

  describe('Scenario: Deep merge user-level and project-level config', () => {
    it('should deep merge user and project config with project overriding', async () => {
      // Given user-level config contains {"research": {"timeout": 60, "tools": ["perplexity"]}}
      const userConfigDir = join(testDir, '.fspec');
      await mkdir(userConfigDir, { recursive: true });
      await writeFile(
        join(userConfigDir, 'fspec-config.json'),
        JSON.stringify({ research: { timeout: 60, tools: ['perplexity'] } })
      );

      // And project-level config contains {"research": {"tools": ["jira"]}}
      const cwd = join(testDir, 'project');
      const projectConfigDir = join(cwd, 'spec');
      await mkdir(projectConfigDir, { recursive: true });
      await writeFile(
        join(projectConfigDir, 'fspec-config.json'),
        JSON.stringify({ research: { tools: ['jira'] } })
      );

      // When I call loadConfig()
      const config = await loadConfig(cwd);

      // Then it should return {"research": {"timeout": 60, "tools": ["jira"]}}
      expect(config).toEqual({ research: { timeout: 60, tools: ['jira'] } });

      // And the timeout setting from user-level should be preserved
      expect(config.research.timeout).toBe(60);

      // And the tools setting from project-level should override user-level
      expect(config.research.tools).toEqual(['jira']);
    });
  });

  describe('Scenario: Throw error on invalid JSON syntax', () => {
    it('should throw error with helpful message on invalid JSON', async () => {
      // Given project-level config has invalid JSON syntax (missing bracket)
      const cwd = join(testDir, 'project');
      const projectConfigDir = join(cwd, 'spec');
      await mkdir(projectConfigDir, { recursive: true });
      await writeFile(
        join(projectConfigDir, 'fspec-config.json'),
        '{"invalid": "json"' // Missing closing bracket
      );

      // When I call loadConfig()
      // Then it should throw an error
      await expect(loadConfig(cwd)).rejects.toThrow();

      // And the error message should contain "Invalid JSON in spec/fspec-config.json"
      await expect(loadConfig(cwd)).rejects.toThrow(
        /Invalid JSON.*spec\/fspec-config\.json/
      );

      // And the error message should contain parse error details
      await expect(loadConfig(cwd)).rejects.toThrow(
        /(Unexpected token|Expected|position|column)/
      );
    });
  });

  describe('Scenario: Write config to user-level scope', () => {
    it('should write config to user-level with proper formatting', async () => {
      // Given I want to save personal defaults
      const cwd = join(testDir, 'project');
      await mkdir(cwd, { recursive: true });

      // When I call writeConfig('user', {"newSetting": true})
      await writeConfig('user', { newSetting: true });

      // Then it should write to ~/.fspec/fspec-config.json
      const userConfigPath = join(testDir, '.fspec', 'fspec-config.json');
      const { readFile } = await import('fs/promises');
      const fileContent = await readFile(userConfigPath, 'utf-8');

      // And the file should contain properly formatted JSON
      expect(fileContent).toBeTruthy();

      // And the file should be readable as valid JSON
      const parsed = JSON.parse(fileContent);
      expect(parsed).toEqual({ newSetting: true });
    });
  });
});
