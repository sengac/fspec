/**
 * Feature: spec/features/oauth-tui-broken-flows.feature
 *
 * PROV-028: E2E NAPI integration tests for Claude OAuth flows.
 * These tests call REAL Rust code via NAPI bindings — no mocks.
 *
 * Tests the TypeScript → Rust → TypeScript round-trip for:
 * - claudeOauthHeadlessStart: sync, generates PKCE + authorize URL
 * - claudeOauthGetTokens / claudeOauthClearTokens: async, file I/O
 *
 * These verify the NAPI boundary works correctly and the Rust
 * implementation returns properly-shaped data to TypeScript.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { tmpdir } from 'os';
import { mkdtemp, rm, writeFile } from 'fs/promises';
import { join } from 'path';
import { URL } from 'url';
import {
  claudeOauthHeadlessStart,
  claudeOauthGetTokens,
  claudeOauthClearTokens,
} from '@sengac/codelet-napi';

describe('E2E NAPI: Claude OAuth flows (TypeScript → Rust → TypeScript)', () => {
  let tempDir: string;
  let originalFspecHome: string | undefined;

  beforeEach(async () => {
    // Create isolated temp dir for FSPEC_HOME
    tempDir = await mkdtemp(join(tmpdir(), 'fspec-claude-oauth-e2e-'));
    originalFspecHome = process.env['FSPEC_HOME'];
    process.env['FSPEC_HOME'] = tempDir;
  });

  afterEach(async () => {
    // Restore original FSPEC_HOME
    if (originalFspecHome !== undefined) {
      process.env['FSPEC_HOME'] = originalFspecHome;
    } else {
      delete process.env['FSPEC_HOME'];
    }
    // Clean up temp dir
    await rm(tempDir, { recursive: true, force: true });
  });

  // =========================================================================
  // claudeOauthHeadlessStart — sync NAPI call, no network
  // =========================================================================

  describe('claudeOauthHeadlessStart (sync, no network)', () => {
    it('should return an authorize URL containing claude.ai and PKCE parameters', () => {
      // @step Given I call claudeOauthHeadlessStart from TypeScript
      const result = claudeOauthHeadlessStart();

      // @step Then the result should have an authorizeUrl string
      expect(typeof result.authorizeUrl).toBe('string');
      expect(result.authorizeUrl.length).toBeGreaterThan(0);

      // @step And the authorize URL should point to claude.ai OAuth
      expect(result.authorizeUrl).toContain('claude.ai/oauth/authorize');

      // @step And the authorize URL should include PKCE code_challenge parameter
      expect(result.authorizeUrl).toContain('code_challenge=');
      expect(result.authorizeUrl).toContain('code_challenge_method=S256');
    });

    it('should return a pkceVerifier string of sufficient length', () => {
      const result = claudeOauthHeadlessStart();

      // @step Then the pkceVerifier should be at least 43 characters (RFC 7636)
      expect(typeof result.pkceVerifier).toBe('string');
      expect(result.pkceVerifier.length).toBeGreaterThanOrEqual(43);
    });

    it('should generate unique PKCE values on each call', () => {
      // @step When I call claudeOauthHeadlessStart twice
      const result1 = claudeOauthHeadlessStart();
      const result2 = claudeOauthHeadlessStart();

      // @step Then each call should return a different pkceVerifier
      expect(result1.pkceVerifier).not.toBe(result2.pkceVerifier);

      // @step And each call should return a different authorize URL (different state)
      expect(result1.authorizeUrl).not.toBe(result2.authorizeUrl);
    });

    it('should include required OAuth parameters in authorize URL', () => {
      const result = claudeOauthHeadlessStart();
      const url = new URL(result.authorizeUrl);

      // @step Then the URL should have response_type=code
      expect(url.searchParams.get('response_type')).toBe('code');

      // @step And the URL should have a client_id
      const clientId = url.searchParams.get('client_id');
      expect(clientId).toBeTruthy();
      expect(clientId!.length).toBeGreaterThan(0);

      // @step And the URL should have a redirect_uri
      expect(url.searchParams.get('redirect_uri')).toBeTruthy();

      // @step And the URL should have a state parameter (CSRF)
      expect(url.searchParams.get('state')).toBeTruthy();
    });
  });

  // =========================================================================
  // claudeOauthGetTokens — async NAPI call, reads file system
  // =========================================================================

  describe('claudeOauthGetTokens (async, file I/O)', () => {
    it('should return null when no claude_auth.json exists', async () => {
      // @step Given FSPEC_HOME points to an empty temp directory
      // @step When I call claudeOauthGetTokens
      const tokens = await claudeOauthGetTokens();

      // @step Then the result should be null (no tokens)
      expect(tokens).toBeNull();
    });

    it('should return tokens when claude_auth.json exists with valid data', async () => {
      // @step Given a claude_auth.json exists with token data
      const authData = {
        access_token: 'e2e-test-access-token',
        refresh_token: 'e2e-test-refresh-token',
        expires: Date.now() + 3600000,
      };
      await writeFile(
        join(tempDir, 'claude_auth.json'),
        JSON.stringify(authData)
      );

      // @step When I call claudeOauthGetTokens
      const tokens = await claudeOauthGetTokens();

      // @step Then the result should contain the stored tokens
      expect(tokens).not.toBeNull();
      expect(tokens!.accessToken).toBe('e2e-test-access-token');
      expect(tokens!.refreshToken).toBe('e2e-test-refresh-token');
      expect(tokens!.expires).toBeGreaterThan(0);
    });
  });

  // =========================================================================
  // claudeOauthClearTokens — async NAPI call, deletes file
  // =========================================================================

  describe('claudeOauthClearTokens (async, file I/O)', () => {
    it('should succeed idempotently when no claude_auth.json exists', async () => {
      // @step Given no claude_auth.json exists
      // @step When I call claudeOauthClearTokens
      // @step Then it should succeed without error (idempotent)
      await expect(claudeOauthClearTokens()).resolves.not.toThrow();
    });

    it('should delete claude_auth.json when it exists', async () => {
      // @step Given a claude_auth.json exists
      const authData = {
        access_token: 'to-be-cleared',
        refresh_token: 'to-be-cleared',
        expires: Date.now() + 3600000,
      };
      await writeFile(
        join(tempDir, 'claude_auth.json'),
        JSON.stringify(authData)
      );

      // @step When I call claudeOauthClearTokens
      await claudeOauthClearTokens();

      // @step Then claudeOauthGetTokens should return null
      const tokens = await claudeOauthGetTokens();
      expect(tokens).toBeNull();
    });
  });

  // =========================================================================
  // Round-trip: headless start → verify URL → (simulated complete) → get/clear
  // =========================================================================

  describe('Round-trip flow verification', () => {
    it('should demonstrate the full headless flow data shape', () => {
      // @step Given I start a headless login flow
      const startResult = claudeOauthHeadlessStart();

      // @step Then the authorize URL should be a valid URL
      expect(() => new URL(startResult.authorizeUrl)).not.toThrow();

      // @step And the pkce verifier should be usable as a state parameter
      const url = new URL(startResult.authorizeUrl);
      const state = url.searchParams.get('state');
      // The state param should match the pkce verifier
      // (Anthropic uses verifier as state — unlike Codex which uses separate state)
      expect(state).toBe(startResult.pkceVerifier);
    });
  });
});
