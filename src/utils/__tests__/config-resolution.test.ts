/**
 * Feature: spec/features/configuration-management-with-tui-integration.feature
 *
 * Tests for multi-layer config resolution system (RES-012 Phase 1).
 * Tests config priority: ENV vars → User config → Project config → Defaults
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import fs from 'fs';
import path from 'path';
import os from 'os';
import { resolveConfig, validateConfig } from '../config-resolution';

describe('Feature: Configuration Management with TUI Integration', () => {
  let tempDir: string;
  let originalEnv: Record<string, string | undefined>;
  let userConfigPath: string;
  let projectConfigPath: string;

  beforeEach(() => {
    // Save original environment
    originalEnv = { ...process.env };

    // Create temp directory for test configs
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'fspec-config-test-'));

    // Set up config paths
    userConfigPath = path.join(tempDir, '.fspec', 'fspec-config.json');
    projectConfigPath = path.join(tempDir, 'spec', 'fspec-config.json');

    // Create directories
    fs.mkdirSync(path.dirname(userConfigPath), { recursive: true });
    fs.mkdirSync(path.dirname(projectConfigPath), { recursive: true });
  });

  afterEach(() => {
    // Restore environment
    process.env = originalEnv;

    // Clean up temp directory
    if (fs.existsSync(tempDir)) {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  });

  describe('Scenario: Environment variable overrides user config file', () => {
    it('should use environment variable when both env var and user config exist', () => {
      // @step Given I have Perplexity API key "user-key" in ~/.fspec/fspec-config.json
      const userConfig = {
        research: {
          perplexity: {
            apiKey: 'user-key',
          },
        },
      };
      fs.writeFileSync(userConfigPath, JSON.stringify(userConfig, null, 2));

      // @step And I have PERPLEXITY_API_KEY environment variable set to "env-key"
      process.env.PERPLEXITY_API_KEY = 'env-key';

      // @step When the config resolution system loads Perplexity configuration
      const config = resolveConfig('perplexity', {
        userConfigPath,
        projectConfigPath,
      });

      // @step Then the API key should be "env-key"
      expect(config).toBeDefined();
      expect(config.apiKey).toBe('env-key');

      // @step And the config source should be "ENV"
      expect(config.source).toBe('ENV');
    });
  });

  describe("Scenario: Project config used when env var and user config don't exist", () => {
    it('should use project config when no env var or user config exists', () => {
      // @step Given I have no JIRA_URL environment variable set
      delete process.env.JIRA_URL;

      // @step And I have no ~/.fspec/fspec-config.json file
      if (fs.existsSync(userConfigPath)) {
        fs.unlinkSync(userConfigPath);
      }

      // @step And I have Jira URL "https://company.atlassian.net" in spec/fspec-config.json
      const projectConfig = {
        research: {
          jira: {
            url: 'https://company.atlassian.net',
          },
        },
      };
      fs.writeFileSync(
        projectConfigPath,
        JSON.stringify(projectConfig, null, 2)
      );

      // @step When the config resolution system loads Jira configuration
      const config = resolveConfig('jira', {
        userConfigPath,
        projectConfigPath,
      });

      // @step Then the Jira URL should be "https://company.atlassian.net"
      expect(config).toBeDefined();
      expect(config.url).toBe('https://company.atlassian.net');

      // @step And the config source should be "PROJECT"
      expect(config.source).toBe('PROJECT');
    });
  });

  describe('Scenario: .env file loads environment variables with precedence over config files', () => {
    it('should load .env file and use it with ENV priority', () => {
      // @step Given I have a .env file in project root with PERPLEXITY_API_KEY=pplx-abc123
      const envPath = path.join(tempDir, '.env');
      fs.writeFileSync(envPath, 'PERPLEXITY_API_KEY=pplx-abc123\n');

      // @step And I have Perplexity API key "user-key" in ~/.fspec/fspec-config.json
      const userConfig = {
        research: {
          perplexity: {
            apiKey: 'user-key',
          },
        },
      };
      fs.writeFileSync(userConfigPath, JSON.stringify(userConfig, null, 2));

      // @step When the config resolution system loads with dotenv support
      const config = resolveConfig('perplexity', {
        userConfigPath,
        projectConfigPath,
        envPath,
      });

      // @step Then the API key should be "pplx-abc123"
      expect(config).toBeDefined();
      expect(config.apiKey).toBe('pplx-abc123');

      // @step And the config source should be "ENV"
      expect(config.source).toBe('ENV');
    });
  });

  describe('Scenario: Validation detects missing required configuration', () => {
    it('should fail validation when required config is missing', () => {
      // @step Given I have no CONFLUENCE_TOKEN environment variable set
      delete process.env.CONFLUENCE_TOKEN;

      // @step And I have no Confluence configuration in any config file
      // (no config files created - they don't exist)

      // @step When I attempt to use the Confluence research tool
      let validationError: Error | null = null;
      try {
        validateConfig('confluence', { userConfigPath, projectConfigPath });
      } catch (error) {
        validationError = error as Error;
      }

      // @step Then validation should fail with error "Missing required configuration: CONFLUENCE_TOKEN"
      expect(validationError).toBeDefined();
      expect(validationError!.message).toContain(
        'Missing required configuration'
      );
      expect(validationError!.message).toContain('CONFLUENCE_TOKEN');

      // @step And the error message should suggest how to configure the tool
      expect(validationError!.message).toMatch(/configure|set|add/i);
    });
  });

  describe('Scenario: Default values used when no configuration exists', () => {
    it('should use default values when no config source provides them', () => {
      // @step Given I have no PERPLEXITY_MODEL environment variable set
      delete process.env.PERPLEXITY_MODEL;

      // @step And I have no Perplexity model setting in any config file
      // (no config files created - they don't exist)

      // @step When the config resolution system loads Perplexity configuration
      const config = resolveConfig('perplexity', {
        userConfigPath,
        projectConfigPath,
      });

      // @step Then the model should be "sonar"
      expect(config).toBeDefined();
      expect(config.model).toBe('sonar');

      // @step And the config source should be "DEFAULT"
      expect(config.source).toBe('DEFAULT');
    });
  });
});
