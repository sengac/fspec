@codex-oauth
@PROV-022
Feature: Anthropic device auth flow for headless login
  """
  New file: codelet/providers/src/claude_headless_login.rs — Simple async function that orchestrates: generate PKCE → invoke code-entry callback with authorize URL → validate state → exchange code → persist tokens. ~80-100 lines. No HTTP server, no hyper dependency, no port binding.
  ClaudeHeadlessLoginConfig struct mirrors DeviceAuthConfig pattern from Codex PROV-014: token_endpoint_base (wiremock or production), timeout_ms, pkce (optional injection for tests), code_entry_fn (async callback that receives authorize URL and returns code#state string). Tests inject short timeouts and mock callbacks.
  Reuses from PROV-020 (claude_oauth.rs): generate_pkce(), build_authorize_url(), parse_authorization_code(), exchange_authorization_code(), calculate_expiry(). Reuses from PROV-021 (claude_auth.rs): write_claude_auth(), ClaudeAuthJson. No new crate dependencies.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Anthropic has NO RFC 8628 device authorization endpoints — headless auth is a CLI code-paste flow: display authorize URL, user visits on another device, copies code#state from Anthropic callback page, enters it via callback/stdin
  #   2. Headless login generates PKCE + builds authorize URL using existing PROV-020 functions (generate_pkce, build_authorize_url), then displays the URL for the user to visit on another device
  #   3. Code entry is via an async callback function (not stdin directly) — the callback receives the authorize URL and returns the user-pasted code#state string. This allows NAPI/TUI callers (PROV-024/PROV-025) to provide their own input mechanism
  #   4. State validation: the state portion of the pasted code#state must match the PKCE verifier. Mismatch is a CSRF error (same as PROV-021 rule). Missing state (no # separator) is also an error.
  #   5. Token exchange uses existing PROV-020 exchange_authorization_code() with the configurable base URL (production: console.anthropic.com, tests: wiremock). JSON POST with code, state, grant_type, client_id, redirect_uri, code_verifier.
  #   6. On success, tokens persisted to ~/.config/codelet/claude_auth.json using existing write_claude_auth() from PROV-021 (claude_auth.rs). Returns ClaudeAuthJson with access_token, refresh_token, and expires timestamp — identical output to browser OAuth (PROV-021).
  #   7. Headless login has a configurable timeout — if the callback does not return within the timeout, the flow terminates gracefully with a timeout error
  #   8. Single public entry point: async fn claude_headless_login(config: ClaudeHeadlessLoginConfig) -> Result<ClaudeAuthJson> — mirrors the DeviceAuthConfig pattern from Codex PROV-014 for testability
  #   9. No HTTP server, no browser opening, no port binding — headless login is purely an async function that generates PKCE, invokes the code-entry callback, validates, exchanges, and persists. Much simpler than PROV-021.
  #
  # EXAMPLES:
  #   1. Happy path: claude_headless_login() called → PKCE generated → callback invoked with authorize URL → user returns 'authcode123#verifier456' → parse_authorization_code splits on # → state matches verifier → exchange_authorization_code sends JSON POST → tokens returned → calculate_expiry computes timestamp → write_claude_auth persists to claude_auth.json → returns ClaudeAuthJson
  #   2. State mismatch: user pastes 'code#wrong_state' → parse_authorization_code extracts state → state != PKCE verifier → returns CSRF mismatch error → no tokens persisted
  #   3. Missing state: user pastes code without # separator → parse_authorization_code returns (code, None) → treated as missing state → error returned
  #   4. Timeout: callback blocks indefinitely (user never pastes code) → timeout_ms expires → flow terminates with timeout error → no tokens persisted
  #   5. Token exchange fails: user pastes valid code#state → state validates → exchange POST returns 400 → returns exchange error → no tokens persisted
  #   6. Callback returns empty string: user submits empty code → error before any state validation or exchange → descriptive error
  #   7. Output matches browser OAuth: headless login returns same ClaudeAuthJson struct as claude_browser_oauth_login (PROV-021) — access_token, refresh_token, expires — both flows are interchangeable for downstream consumers
  #
  # ASSUMPTIONS:
  #   1. This card does NOT implement NAPI bindings (PROV-024), TUI integration (PROV-025), or token refresh during requests (PROV-023). Purely the Rust-side headless login function and state validation logic.
  #   2. Anthropic does not expose device authorization endpoints like OpenAI (no /deviceauth/usercode, no /deviceauth/token). The headless flow is a standard OAuth authorization code flow but with manual code paste instead of browser redirect — exactly matching opencode's anthropic auth plugin behavior.
  #
  # ========================================
  Background: User Story
    As a developer using codelet in a headless environment
    I want to authenticate with my Claude Pro/Max subscription without a browser
    So that I can use Claude subscription models from SSH sessions, containers, and headless servers where a browser can't be opened

  Scenario: Successful headless login with code paste
    Given no Claude credentials exist in claude_auth.json
    When the user initiates headless Claude login
    Then PKCE codes should be generated and an authorize URL built
    And the code-entry callback should receive the authorize URL
    When the callback returns a valid code#state string
    Then the state should be validated against the PKCE verifier
    And the authorization code should be exchanged for tokens via JSON POST
    And the tokens should be persisted to claude_auth.json with access_token, refresh_token, and expires
    And the function should return a ClaudeAuthJson

  Scenario: Code paste with mismatched state is rejected as CSRF
    Given a headless Claude login is in progress
    When the callback returns a code#state string with an incorrect state value
    Then the login should fail with a CSRF state mismatch error
    And no tokens should be persisted to claude_auth.json

  Scenario: Code without state hash separator is rejected
    Given a headless Claude login is in progress
    When the callback returns a code without a hash separator
    Then the login should fail with a missing state error
    And no tokens should be persisted to claude_auth.json

  Scenario: Headless login times out when callback blocks
    Given a headless Claude login is configured with a short timeout
    When the code-entry callback blocks without returning
    Then the login should fail with a timeout error
    And no tokens should be persisted to claude_auth.json

  Scenario: Token exchange failure after valid state validation
    Given a headless Claude login is in progress
    When the callback returns a valid code#state string
    And the state validates successfully
    But the token exchange endpoint returns an error
    Then the login should fail with a token exchange error
    And no tokens should be persisted to claude_auth.json

  Scenario: Empty code string is rejected before validation
    Given a headless Claude login is in progress
    When the callback returns an empty string
    Then the login should fail with a descriptive error about empty code
    And no state validation or token exchange should be attempted

  Scenario: Headless login produces same ClaudeAuthJson output as browser OAuth
    Given a headless Claude login completes successfully
    When the tokens are returned
    Then the output should be a ClaudeAuthJson with access_token, refresh_token, and expires
    And the output should be identical in structure to browser OAuth login output from PROV-021
