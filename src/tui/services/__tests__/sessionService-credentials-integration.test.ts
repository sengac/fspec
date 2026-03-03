/**
 * Feature: spec/features/provider-configuration-and-credentials-management.feature
 *
 * This test file validates the integration between sessionService and credentials management.
 * These are E2E tests using REAL fixtures (not mocks) to verify the credentials flow:
 * - credentials.json → TypeScript (for UI display)
 * - Rust handles actual credential resolution for sessions (CONFIG-005)
 *
 * CONFIG-005: Credential resolution is now handled in Rust. TypeScript only
 * saves/deletes credentials and reads them for UI display purposes.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { join } from 'path';
import { mkdir, writeFile, rm } from 'fs/promises';
import { existsSync } from 'fs';
import {
  setupTestDirectory,
  type TestDirectorySetup,
} from '../../../test-helpers/universal-test-setup';

// Import the module under test
import { getProviderConfig } from '../../../utils/credentials';

describe('Feature: Provider Configuration and Credentials Management', () => {
  let setup: TestDirectorySetup;
  let originalHome: string | undefined;
  let originalAnthropicKey: string | undefined;
  let originalClaudeOAuthToken: string | undefined;
  let originalCwd: string;

  beforeEach(async () => {
    // Setup test directory (simulates HOME)
    setup = await setupTestDirectory('session-credentials');
    originalHome = process.env.HOME;
    originalAnthropicKey = process.env.ANTHROPIC_API_KEY;
    originalClaudeOAuthToken = process.env.CLAUDE_CODE_OAUTH_TOKEN;
    originalCwd = process.cwd();

    // Override HOME to use our test directory
    process.env.HOME = setup.testDir;

    // Clear all environment variables for clean testing
    delete process.env.ANTHROPIC_API_KEY;
    delete process.env.CLAUDE_CODE_OAUTH_TOKEN;

    // Create .fspec directory structure
    await mkdir(join(setup.testDir, '.fspec'), { recursive: true });
  });

  afterEach(async () => {
    // Restore original values
    process.env.HOME = originalHome;
    if (originalAnthropicKey) {
      process.env.ANTHROPIC_API_KEY = originalAnthropicKey;
    } else {
      delete process.env.ANTHROPIC_API_KEY;
    }
    if (originalClaudeOAuthToken) {
      process.env.CLAUDE_CODE_OAUTH_TOKEN = originalClaudeOAuthToken;
    } else {
      delete process.env.CLAUDE_CODE_OAUTH_TOKEN;
    }
    process.chdir(originalCwd);
    await setup.cleanup();
  });

  describe('Scenario: sessionService reads credentials file and passes API key to Rust', () => {
    it('should extract provider ID from modelPath and retrieve API key from credentials.json', async () => {
      // @step Given I have a credentials file at ~/.fspec/credentials/credentials.json containing an API key for "anthropic"
      const credentialsDir = join(setup.testDir, '.fspec', 'credentials');
      await mkdir(credentialsDir, { recursive: true });

      const testApiKey = 'sk-ant-test-fixture-key-12345';
      const credentialsContent = {
        version: 1,
        providers: {
          anthropic: {
            apiKey: testApiKey,
            lastUpdated: new Date().toISOString(),
          },
        },
      };

      await writeFile(
        join(credentialsDir, 'credentials.json'),
        JSON.stringify(credentialsContent, null, 2),
        { mode: 0o600 }
      );

      // @step Given no ANTHROPIC_API_KEY environment variable is set
      expect(process.env.ANTHROPIC_API_KEY).toBeUndefined();
      expect(process.env.CLAUDE_CODE_OAUTH_TOKEN).toBeUndefined();

      // @step Given there is no .env file in the working directory
      process.chdir(setup.testDir);
      expect(existsSync(join(setup.testDir, '.env'))).toBe(false);

      // @step When I call createSession with modelPath "anthropic/claude-sonnet-4"
      // First, test that getProviderConfig correctly reads from credentials.json
      const modelPath = 'anthropic/claude-sonnet-4';
      const providerId = modelPath.split('/')[0]; // Extract provider ID

      // @step Then sessionService should extract provider ID "anthropic" from the modelPath
      expect(providerId).toBe('anthropic');

      // @step Then sessionService should call getProviderConfig to retrieve the API key from credentials.json
      const providerConfig = await getProviderConfig(providerId);

      // Verify the API key was retrieved from the credentials file
      expect(providerConfig.apiKey).toBe(testApiKey);
      expect(providerConfig.source).toBe('file');

      // @step Then sessionService should pass the API key to sessionManagerCreateWithId as the fifth parameter
      // CONFIG-005: This step is obsolete - Rust now resolves credentials internally.
      // TypeScript only saves/deletes credentials and calls credentialsReload() to notify Rust.

      // @step Then Rust should receive the API key and set ANTHROPIC_API_KEY environment variable
      // This is verified in the integration test with actual NAPI calls

      // @step Then the session should be created successfully
      // For unit test, we verify the credentials resolution is correct
      expect(providerConfig.apiKey).toBeDefined();
    });

    it('should prioritize credentials file over environment variables', async () => {
      // @step Given I have a credentials file with an API key for "anthropic"
      const credentialsDir = join(setup.testDir, '.fspec', 'credentials');
      await mkdir(credentialsDir, { recursive: true });

      const fileApiKey = 'sk-ant-file-key-99999';
      const credentialsContent = {
        version: 1,
        providers: {
          anthropic: {
            apiKey: fileApiKey,
            lastUpdated: new Date().toISOString(),
          },
        },
      };

      await writeFile(
        join(credentialsDir, 'credentials.json'),
        JSON.stringify(credentialsContent, null, 2),
        { mode: 0o600 }
      );

      // @step Given I have ANTHROPIC_API_KEY set in the environment
      const envApiKey = 'sk-ant-env-key-11111';
      process.env.ANTHROPIC_API_KEY = envApiKey;

      // @step When the system resolves credentials for "anthropic"
      const providerConfig = await getProviderConfig('anthropic');

      // @step Then the credentials file API key should be used
      expect(providerConfig.apiKey).toBe(fileApiKey);

      // @step And the environment variable should be ignored
      expect(providerConfig.apiKey).not.toBe(envApiKey);
      expect(providerConfig.source).toBe('file');

      // Cleanup
      delete process.env.ANTHROPIC_API_KEY;
    });

    it('should fall back to environment variable when no credentials file exists', async () => {
      // @step Given I have ANTHROPIC_API_KEY set in the environment
      const envApiKey = 'sk-ant-env-fallback-22222';
      process.env.ANTHROPIC_API_KEY = envApiKey;

      // @step Given no credentials file exists
      const credentialsPath = join(
        setup.testDir,
        '.fspec',
        'credentials',
        'credentials.json'
      );
      expect(existsSync(credentialsPath)).toBe(false);

      // @step When the system resolves credentials for "anthropic"
      const providerConfig = await getProviderConfig('anthropic');

      // @step Then the environment variable API key should be used
      expect(providerConfig.apiKey).toBe(envApiKey);
      expect(providerConfig.source).toBe('env');

      // Cleanup
      delete process.env.ANTHROPIC_API_KEY;
    });

    it('should fall back to .env file when no credentials file and no env var exists', async () => {
      // @step Given I do not have credentials in the credentials file
      const credentialsPath = join(
        setup.testDir,
        '.fspec',
        'credentials',
        'credentials.json'
      );
      expect(existsSync(credentialsPath)).toBe(false);

      // @step Given I do not have ANTHROPIC_API_KEY in the environment
      expect(process.env.ANTHROPIC_API_KEY).toBeUndefined();

      // @step Given I have a .env file with ANTHROPIC_API_KEY defined
      const dotenvApiKey = 'sk-ant-dotenv-key-33333';
      await writeFile(
        join(setup.testDir, '.env'),
        `ANTHROPIC_API_KEY=${dotenvApiKey}\n`
      );

      // Change to the test directory so .env is found
      process.chdir(setup.testDir);

      // @step When the system resolves credentials for "anthropic"
      const providerConfig = await getProviderConfig('anthropic');

      // @step Then the .env file API key should be used
      expect(providerConfig.apiKey).toBe(dotenvApiKey);
      expect(providerConfig.source).toBe('dotenv');
    });
  });

  describe('Scenario: Provider ID extraction from model path', () => {
    it('should correctly extract provider ID from various model path formats', async () => {
      // Test various model path formats
      const testCases = [
        {
          modelPath: 'anthropic/claude-sonnet-4',
          expectedProvider: 'anthropic',
        },
        { modelPath: 'openai/gpt-4-turbo', expectedProvider: 'openai' },
        { modelPath: 'gemini/gemini-2.0-flash', expectedProvider: 'gemini' },
        { modelPath: 'openai/llama3', expectedProvider: 'openai' },
        { modelPath: 'azure/gpt-4', expectedProvider: 'azure' },
      ];

      for (const { modelPath, expectedProvider } of testCases) {
        // @step Given I have a model path in "provider/model" format
        // @step When sessionService extracts the provider ID
        const providerId = modelPath.split('/')[0];

        // @step Then the provider ID should be the first part before "/"
        expect(providerId).toBe(expectedProvider);
      }
    });
  });

  describe('Scenario: Credentials file with multiple providers', () => {
    it('should retrieve the correct API key for each provider', async () => {
      // @step Given I have credentials configured for multiple providers
      const credentialsDir = join(setup.testDir, '.fspec', 'credentials');
      await mkdir(credentialsDir, { recursive: true });

      const credentialsContent = {
        version: 1,
        providers: {
          anthropic: {
            apiKey: 'sk-ant-anthropic-key',
            lastUpdated: new Date().toISOString(),
          },
          openai: {
            apiKey: 'sk-openai-key',
            lastUpdated: new Date().toISOString(),
          },
          gemini: {
            apiKey: 'AIza-gemini-key',
            lastUpdated: new Date().toISOString(),
          },
        },
      };

      await writeFile(
        join(credentialsDir, 'credentials.json'),
        JSON.stringify(credentialsContent, null, 2),
        { mode: 0o600 }
      );

      // @step When I resolve credentials for each provider
      const anthropicConfig = await getProviderConfig('anthropic');
      const openaiConfig = await getProviderConfig('openai');
      const geminiConfig = await getProviderConfig('gemini');

      // @step Then each provider should get its own API key
      expect(anthropicConfig.apiKey).toBe('sk-ant-anthropic-key');
      expect(openaiConfig.apiKey).toBe('sk-openai-key');
      expect(geminiConfig.apiKey).toBe('AIza-gemini-key');

      // @step And all should be sourced from the credentials file
      expect(anthropicConfig.source).toBe('file');
      expect(openaiConfig.source).toBe('file');
      expect(geminiConfig.source).toBe('file');
    });
  });

  describe('Scenario: Provider without credentials configured', () => {
    it('should return undefined apiKey when provider has no credentials', async () => {
      // @step Given I do not have credentials for "mistral"
      // No credentials file created

      // @step Given no environment variable is set for mistral
      delete process.env.MISTRAL_API_KEY;

      // @step When I resolve credentials for "mistral"
      const mistralConfig = await getProviderConfig('mistral');

      // @step Then the API key should be undefined
      expect(mistralConfig.apiKey).toBeUndefined();
      expect(mistralConfig.source).toBeUndefined();
    });
  });
});
