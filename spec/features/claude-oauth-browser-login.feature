@http-server
@providers
@authentication
@PROV-021
Feature: Anthropic OAuth browser callback server and CSRF state validation
  """
  New file: codelet/providers/src/claude_oauth_server.rs — Hyper-based HTTP server for Anthropic browser OAuth flow. Routes: GET / (form page with authorize URL and code paste input), POST /submit (receives code, validates state, exchanges tokens), GET /cancel (abort flow), 404 for everything else. Mirrors codex_oauth_server.rs architecture.
  New file: codelet/providers/src/claude_auth.rs — Claude auth persistence module (mirrors codex_auth.rs). ClaudeAuthJson struct with access_token, refresh_token, expires (ms timestamp). write_claude_auth() and read_claude_auth() functions. Path: ~/.config/codelet/claude_auth.json (not ~/.codex/ which is Codex-specific).
  ClaudeOAuthServerConfig struct mirrors OAuthServerConfig from codex_oauth_server.rs — contains: listener (TcpListener), open_browser (bool), timeout_ms (u64), pkce (Option<PkceCodes>). Tests inject port-0 listeners and open_browser=false.
  Key difference from Codex server: instead of /auth/callback receiving a redirect, the server shows a form at GET / where user pastes the code#state string. POST /submit processes the form data. This is because Anthropic's redirect_uri is remote (console.anthropic.com), not local.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Anthropic redirect_uri is https://console.anthropic.com/oauth/code/callback (remote, not localhost) — user must manually copy the authorization code from Anthropic's callback page, unlike Codex which uses a local server redirect
  #   2. Login orchestrator starts a local HTTP server on an ephemeral port that shows a form page for code paste entry — this gives NAPI/TUI consistent UX similar to Codex PROV-013 server pattern
  #   3. Browser opens automatically to claude.ai/oauth/authorize with PKCE parameters using open crate (same as Codex PROV-013)
  #   4. Authorization code arrives in code#state format — state portion must match the PKCE verifier (PROV-020 rule: state=verifier for Anthropic, unlike Codex which uses separate state)
  #   5. Server has a configurable timeout (5 minutes default) — if no code submitted, server shuts down and login fails gracefully
  #   6. After successful exchange, tokens persisted to ~/.config/codelet/claude_auth.json (analogous to Codex auth.json) with access_token, refresh_token, and expires timestamp
  #   7. Server serves HTML success/error/cancel pages — reuse HTML template pattern from codex_oauth.rs (HTML_SUCCESS, html_error, HTML_CANCELLED)
  #   8. Single public entry point: async fn browser_oauth_login() -> Result<ClaudeAuthJson> — mirrors Codex codex_oauth_server::browser_oauth_login() pattern
  #   9. Uses existing PROV-020 functions: generate_pkce(), build_authorize_url(), parse_authorization_code(), exchange_authorization_code(), calculate_expiry() — no duplication
  #   10. Local server form page displays the authorize URL as a clickable link (fallback for environments where open crate fails)
  #
  # EXAMPLES:
  #   1. Happy path: browser_oauth_login() called → PKCE generated → browser opens to claude.ai/oauth/authorize → local server starts showing form → user authorizes on claude.ai → copies code#state from Anthropic callback page → pastes into form → state validated against PKCE verifier → code exchanged for tokens via JSON POST → tokens persisted to claude_auth.json → server shuts down → returns ClaudeAuthJson
  #   2. State mismatch: user pastes code#wrongstate → parse_authorization_code extracts state → state != verifier → server returns HTML error page with CSRF warning → no tokens persisted → server shuts down → returns CSRF error
  #   3. Timeout: user opens browser but never pastes code → 5 minutes elapse → server shuts down cleanly → returns timeout error
  #   4. Token exchange fails: user pastes valid code#state → state validates → exchange POST to console.anthropic.com/v1/oauth/token returns 400 → server shows error page → returns exchange error
  #   5. Cancel via route: user navigates to local server /cancel → server returns cancel page → shuts down → returns cancellation error
  #   6. Code without state: user pastes code without # separator → parse_authorization_code returns (code, None) → no state to validate → treated as missing state validation → error
  #   7. Browser fails to open: open crate fails (headless environment) → logs warning with authorize URL → local server still shows form with clickable link → user can manually navigate → flow continues normally
  #
  # ASSUMPTIONS:
  #   1. This card does NOT implement NAPI bindings (PROV-024), TUI integration (PROV-025), device auth (PROV-022), or token refresh during requests (PROV-023). Purely the Rust-side browser login orchestrator and auth persistence.
  #   2. The open crate is already available in providers/Cargo.toml from PROV-013 (Codex server uses it).
  #
  # ========================================
  Background: User Story
    As a user with a Claude Max/Pro subscription
    I want to complete browser-based OAuth login with code paste
    So that authenticate with my Anthropic subscription from codelet without manually configuring tokens

  Scenario: Successful browser OAuth login with code paste
    Given no existing Claude credentials are available
    When I initiate Claude browser OAuth login
    Then the OAuth server should start on an ephemeral port
    And a PKCE code verifier and S256 challenge should be generated
    And the browser should open to the Claude authorize URL with PKCE parameters
    And the server should serve a form page with the authorize URL as a clickable link
    When the user submits an authorization code with valid state via the form
    Then the code should be parsed from "code#state" format
    And the state should match the PKCE verifier
    And the code should be exchanged for tokens via JSON POST to the Anthropic token endpoint
    And the tokens should be persisted to claude_auth.json with access_token, refresh_token, and expires
    And the OAuth server should shut down
    And the login function should return the Claude auth credentials

  Scenario: Code paste with mismatched state is rejected as CSRF
    Given the Claude OAuth server is running and waiting for code submission
    When the user submits a code with state that does not match the PKCE verifier
    Then the server should return an HTML error page with CSRF warning
    And no tokens should be persisted to claude_auth.json
    And the OAuth server should shut down
    And the login function should return a CSRF error

  Scenario: Login times out after 5 minutes without code submission
    Given the Claude OAuth server is running and waiting for code submission
    When the timeout elapses without receiving a code submission
    Then the OAuth server should shut down cleanly
    And the login function should return a timeout error

  Scenario: Token exchange fails after valid state validation
    Given the Claude OAuth server is running and waiting for code submission
    When the user submits a code with valid state
    And the token exchange POST to the Anthropic token endpoint returns an error
    Then the server should return an HTML error page
    And no tokens should be persisted to claude_auth.json
    And the login function should return the exchange error

  Scenario: User cancels OAuth flow via cancel route
    Given the Claude OAuth server is running and waiting for code submission
    When a request is made to the /cancel route
    Then the server should return a cancel confirmation page
    And the OAuth server should shut down
    And the login function should return a cancellation error

  Scenario: Code without state hash separator is rejected
    Given the Claude OAuth server is running and waiting for code submission
    When the user submits a code without a "#" separator
    Then the server should return an HTML error page indicating missing state
    And no tokens should be persisted to claude_auth.json
    And the login function should return a missing state error

  Scenario: Browser fails to open but server still shows form with link
    Given the browser open command will fail
    When I initiate Claude browser OAuth login
    Then the OAuth server should start and serve the form page
    And the form page should contain the authorize URL as a clickable link
    And the server should log a warning with the authorize URL
    And the login flow should continue waiting for code submission

  Scenario: Server handles 404 requests without shutting down
    Given the Claude OAuth server is running and waiting for code submission
    When a request is made to an unknown path like "/favicon.ico"
    Then the server should return a 404 response
    And the server should remain running and accept further requests

  Scenario: Claude auth persistence writes correct JSON structure
    Given a successful token exchange has returned tokens
    When the tokens are persisted to claude_auth.json
    Then the file should exist at the codelet config directory
    And the JSON should contain "access_token" with the access token value
    And the JSON should contain "refresh_token" with the refresh token value
    And the JSON should contain "expires" as a millisecond timestamp in the future
