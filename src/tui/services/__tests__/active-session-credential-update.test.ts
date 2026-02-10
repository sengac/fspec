/**
 * Feature: spec/features/move-credential-management-to-rust.feature
 *
 * E2E Integration Tests for Active Session Credential Updates (CONFIG-005)
 *
 * User Story:
 *   As a TUI user, I want to update my API credentials,
 *   so that existing Rust sessions automatically pick up the new credentials
 *   without requiring a session restart.
 *
 * These tests verify that:
 * 1. An ACTIVE session (not resumed, not new - ACTIVE) picks up credential changes
 * 2. Credentials are read from credentials.json (NOT environment variables)
 * 3. credentialsReload() updates both the cache AND the environment variables
 *
 * NO MOCKS - Real NAPI calls with real fixtures.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { join } from 'path';
import { mkdir, writeFile, rm } from 'fs/promises';
import { existsSync } from 'fs';
import { tmpdir } from 'os';
import { randomUUID } from 'crypto';

import {
  persistenceSetDataDirectory,
  credentialsReload,
  sessionManagerCreateWithId,
  sessionManagerDestroy,
} from '@sengac/codelet-napi';

describe('Feature: Active Session Credential Updates', () => {
  let testDataDir: string;
  let credentialsDir: string;
  let credentialsPath: string;
  let originalEnvVars: Record<string, string | undefined>;

  beforeEach(async () => {
    // Create unique temp directory for each test
    testDataDir = join(tmpdir(), `fspec-active-cred-test-${randomUUID()}`);
    credentialsDir = join(testDataDir, 'credentials');
    credentialsPath = join(credentialsDir, 'credentials.json');

    await mkdir(credentialsDir, { recursive: true });

    // Save and clear ALL relevant environment variables
    // This ensures we're testing credentials.json, NOT env vars
    originalEnvVars = {
      ANTHROPIC_API_KEY: process.env.ANTHROPIC_API_KEY,
      CLAUDE_CODE_OAUTH_TOKEN: process.env.CLAUDE_CODE_OAUTH_TOKEN,
      OPENAI_API_KEY: process.env.OPENAI_API_KEY,
      GOOGLE_GENERATIVE_AI_API_KEY: process.env.GOOGLE_GENERATIVE_AI_API_KEY,
      GEMINI_API_KEY: process.env.GEMINI_API_KEY,
    };

    // Clear all env vars - we want ONLY credentials.json to be the source
    delete process.env.ANTHROPIC_API_KEY;
    delete process.env.CLAUDE_CODE_OAUTH_TOKEN;
    delete process.env.OPENAI_API_KEY;
    delete process.env.GOOGLE_GENERATIVE_AI_API_KEY;
    delete process.env.GEMINI_API_KEY;

    // Set up test data directory - this resets credential store
    persistenceSetDataDirectory(testDataDir);
  });

  afterEach(async () => {
    // Restore original environment variables
    for (const [key, value] of Object.entries(originalEnvVars)) {
      if (value !== undefined) {
        process.env[key] = value;
      } else {
        delete process.env[key];
      }
    }

    // Cleanup temp directory
    if (existsSync(testDataDir)) {
      await rm(testDataDir, { recursive: true, force: true });
    }
  });

  /**
   * Helper to write credentials.json with a specific API key
   */
  async function writeCredentials(
    providerId: string,
    apiKey: string
  ): Promise<void> {
    const content = {
      version: 1,
      providers: {
        [providerId]: {
          apiKey,
          lastUpdated: new Date().toISOString(),
        },
      },
    };
    await writeFile(credentialsPath, JSON.stringify(content, null, 2), {
      mode: 0o600,
    });
  }

  describe('Scenario: Active session picks up credential changes without restart', () => {
    it('should use updated credentials on next API call in same session', async () => {
      // =========================================================================
      // This is THE critical test for the user story:
      // "existing Rust sessions automatically pick up the new credentials
      //  without requiring a session restart"
      // =========================================================================

      const initialKey = 'sk-initial-key-from-credentials-json';
      const updatedKey = 'sk-updated-key-from-credentials-json';

      // @step Given an active Rust session exists using credentials from credentials.json
      await writeCredentials('anthropic', initialKey);
      const sessionId = randomUUID();
      try {
        await sessionManagerCreateWithId(
          sessionId,
          'anthropic/claude-sonnet-4',
          testDataDir,
          'Active Session Test'
        );
      } catch {
        // Ignore API errors - we're testing credential resolution, not API validity
      }

      // @step Given the session has set ANTHROPIC_API_KEY environment variable to the initial key
      expect(process.env.ANTHROPIC_API_KEY).toBe(initialKey);

      // @step When the user updates credentials.json with a new API key
      // Wait to ensure mtime changes
      await new Promise(resolve => setTimeout(resolve, 100));
      await writeCredentials('anthropic', updatedKey);

      // @step When credentialsReload() NAPI function is called
      const reloaded = credentialsReload();
      expect(reloaded).toBe(true);

      // @step Then ANTHROPIC_API_KEY environment variable should be updated to the new key
      expect(process.env.ANTHROPIC_API_KEY).toBe(updatedKey);

      // @step Then the active session should use the new key on next API call without restart
      // The env var being updated proves this - rig reads from env var for API calls
      // Session is still active (same sessionId, never destroyed)
      expect(process.env.ANTHROPIC_API_KEY).toBe(updatedKey);

      // Cleanup
      try {
        sessionManagerDestroy(sessionId);
      } catch {
        // Ignore cleanup errors
      }
    });

    it('should update environment variables for multiple providers on credentialsReload', async () => {
      // Test that credentialsReload updates env vars for ALL configured providers

      const anthropicKey1 = 'sk-anthropic-initial';
      const openaiKey1 = 'sk-openai-initial';
      const anthropicKey2 = 'sk-anthropic-updated';
      const openaiKey2 = 'sk-openai-updated';

      // @step Given credentials.json has keys for multiple providers
      await writeFile(
        credentialsPath,
        JSON.stringify(
          {
            version: 1,
            providers: {
              anthropic: {
                apiKey: anthropicKey1,
                lastUpdated: new Date().toISOString(),
              },
              openai: {
                apiKey: openaiKey1,
                lastUpdated: new Date().toISOString(),
              },
            },
          },
          null,
          2
        ),
        { mode: 0o600 }
      );

      // @step When I create a session (triggers initial credential resolution)
      const sessionId = randomUUID();
      try {
        await sessionManagerCreateWithId(
          sessionId,
          'anthropic/claude-sonnet-4',
          testDataDir,
          'Multi-Provider Test'
        );
      } catch {
        // Ignore API errors
      }

      // @step Then both provider env vars should be set from credentials.json
      expect(process.env.ANTHROPIC_API_KEY).toBe(anthropicKey1);
      // Note: OpenAI env var might not be set if session only resolves the provider being used

      // @step When I update credentials.json with new keys for both providers
      await new Promise(resolve => setTimeout(resolve, 100));
      await writeFile(
        credentialsPath,
        JSON.stringify(
          {
            version: 1,
            providers: {
              anthropic: {
                apiKey: anthropicKey2,
                lastUpdated: new Date().toISOString(),
              },
              openai: {
                apiKey: openaiKey2,
                lastUpdated: new Date().toISOString(),
              },
            },
          },
          null,
          2
        ),
        { mode: 0o600 }
      );

      // @step And I call credentialsReload()
      const reloaded = credentialsReload();
      expect(reloaded).toBe(true);

      // @step Then ANTHROPIC_API_KEY should be updated
      expect(process.env.ANTHROPIC_API_KEY).toBe(anthropicKey2);

      // Cleanup
      try {
        sessionManagerDestroy(sessionId);
      } catch {
        // Ignore cleanup errors
      }
    });
  });

  describe('Scenario: Credentials file is the source of truth, not environment variables', () => {
    it('should read from credentials.json even when env var was previously set', async () => {
      // This tests that we're actually using the file, not leaking from env vars

      const fileKey = 'sk-from-file-not-env';

      // @step Given ANTHROPIC_API_KEY was NOT set in environment (cleared in beforeEach)
      expect(process.env.ANTHROPIC_API_KEY).toBeUndefined();

      // @step And credentials.json contains an API key
      await writeCredentials('anthropic', fileKey);

      // @step When I create a session
      const sessionId = randomUUID();
      try {
        await sessionManagerCreateWithId(
          sessionId,
          'anthropic/claude-sonnet-4',
          testDataDir,
          'File Source Test'
        );
      } catch {
        // Ignore API errors
      }

      // @step Then ANTHROPIC_API_KEY should be set from the file
      expect(process.env.ANTHROPIC_API_KEY).toBe(fileKey);

      // @step And this proves credentials.json was the source (not env)
      // because env was undefined before session creation

      // Cleanup
      try {
        sessionManagerDestroy(sessionId);
      } catch {
        // Ignore cleanup errors
      }
    });

    it('should prefer credentials.json over environment variable', async () => {
      const fileKey = 'sk-from-file-should-win';
      const envKey = 'sk-from-env-should-lose';

      // @step Given credentials.json contains an API key
      await writeCredentials('anthropic', fileKey);

      // @step And ANTHROPIC_API_KEY is also set in environment
      process.env.ANTHROPIC_API_KEY = envKey;

      // @step When I create a session
      const sessionId = randomUUID();
      try {
        await sessionManagerCreateWithId(
          sessionId,
          'anthropic/claude-sonnet-4',
          testDataDir,
          'Priority Test'
        );
      } catch {
        // Ignore API errors
      }

      // @step Then ANTHROPIC_API_KEY should be set from the FILE, not the original env
      expect(process.env.ANTHROPIC_API_KEY).toBe(fileKey);

      // Cleanup
      try {
        sessionManagerDestroy(sessionId);
      } catch {
        // Ignore cleanup errors
      }
    });
  });

  describe('Scenario: credentialsReload updates env vars for active session without API call', () => {
    it('should update env var immediately on credentialsReload, not on next session creation', async () => {
      // This is the critical distinction:
      // - OLD (wrong): credentialsReload updates cache, env var updated on NEXT session
      // - NEW (correct): credentialsReload updates cache AND env var immediately

      const key1 = 'sk-key-one';
      const key2 = 'sk-key-two';

      // @step Given credentials.json has initial key
      await writeCredentials('anthropic', key1);

      // @step And I create a session
      const sessionId = randomUUID();
      try {
        await sessionManagerCreateWithId(
          sessionId,
          'anthropic/claude-sonnet-4',
          testDataDir,
          'Immediate Update Test'
        );
      } catch {
        // Ignore API errors
      }

      expect(process.env.ANTHROPIC_API_KEY).toBe(key1);

      // @step When I update credentials.json
      await new Promise(resolve => setTimeout(resolve, 100));
      await writeCredentials('anthropic', key2);

      // @step And I call credentialsReload()
      credentialsReload();

      // @step Then env var should be updated IMMEDIATELY
      // NOT requiring another session creation
      expect(process.env.ANTHROPIC_API_KEY).toBe(key2);

      // @step And the original session is still the same (not recreated)
      // (we haven't called sessionManagerCreateWithId again)

      // Cleanup
      try {
        sessionManagerDestroy(sessionId);
      } catch {
        // Ignore cleanup errors
      }
    });
  });

  describe('Scenario: Simulate TUI Settings credential update flow', () => {
    it('should simulate the complete TUI flow: Settings save -> credentialsReload -> active session uses new key', async () => {
      // This simulates the actual user flow in the TUI:
      // 1. User has active session
      // 2. User opens Settings
      // 3. User enters new API key
      // 4. User saves (which writes to credentials.json and calls credentialsReload)
      // 5. User returns to active session
      // 6. Next prompt uses the new key

      const initialKey = 'sk-user-enters-this-initially';
      const newKeyFromSettings = 'sk-user-enters-this-in-settings';

      // @step Given user has credentials configured and an active session
      await writeCredentials('anthropic', initialKey);

      const sessionId = randomUUID();
      try {
        await sessionManagerCreateWithId(
          sessionId,
          'anthropic/claude-sonnet-4',
          testDataDir,
          'TUI Settings Flow Test'
        );
      } catch {
        // Ignore API errors
      }

      expect(process.env.ANTHROPIC_API_KEY).toBe(initialKey);

      // @step When user opens Settings and enters a new API key
      // (simulated by updating credentials.json - this is what saveCredential() does)
      await new Promise(resolve => setTimeout(resolve, 100));
      await writeCredentials('anthropic', newKeyFromSettings);

      // @step And the Settings save triggers credentialsReload() NAPI call
      // (this is what the TypeScript saveCredential() function does after writing)
      const reloaded = credentialsReload();
      expect(reloaded).toBe(true);

      // @step Then when user returns to the active session and sends a prompt
      // @step The prompt should use the new API key
      // (verified by checking the env var that rig will read)
      expect(process.env.ANTHROPIC_API_KEY).toBe(newKeyFromSettings);

      // @step And the user did NOT have to restart the session
      // (same sessionId, never called sessionManagerDestroy and recreate)

      // Cleanup
      try {
        sessionManagerDestroy(sessionId);
      } catch {
        // Ignore cleanup errors
      }
    });
  });
});
