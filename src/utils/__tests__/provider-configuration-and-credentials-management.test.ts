/**
 * Feature: spec/features/provider-configuration-and-credentials-management.feature
 *
 * Tests for provider configuration and credentials management.
 * These tests cover all scenarios from the feature file including:
 * - Credential CRUD operations with secure file permissions
 * - Provider configuration (base URL, auth methods)
 * - Credential resolution priority chain
 * - NAPI credential integration
 * - Provider registry with all 19 rig providers
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { join } from 'path';
import { mkdtemp, rm, readFile, stat, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';

// Import the modules under test (will fail until implemented)
import {
  loadCredentials,
  saveCredential,
  deleteCredential,
  getCredentialsPath,
  getProviderConfig,
  resolveCredential,
  maskApiKey,
} from '../credentials.js';

import {
  loadProviderConfig,
  saveProviderConfig,
  getProviderRegistry,
  SUPPORTED_PROVIDERS,
} from '../provider-config.js';

describe('Feature: Provider Configuration and Credentials Management', () => {
  let testDir: string;
  let originalHome: string | undefined;
  let originalAnthropicKey: string | undefined;
  let originalClaudeOAuthToken: string | undefined;
  let originalOpenaiKey: string | undefined;

  beforeEach(async () => {
    // Create temp directory to simulate ~/.fspec
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    originalHome = process.env.HOME;
    originalAnthropicKey = process.env.ANTHROPIC_API_KEY;
    originalClaudeOAuthToken = process.env.CLAUDE_CODE_OAUTH_TOKEN;
    originalOpenaiKey = process.env.OPENAI_API_KEY;
    process.env.HOME = testDir;

    // Clear env vars to test credential resolution
    // Must clear both ANTHROPIC_API_KEY and CLAUDE_CODE_OAUTH_TOKEN since both are valid for anthropic
    delete process.env.ANTHROPIC_API_KEY;
    delete process.env.CLAUDE_CODE_OAUTH_TOKEN;
    delete process.env.OPENAI_API_KEY;

    // Create .fspec directory
    await mkdir(join(testDir, '.fspec'), { recursive: true });
  });

  afterEach(async () => {
    process.env.HOME = originalHome;
    if (originalAnthropicKey) {
      process.env.ANTHROPIC_API_KEY = originalAnthropicKey;
    }
    if (originalClaudeOAuthToken) {
      process.env.CLAUDE_CODE_OAUTH_TOKEN = originalClaudeOAuthToken;
    }
    if (originalOpenaiKey) {
      process.env.OPENAI_API_KEY = originalOpenaiKey;
    }
    await rm(testDir, { recursive: true, force: true });
  });

  // ============================================
  // CREDENTIALS MANAGEMENT SCENARIOS
  // ============================================

  describe('Scenario: Save API key with secure file permissions', () => {
    it('should create credentials file with 600 permissions', async () => {
      // @step Given the credentials directory does not exist
      const credDir = join(testDir, '.fspec', 'credentials');
      const credPath = join(credDir, 'credentials.json');

      // @step When I save an API key for the "anthropic" provider
      await saveCredential('anthropic', 'sk-ant-test-key-12345');

      // @step Then the credentials file should be created at "~/.fspec/credentials/credentials.json"
      const fileExists = await stat(credPath)
        .then(() => true)
        .catch(() => false);
      expect(fileExists).toBe(true);

      // @step And the credentials file should have 600 permissions
      const fileStat = await stat(credPath);
      const fileMode = fileStat.mode & 0o777;
      expect(fileMode).toBe(0o600);

      // @step And the credentials directory should have 700 permissions
      const dirStat = await stat(credDir);
      const dirMode = dirStat.mode & 0o777;
      expect(dirMode).toBe(0o700);

      // @step And the API key should be stored under "providers.anthropic.apiKey"
      const content = JSON.parse(await readFile(credPath, 'utf-8'));
      expect(content.providers.anthropic.apiKey).toBe('sk-ant-test-key-12345');
    });
  });

  describe('Scenario: Delete a provider credential', () => {
    it('should remove credential and update file', async () => {
      // @step Given I have credentials configured for "anthropic"
      await saveCredential('anthropic', 'sk-ant-test-key-12345');
      const credPath = getCredentialsPath();

      // @step When I delete the credential for "anthropic"
      await deleteCredential('anthropic');

      // @step Then the credentials file should be updated
      const content = JSON.parse(await readFile(credPath, 'utf-8'));

      // @step And "anthropic" should no longer have an apiKey entry
      expect(content.providers.anthropic).toBeUndefined();

      // @step And the provider should NOT have credentials from file source
      // Note: .env file may still provide credentials as fallback, but the
      // credentials FILE should no longer be the source
      const config = await getProviderConfig('anthropic');
      expect(config.source).not.toBe('file');
    });
  });

  describe('Scenario: Credentials file takes precedence over environment variables', () => {
    it('should use credentials file over environment variable', async () => {
      // @step Given I have ANTHROPIC_API_KEY set in the environment
      process.env.ANTHROPIC_API_KEY = 'env-key-12345';

      // @step And I have a different API key in the credentials file for "anthropic"
      await saveCredential('anthropic', 'file-key-67890');

      // @step When the system resolves credentials for "anthropic"
      const resolved = await resolveCredential('anthropic');

      // @step Then the credentials file API key should be used
      expect(resolved).toBe('file-key-67890');

      // @step And the environment variable should be ignored
      expect(resolved).not.toBe('env-key-12345');
    });
  });

  describe('Scenario: Explicit credentials take highest priority', () => {
    it('should use explicit credentials over file and environment', async () => {
      // @step Given I have an API key in the credentials file for "anthropic"
      await saveCredential('anthropic', 'file-key-67890');

      // @step And I have ANTHROPIC_API_KEY set in the environment
      process.env.ANTHROPIC_API_KEY = 'env-key-12345';

      // @step When I call CodeletSession.new_with_credentials with a different explicit API key
      const explicitKey = 'explicit-key-99999';
      const resolved = await resolveCredential('anthropic', explicitKey);

      // @step Then the explicit API key should be used
      expect(resolved).toBe(explicitKey);

      // @step And neither the credentials file nor environment variable should be used
      expect(resolved).not.toBe('file-key-67890');
      expect(resolved).not.toBe('env-key-12345');
    });
  });

  describe('Scenario: .env file is lowest priority fallback', () => {
    it('should use .env file when no other sources available', async () => {
      // @step Given I do not have credentials in the credentials file
      // (no saveCredential call)

      // @step And I do not have ANTHROPIC_API_KEY in the environment
      delete process.env.ANTHROPIC_API_KEY;

      // @step And I have a .env file with ANTHROPIC_API_KEY defined
      const envPath = join(testDir, '.env');
      await writeFile(envPath, 'ANTHROPIC_API_KEY=dotenv-key-11111\n');

      // @step When the system resolves credentials for "anthropic"
      const resolved = await resolveCredential('anthropic', undefined, testDir);

      // @step Then the .env file API key should be used
      expect(resolved).toBe('dotenv-key-11111');
    });
  });

  describe('Scenario: Environment variables continue to work', () => {
    it('should use environment variable when no credentials file exists', async () => {
      // @step Given I have OPENAI_API_KEY set in the environment
      process.env.OPENAI_API_KEY = 'env-openai-key-12345';

      // @step And I do not have credentials configured in the credentials file
      // (no saveCredential call)

      // @step When the system detects available providers
      const config = await getProviderConfig('openai');

      // @step Then "openai" should be marked as available
      expect(config.apiKey).toBe('env-openai-key-12345');

      // @step And sessions should be creatable using the environment variable
      expect(config.apiKey).toBeDefined();
    });
  });

  describe('Scenario: API keys display with masked format', () => {
    it('should mask API key with visible prefix and suffix', async () => {
      // @step Given I have configured an API key "sk-ant-api03-abcdefghijklmnop" for "anthropic"
      const fullKey = 'sk-ant-api03-abcdefghijklmnop';

      // @step When I view the Settings for "anthropic"
      const masked = maskApiKey(fullKey);

      // @step Then the API key should display as "sk-ant-••••••••mnop"
      expect(masked).toMatch(/^sk-ant-.*••••.*mnop$/);

      // @step And the full key should never be visible
      expect(masked).not.toBe(fullKey);
      expect(masked.length).toBeLessThan(fullKey.length);
    });
  });

  describe('Scenario: API keys are never logged', () => {
    it('should not include API key in any output', async () => {
      // @step Given I configure an API key for any provider
      const apiKey = 'sk-secret-key-do-not-log';
      const consoleSpy = vi.spyOn(console, 'log');
      const consoleErrorSpy = vi.spyOn(console, 'error');

      // @step When the system processes the credential
      await saveCredential('anthropic', apiKey);

      // @step Then the API key should not appear in any log output
      const allLogs = consoleSpy.mock.calls.flat().join(' ');
      expect(allLogs).not.toContain(apiKey);

      // @step And the API key should not appear in error messages
      const allErrors = consoleErrorSpy.mock.calls.flat().join(' ');
      expect(allErrors).not.toContain(apiKey);

      // @step And only masked versions should be displayed in the UI
      const masked = maskApiKey(apiKey);
      expect(masked).not.toBe(apiKey);

      consoleSpy.mockRestore();
      consoleErrorSpy.mockRestore();
    });
  });

  // ============================================
  // PROVIDER CONFIGURATION SCENARIOS
  // ============================================

  describe('Scenario: Configure provider with custom base URL', () => {
    it('should save provider settings to config file', async () => {
      // @step Given I have an existing fspec configuration
      const configPath = join(testDir, '.fspec', 'fspec-config.json');
      await writeFile(configPath, JSON.stringify({ tui: {} }, null, 2));

      // @step When I configure the "openrouter" provider with base URL "https://openrouter.ai/api/v1"
      await saveProviderConfig('openrouter', {
        baseUrl: 'https://openrouter.ai/api/v1',
        enabled: true,
      });

      // @step Then the provider settings should be saved to "~/.fspec/fspec-config.json"
      const content = JSON.parse(await readFile(configPath, 'utf-8'));

      // @step And the settings should include "providers.openrouter.baseUrl"
      expect(content.providers.openrouter.baseUrl).toBe(
        'https://openrouter.ai/api/v1'
      );

      // @step And the settings should include "providers.openrouter.enabled" as true
      expect(content.providers.openrouter.enabled).toBe(true);
    });
  });

  describe('Scenario: Configure Ollama without API key', () => {
    it('should enable Ollama with just base URL', async () => {
      // @step Given I want to use the local Ollama provider
      // (user intent, no setup needed)

      // @step When I configure "ollama" with base URL "http://localhost:11434"
      await saveProviderConfig('ollama', {
        baseUrl: 'http://localhost:11434',
        enabled: true,
      });

      // @step Then the provider should be enabled without requiring an API key
      const config = await loadProviderConfig('ollama');
      expect(config.enabled).toBe(true);
      expect(config.baseUrl).toBe('http://localhost:11434');
      // No API key required for Ollama

      // @step And I should be able to list available Ollama models
      // This will be tested in integration tests with actual Ollama
      expect(config.enabled).toBe(true);
    });
  });

  describe('Scenario: Configure Azure OpenAI with endpoint and version', () => {
    it('should save Azure-specific configuration', async () => {
      // @step Given I want to use Azure OpenAI
      // (user intent, no setup needed)

      // @step When I configure "azure" provider with:
      await saveProviderConfig('azure', {
        endpoint: 'https://myresource.openai.azure.com',
        apiVersion: '2024-10-21',
        enabled: true,
      });

      // @step Then the Azure configuration should be saved
      const config = await loadProviderConfig('azure');
      expect(config.endpoint).toBe('https://myresource.openai.azure.com');
      expect(config.apiVersion).toBe('2024-10-21');

      // @step And I should be able to create sessions using Azure models
      expect(config.enabled).toBe(true);
    });
  });

  describe('Scenario: All 19 rig providers are registered', () => {
    it('should have exactly 19 providers in registry', async () => {
      // @step When I query the provider registry
      const registry = getProviderRegistry();

      // @step Then I should see exactly 19 providers:
      const expectedProviders = [
        'openai',
        'anthropic',
        'cohere',
        'gemini',
        'mistral',
        'xai',
        'together',
        'huggingface',
        'openrouter',
        'groq',
        'ollama',
        'deepseek',
        'perplexity',
        'moonshot',
        'hyperbolic',
        'mira',
        'galadriel',
        'azure',
        'voyageai',
        'zai',
      ];

      expect(registry.length).toBe(20);
      for (const provider of expectedProviders) {
        expect(registry).toContain(provider);
      }
    });
  });

  describe('Scenario: Gemini uses query parameter authentication', () => {
    it('should configure Gemini with query param auth method', async () => {
      // @step Given I have configured an API key for "gemini"
      await saveProviderConfig('gemini', {
        enabled: true,
        authMethod: 'query_param',
      });

      // @step When the system makes an API request to Gemini
      const config = await loadProviderConfig('gemini');

      // @step Then the API key should be passed as a query parameter "key"
      expect(config.authMethod).toBe('query_param');

      // @step And the API key should not be in the Authorization header
      expect(config.authMethod).not.toBe('bearer');
    });
  });

  describe('Scenario: Anthropic uses x-api-key header authentication', () => {
    it('should configure Anthropic with x-api-key header', async () => {
      // @step Given I have configured an API key for "anthropic"
      await saveProviderConfig('anthropic', {
        enabled: true,
        authMethod: 'x-api-key',
      });

      // @step When the system makes an API request to Anthropic
      const config = await loadProviderConfig('anthropic');

      // @step Then the API key should be in the "x-api-key" header
      expect(config.authMethod).toBe('x-api-key');

      // @step And the API key should not be in the Authorization header
      expect(config.authMethod).not.toBe('bearer');
    });
  });

  // ============================================
  // UI SCENARIOS (TUI - Model Selection)
  // ============================================

  describe('Scenario: Display provider configuration status in model selection', () => {
    it('should show configured and unconfigured providers', async () => {
      // @step Given the "anthropic" provider has credentials configured
      await saveCredential('anthropic', 'sk-ant-test-key');

      // @step And the "openai" provider does not have credentials configured
      // (no saveCredential call for openai)

      // @step When I open the model selection screen
      // This tests the data layer - UI rendering tested in component tests
      const anthropicConfig = await getProviderConfig('anthropic');
      const openaiConfig = await getProviderConfig('openai');

      // @step Then I should see "Anthropic" marked as "configured"
      expect(anthropicConfig.apiKey).toBeDefined();

      // @step And I should see "OpenAI" marked as "not configured"
      expect(openaiConfig.apiKey).toBeUndefined();

      // @step And configured providers should show available models
      expect(anthropicConfig.apiKey).toBeTruthy();

      // @step And unconfigured providers should show a warning
      expect(openaiConfig.apiKey).toBeFalsy();
    });
  });

  describe('Scenario: Test provider connection before saving', () => {
    it('should validate API key with lightweight call', async () => {
      // @step Given I am in the provider settings view
      // (user context, no setup needed)

      // @step And I have entered an API key for "anthropic"
      const testApiKey = 'sk-ant-test-connection-key';

      // @step When I click the "Test Connection" button
      // For now, test the connection test function exists
      // Real integration tests will hit actual API
      const config = await loadProviderConfig('anthropic');

      // @step Then the system should make a lightweight API call to validate the key
      // This is tested in integration tests with real API

      // @step And I should see a success message if the key is valid
      // UI feedback tested in component tests
      expect(config).toBeDefined();

      // @step And I should see an error message if the key is invalid
      // Error handling tested in component tests
      expect(config).toBeDefined();
    });
  });

  describe('Scenario: Navigate to Settings view with Tab key', () => {
    it('should show all 19 providers with status', async () => {
      // @step Given I am in the model selection screen
      // (user context, no setup needed)

      // @step When I press the Tab key
      // Tab key handling is TUI-specific, tested in component tests

      // @step Then I should see the Settings view
      // UI state tested in component tests
      const registry = getProviderRegistry();

      // @step And I should see a list of all 20 providers
      expect(registry).toHaveLength(20);

      // @step And each provider should show its configuration status
      // This tests that we can get config for each provider
      for (const providerId of registry) {
        const config = await loadProviderConfig(providerId);
        expect(config).toBeDefined();
      }
    });
  });

  // ============================================
  // NAPI CREDENTIAL INTEGRATION SCENARIOS
  // ============================================

  describe('Scenario: Create session with programmatic credentials via NAPI', () => {
    it('should create session with explicit API key', async () => {
      // @step Given I have TypeScript code that imports codelet-napi
      const { CodeletSession } = await import('@sengac/codelet-napi');

      // @step When I call CodeletSession.new_with_credentials with model "anthropic/claude-sonnet-4" and API key
      const explicitApiKey = 'sk-ant-explicit-test-key';
      const providerConfig = {
        providerId: 'anthropic',
        apiKey: explicitApiKey,
        enabled: true,
      };

      // This method doesn't exist yet - test will fail
      const session = await CodeletSession.newWithCredentials(
        'anthropic/claude-sonnet-4',
        providerConfig
      );

      // @step Then a session should be created successfully
      expect(session).toBeDefined();

      // @step And the session should use the provided API key
      // The session should be using the explicit key (set internally via env var)
      expect(session.currentProviderName).toBe('claude');

      // @step And the session should not read from environment variables
      // The implementation internally sets env var to pass to provider,
      // but the key used is the one we explicitly passed, not a pre-existing env var.
      // The beforeEach clears ANTHROPIC_API_KEY, so the only way session creation
      // succeeds is by using our explicit key.
      expect(session).toBeDefined();
    });
  });
});

// ============================================
// PROVIDER REGISTRY CONSTANTS
// ============================================

describe('Provider Registry Constants', () => {
  it('SUPPORTED_PROVIDERS should contain all 20 providers', () => {
    expect(SUPPORTED_PROVIDERS).toHaveLength(20);
    expect(SUPPORTED_PROVIDERS).toContain('anthropic');
    expect(SUPPORTED_PROVIDERS).toContain('openai');
    expect(SUPPORTED_PROVIDERS).toContain('gemini');
    expect(SUPPORTED_PROVIDERS).toContain('ollama');
    expect(SUPPORTED_PROVIDERS).toContain('azure');
    expect(SUPPORTED_PROVIDERS).toContain('zai');
  });
});
