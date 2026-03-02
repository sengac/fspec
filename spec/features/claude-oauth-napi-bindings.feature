@PROV-024
Feature: NAPI bindings for Anthropic OAuth subscription flows

  """
  New file: codelet/napi/src/claude_oauth.rs — All NAPI functions in one module. Imports claude_browser_oauth_login from claude_oauth_server, generate_pkce/build_authorize_url/parse_authorization_code/exchange_authorization_code/calculate_expiry from claude_oauth, refresh_access_token_at from claude_oauth, read_claude_auth/write_claude_auth/get_claude_auth_path from claude_auth. Mirrors codex_oauth.rs structure.
  NapiClaudeTokens #[napi(object)] struct maps to ClaudeAuthJson with access_token, refresh_token, expires (f64 for JS compatibility). NapiClaudeHeadlessStartResult #[napi(object)] with authorize_url (String) and pkce_verifier (String) — verifier returned to TS so it can be passed back to complete().
  lib.rs registration: add `mod claude_oauth;` under #[cfg(not(feature = "noop"))] and `pub use claude_oauth::*;` — same pattern as codex_oauth module.
  Headless two-phase approach: claude_oauth_headless_start() generates PKCE via generate_pkce(), builds authorize URL via build_authorize_url(), returns NapiClaudeHeadlessStartResult. claude_oauth_headless_complete(code_with_state, pkce_verifier) parses code#state, validates state == pkce_verifier, exchanges via exchange_authorization_code(), persists via write_claude_auth(), returns NapiClaudeTokens. No CodeEntryFn callback needed — NAPI boundary stays clean.
  Key difference from Codex NAPI (PROV-015): Claude auth is simpler — no id_token, no account_id, no JWT extraction. But claude_auth is async (tokio::fs) so get_tokens and clear_tokens are async NAPI functions (vs sync in Codex). Also no device polling — headless is start+complete instead of start+poll.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. claude_oauth_browser_login() is an async NAPI function that spawns a tokio task to run claude_browser_oauth_login() from claude_oauth_server.rs, returning a Promise<NapiClaudeTokens> to TypeScript
  #   2. Headless login uses a two-phase design: claude_oauth_headless_start() returns NapiClaudeHeadlessStartResult with authorize_url and pkce_verifier (so TUI can display URL and collect code), then claude_oauth_headless_complete(code_with_state, pkce_verifier) validates state, exchanges code, and returns NapiClaudeTokens
  #   3. claude_oauth_refresh_token(refresh_token: string) is an async NAPI function that calls refresh_access_token_at() from claude_oauth.rs and returns NapiClaudeTokens with refreshed tokens persisted to claude_auth.json
  #   4. claude_oauth_get_tokens() is an async NAPI function that reads claude_auth.json via read_claude_auth() and returns NapiClaudeTokens or null — async because claude_auth uses tokio::fs unlike Codex which is sync
  #   5. claude_oauth_clear_tokens() is an async NAPI function that deletes claude_auth.json (or clears its content) for disconnect — mirrors codex_oauth_clear_tokens() pattern
  #   6. All NAPI functions convert Rust errors to napi::Error via Error::from_reason() — TypeScript sees rejected promises with descriptive error messages
  #   7. NapiClaudeTokens is an #[napi(object)] struct with fields: access_token (String), refresh_token (String), expires (f64, milliseconds since epoch) — maps to ClaudeAuthJson from claude_auth.rs
  #   8. The NAPI module file is codelet/napi/src/claude_oauth.rs, registered in lib.rs under #[cfg(not(feature = "noop"))] — same pattern as codex_oauth module
  #
  # EXAMPLES:
  #   1. TUI calls claude_oauth_browser_login(): tokio spawns claude_browser_oauth_login(), local server starts, browser opens, user authorizes on claude.ai, pastes code#state, tokens exchanged, Promise resolves with NapiClaudeTokens containing access_token, refresh_token, expires
  #   2. TUI calls claude_oauth_headless_start(): generates PKCE, builds authorize URL, returns NapiClaudeHeadlessStartResult with authorize_url and pkce_verifier. TUI displays URL. User visits URL, authorizes, copies code#state.
  #   3. TUI calls claude_oauth_headless_complete('authcode123#verifier456', 'verifier456'): validates state matches pkce_verifier, exchanges code via JSON POST, persists tokens to claude_auth.json, returns NapiClaudeTokens
  #   4. Headless complete with wrong state: claude_oauth_headless_complete('code#wrong_state', 'real_verifier') → Promise rejects with error containing 'CSRF validation failed — state mismatch'
  #   5. Headless complete with no hash separator: claude_oauth_headless_complete('codeonly', 'verifier') → Promise rejects with error containing 'Missing state'
  #   6. TUI calls claude_oauth_refresh_token('rt_abc123'): Rust calls refresh_access_token_at(), returns NapiClaudeTokens with refreshed access_token. Persists updated tokens to claude_auth.json.
  #   7. Token refresh fails: claude_oauth_refresh_token('invalid_rt') → Promise rejects with error describing the failure status
  #   8. TUI calls claude_oauth_get_tokens() with valid claude_auth.json: returns NapiClaudeTokens with access_token, refresh_token, expires populated
  #   9. TUI calls claude_oauth_get_tokens() with no claude_auth.json: returns null (not an error — absence of tokens is a valid state)
  #   10. Browser login times out: claude_oauth_browser_login() Promise rejects with error containing 'timed out'
  #   11. TUI calls claude_oauth_clear_tokens() with existing claude_auth.json: file deleted, subsequent get_tokens returns null
  #   12. TUI calls claude_oauth_clear_tokens() with no claude_auth.json: returns Ok(()) — idempotent, not an error
  #
  # ASSUMPTIONS:
  #   1. This card does NOT implement TUI integration (PROV-025), provider routing (PROV-026), or parity testing (PROV-027). Purely the NAPI bridge between Rust OAuth code and TypeScript.
  #   2. All Rust OAuth modules (claude_oauth.rs, claude_oauth_server.rs, claude_headless_login.rs, claude_auth.rs, claude_refreshing_client.rs) are already implemented and tested by PROV-020/021/022/023.
  #
  # ========================================

  Background: User Story
    As a TUI developer
    I want to call Claude OAuth flows from TypeScript via NAPI bindings
    So that the TUI can initiate browser login, headless login, token refresh, token retrieval, and token clearing without leaving the TypeScript/Ink layer

  Scenario: Successful browser OAuth login via NAPI
    Given the Claude browser OAuth flow is configured with a test server
    When TypeScript calls claude_oauth_browser_login()
    Then the Promise should resolve with NapiClaudeTokens
    And the tokens should contain access_token, refresh_token, and expires

  Scenario: Browser OAuth login times out
    Given the Claude browser OAuth flow is configured with a short timeout
    When TypeScript calls claude_oauth_browser_login()
    And no code is submitted before the timeout
    Then the Promise should reject with an error containing "timed out"

  Scenario: Headless login start returns authorize URL and PKCE verifier
    When TypeScript calls claude_oauth_headless_start()
    Then the result should contain an authorize_url string pointing to claude.ai
    And the result should contain a pkce_verifier string

  Scenario: Headless login complete exchanges code for tokens
    Given a headless login flow has been started with a known pkce_verifier
    And the token endpoint accepts authorization code requests
    When TypeScript calls claude_oauth_headless_complete with a valid code_with_state and pkce_verifier
    Then the Promise should resolve with NapiClaudeTokens
    And the tokens should be persisted to claude_auth.json

  Scenario: Headless login complete rejects mismatched state as CSRF
    Given a headless login flow has been started with a known pkce_verifier
    When TypeScript calls claude_oauth_headless_complete with code containing a wrong state
    Then the Promise should reject with an error containing "CSRF" or "state mismatch"

  Scenario: Headless login complete rejects code without hash separator
    Given a headless login flow has been started with a known pkce_verifier
    When TypeScript calls claude_oauth_headless_complete with code containing no hash separator
    Then the Promise should reject with an error containing "Missing state"

  Scenario: Token refresh returns new tokens
    Given valid Claude OAuth tokens exist in claude_auth.json
    And the Claude token endpoint accepts refresh_token requests
    When TypeScript calls claude_oauth_refresh_token with a valid refresh token
    Then the Promise should resolve with NapiClaudeTokens containing a new access_token
    And the refreshed tokens should be persisted to claude_auth.json

  Scenario: Token refresh fails with invalid refresh token
    Given the Claude token endpoint rejects the refresh_token
    When TypeScript calls claude_oauth_refresh_token with an invalid refresh token
    Then the Promise should reject with an error describing the failure

  Scenario: Get tokens returns stored tokens from claude_auth.json
    Given valid Claude OAuth tokens exist in claude_auth.json
    When TypeScript calls claude_oauth_get_tokens()
    Then the result should be NapiClaudeTokens with access_token, refresh_token, and expires populated

  Scenario: Get tokens returns null when no claude_auth.json exists
    Given no claude_auth.json file exists
    When TypeScript calls claude_oauth_get_tokens()
    Then the result should be null

  Scenario: Clear tokens removes stored credentials
    Given valid Claude OAuth tokens exist in claude_auth.json
    When TypeScript calls claude_oauth_clear_tokens()
    Then the operation should succeed
    And subsequent calls to claude_oauth_get_tokens() should return null

  Scenario: Clear tokens is idempotent when no credentials exist
    Given no claude_auth.json file exists
    When TypeScript calls claude_oauth_clear_tokens()
    Then the operation should succeed without error
