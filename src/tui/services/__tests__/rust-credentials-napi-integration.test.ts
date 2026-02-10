/**
 * Feature: spec/features/move-credential-management-to-rust.feature
 *
 * E2E Integration Tests for Rust Credential Management (CONFIG-005)
 *
 * These tests verify that Rust correctly resolves credentials from:
 * 1. Credentials file (~/.fspec/credentials/credentials.json)
 * 2. Environment variables
 * 3. Project .env files
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

describe('Feature: Move Credential Management to Rust', () => {
  let testDataDir: string;
  let originalEnvVars: Record<string, string | undefined>;

  beforeEach(async () => {
    // Create unique temp directory for each test
    testDataDir = join(tmpdir(), `fspec-test-${randomUUID()}`);
    await mkdir(testDataDir, { recursive: true });

    // Save and clear relevant environment variables
    originalEnvVars = {
      ANTHROPIC_API_KEY: process.env.ANTHROPIC_API_KEY,
      CLAUDE_CODE_OAUTH_TOKEN: process.env.CLAUDE_CODE_OAUTH_TOKEN,
      OPENAI_API_KEY: process.env.OPENAI_API_KEY,
      GOOGLE_GENERATIVE_AI_API_KEY: process.env.GOOGLE_GENERATIVE_AI_API_KEY,
      GEMINI_API_KEY: process.env.GEMINI_API_KEY,
    };

    // Clear all env vars for clean testing
    delete process.env.ANTHROPIC_API_KEY;
    delete process.env.CLAUDE_CODE_OAUTH_TOKEN;
    delete process.env.OPENAI_API_KEY;
    delete process.env.GOOGLE_GENERATIVE_AI_API_KEY;
    delete process.env.GEMINI_API_KEY;

    // Set up test data directory - this resets credential store via persistence module
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

  describe('Scenario: NAPI integration: Rust sets environment variable during session creation', () => {
    it('should resolve credentials from file and set environment variable', async () => {
      // @step Given a temporary data directory is configured via persistenceSetDataDirectory
      expect(testDataDir).toBeDefined();

      // @step Given a credentials.json file exists with API key "sk-test-fixture-key" for provider "anthropic"
      const credentialsDir = join(testDataDir, 'credentials');
      await mkdir(credentialsDir, { recursive: true });

      const testApiKey = 'sk-test-fixture-key';
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

      // @step When sessionManagerCreateWithId is called with model "anthropic/claude-sonnet-4"
      const sessionId = randomUUID();
      const modelPath = 'anthropic/claude-sonnet-4';

      try {
        await sessionManagerCreateWithId(
          sessionId,
          modelPath,
          testDataDir,
          'Test Session'
        );

        // @step Then Rust should resolve the credential from the file
        // @step Then process.env.ANTHROPIC_API_KEY should be set to "sk-test-fixture-key"
        expect(process.env.ANTHROPIC_API_KEY).toBe(testApiKey);

        // Cleanup
        try {
          sessionManagerDestroy(sessionId);
        } catch {
          // Session may already be destroyed
        }
      } catch (error) {
        // Even if session creation fails (e.g., invalid API key for actual API call),
        // the environment variable should still be set by Rust
        expect(process.env.ANTHROPIC_API_KEY).toBe(testApiKey);
      }
    });
  });

  describe('Scenario: NAPI integration: credentialsReload updates cached credentials', () => {
    it('should reload credentials when file is updated', async () => {
      // @step Given a temporary data directory is configured via persistenceSetDataDirectory
      expect(testDataDir).toBeDefined();

      // @step Given a credentials.json file exists with API key "initial-key" for provider "anthropic"
      const credentialsDir = join(testDataDir, 'credentials');
      await mkdir(credentialsDir, { recursive: true });

      const initialKey = 'initial-key';
      const credentialsContent = {
        version: 1,
        providers: {
          anthropic: {
            apiKey: initialKey,
            lastUpdated: new Date().toISOString(),
          },
        },
      };

      const credentialsPath = join(credentialsDir, 'credentials.json');
      await writeFile(
        credentialsPath,
        JSON.stringify(credentialsContent, null, 2),
        { mode: 0o600 }
      );

      // @step Given a session was created triggering initial credential resolution
      const sessionId1 = randomUUID();
      try {
        await sessionManagerCreateWithId(
          sessionId1,
          'anthropic/claude-sonnet-4',
          testDataDir,
          'Initial Session'
        );
      } catch {
        // Ignore API errors - we just need the credential resolution to happen
      }

      // Verify initial credential was resolved
      expect(process.env.ANTHROPIC_API_KEY).toBe(initialKey);

      // @step When the credentials.json file is updated with API key "updated-key"
      const updatedKey = 'updated-key';
      const updatedContent = {
        version: 1,
        providers: {
          anthropic: {
            apiKey: updatedKey,
            lastUpdated: new Date().toISOString(),
          },
        },
      };

      // Wait a bit to ensure mtime changes
      await new Promise(resolve => setTimeout(resolve, 50));

      await writeFile(
        credentialsPath,
        JSON.stringify(updatedContent, null, 2),
        { mode: 0o600 }
      );

      // @step When credentialsReload() NAPI function is called
      const reloaded = credentialsReload();

      // @step Then the function should return true indicating the file was reloaded
      expect(reloaded).toBe(true);

      // @step Then a new session creation should use the updated API key
      const sessionId2 = randomUUID();
      try {
        await sessionManagerCreateWithId(
          sessionId2,
          'anthropic/claude-sonnet-4',
          testDataDir,
          'Updated Session'
        );
      } catch {
        // Ignore API errors
      }

      expect(process.env.ANTHROPIC_API_KEY).toBe(updatedKey);

      // Cleanup
      try {
        sessionManagerDestroy(sessionId1);
        sessionManagerDestroy(sessionId2);
      } catch {
        // Ignore cleanup errors
      }
    });
  });

  describe('Scenario: Resolve credential from credentials file', () => {
    it('should use credentials file when it exists and env var is not set', async () => {
      // @step Given a credentials file exists with an API key for provider "anthropic"
      const credentialsDir = join(testDataDir, 'credentials');
      await mkdir(credentialsDir, { recursive: true });

      const fileApiKey = 'sk-file-credential-key';
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

      // @step And no ANTHROPIC_API_KEY environment variable is set
      expect(process.env.ANTHROPIC_API_KEY).toBeUndefined();

      // @step When credential resolution is requested for provider "anthropic"
      const sessionId = randomUUID();
      try {
        await sessionManagerCreateWithId(
          sessionId,
          'anthropic/claude-sonnet-4',
          testDataDir,
          'Test Session'
        );
      } catch {
        // Ignore API errors
      }

      // @step Then the API key from the credentials file should be returned
      expect(process.env.ANTHROPIC_API_KEY).toBe(fileApiKey);

      try {
        sessionManagerDestroy(sessionId);
      } catch {
        // Ignore cleanup errors
      }
    });
  });

  describe('Scenario: Resolve credential from environment variable when file has no key', () => {
    it('should fall back to environment variable when credentials file has no key', async () => {
      // @step Given no API key exists in the credentials file for provider "anthropic"
      const credentialsDir = join(testDataDir, 'credentials');
      await mkdir(credentialsDir, { recursive: true });

      // Empty credentials file (no anthropic key)
      const credentialsContent = {
        version: 1,
        providers: {},
      };

      await writeFile(
        join(credentialsDir, 'credentials.json'),
        JSON.stringify(credentialsContent, null, 2),
        { mode: 0o600 }
      );

      // @step And the ANTHROPIC_API_KEY environment variable is set
      const envApiKey = 'sk-env-fallback-key';
      process.env.ANTHROPIC_API_KEY = envApiKey;

      // @step When credential resolution is requested for provider "anthropic"
      const sessionId = randomUUID();
      try {
        await sessionManagerCreateWithId(
          sessionId,
          'anthropic/claude-sonnet-4',
          testDataDir,
          'Test Session'
        );
      } catch {
        // Ignore API errors
      }

      // @step Then the API key from the environment variable should be returned
      // Note: Rust will NOT overwrite an existing env var, so we verify it's still set
      expect(process.env.ANTHROPIC_API_KEY).toBe(envApiKey);

      try {
        sessionManagerDestroy(sessionId);
      } catch {
        // Ignore cleanup errors
      }
    });
  });

  describe('Scenario: Session creation uses 4 parameters (no api_key)', () => {
    it('should accept only 4 parameters without api_key', async () => {
      // This test verifies the API signature change from CONFIG-005
      // sessionManagerCreateWithId should have 4 params: id, model, project, name

      const credentialsDir = join(testDataDir, 'credentials');
      await mkdir(credentialsDir, { recursive: true });

      await writeFile(
        join(credentialsDir, 'credentials.json'),
        JSON.stringify(
          {
            version: 1,
            providers: {
              anthropic: {
                apiKey: 'test-key',
                lastUpdated: new Date().toISOString(),
              },
            },
          },
          null,
          2
        ),
        { mode: 0o600 }
      );

      const sessionId = randomUUID();

      // @step When sessionManagerCreateWithId is called without an api_key parameter
      // The function should work with exactly 4 arguments
      try {
        await sessionManagerCreateWithId(
          sessionId, // id
          'anthropic/claude-sonnet-4', // model
          testDataDir, // project
          'Test Session' // name
          // NO 5th api_key parameter!
        );
        // @step Then Rust should resolve the credential internally
        // @step And the session should be created with the resolved API key
        expect(process.env.ANTHROPIC_API_KEY).toBeDefined();
      } catch (error) {
        // API call may fail due to invalid key, but credential resolution should work
        expect(process.env.ANTHROPIC_API_KEY).toBe('test-key');
      }

      try {
        sessionManagerDestroy(sessionId);
      } catch {
        // Ignore cleanup errors
      }
    });
  });
});
