@PROV-020
Feature: Claude OAuth core flow for Anthropic subscriptions
  """
  New file: codelet/providers/src/claude_oauth.rs — Anthropic OAuth core module, mirrors codex_oauth.rs structure. Constants, PKCE, authorize URL, code exchange, token refresh, header building, tool name prefixing, URL rewriting. All pure functions + async HTTP calls. Re-uses existing sha2/base64/rand crates.
  Key structural difference from Codex OAuth: Anthropic token endpoint uses JSON body (Content-Type: application/json) not form-encoded. Also no id_token in response — just access_token, refresh_token, expires_in. No JWT account_id extraction needed.
  PKCE code can be shared between codex_oauth.rs and claude_oauth.rs — the generate_pkce() and generate_state() functions are identical. Consider extracting to a shared oauth_common module, or just re-implement (they're small).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Anthropic OAuth uses PKCE (RFC 7636, S256) — CLIENT_ID: 9d1c250a-e61b-44d9-88ed-5944d1962f5e, authorize URL for Max mode: https://claude.ai/oauth/authorize
  #   2. Token endpoint is https://console.anthropic.com/v1/oauth/token — accepts JSON body (NOT form-encoded like Codex/OpenAI)
  #   3. Redirect URI is https://console.anthropic.com/oauth/code/callback — Anthropic hosts the callback page, NOT a local server. User copies authorization code from redirect page.
  #   4. OAuth scope: org:create_api_key user:profile user:inference
  #   5. Authorization code format is 'code#state' — the auth server returns code and state concatenated with '#' separator. The exchange function must parse this.
  #   6. Token exchange request body is JSON with: code, state, grant_type=authorization_code, client_id, redirect_uri, code_verifier
  #   7. Token refresh uses JSON POST to token endpoint with: grant_type=refresh_token, refresh_token, client_id
  #   8. OAuth API requests require: Authorization: Bearer {access_token}, anthropic-beta header with oauth-2025-04-20 and interleaved-thinking-2025-05-14, user-agent: claude-cli/2.1.3 (external, cli), x-api-key header removed
  #   9. Tool names must be prefixed with 'mcp_' when using OAuth mode — tool_use blocks in messages and tool definitions both need this prefix. Responses must strip the prefix.
  #   10. The /v1/messages URL must have ?beta=true query parameter appended when using OAuth mode
  #   11. State parameter in the authorize URL is set to the PKCE verifier (not a separate random value like Codex) — this simplifies CSRF validation since state == verifier
  #   12. Token response contains: access_token, refresh_token, expires_in (seconds). The expires timestamp is calculated as Date.now() + expires_in * 1000.
  #
  # EXAMPLES:
  #   1. PKCE generated with S256: verifier is 43-char random string, challenge is Base64URL(SHA-256(verifier)), authorize URL built with code=true, client_id, response_type=code, redirect_uri, scope, code_challenge, code_challenge_method=S256, state=verifier
  #   2. User pastes 'l0pnTslN...#FgE6g_6k...' — exchange fn splits on '#', sends JSON POST with code=l0pnTslN..., state=FgE6g_6k..., grant_type=authorization_code, client_id, redirect_uri, code_verifier to token endpoint, receives {access_token, refresh_token, expires_in}
  #   3. Token expired: refresh_access_token sends JSON POST with grant_type=refresh_token, refresh_token, client_id to https://console.anthropic.com/v1/oauth/token, receives new {access_token, refresh_token, expires_in}
  #   4. Build OAuth headers: sets Authorization=Bearer {token}, anthropic-beta merges required [oauth-2025-04-20, interleaved-thinking-2025-05-14] with any existing beta headers, sets user-agent=claude-cli/2.1.3 (external, cli), removes x-api-key
  #   5. Tool name prefixing: 'Bash' → 'mcp_Bash' in tool definitions and tool_use content blocks. Response stream 'mcp_Bash' → 'Bash' in tool_result names.
  #   6. URL rewriting for OAuth: https://api.anthropic.com/v1/messages → https://api.anthropic.com/v1/messages?beta=true
  #   7. Code exchange fails (invalid code): token endpoint returns non-200, exchange fn returns error with status and body
  #   8. Authorize URL for 'max' mode: https://claude.ai/oauth/authorize?code=true&client_id=9d1c250a-...&response_type=code&redirect_uri=https%3A%2F%2Fconsole.anthropic.com%2Foauth%2Fcode%2Fcallback&scope=org%3Acreate_api_key+user%3Aprofile+user%3Ainference&code_challenge=...&code_challenge_method=S256&state=...
  #
  # ASSUMPTIONS:
  #   1. PROV-021 (browser callback) and PROV-022 (device auth) are NOT in scope — this card is purely the core OAuth primitives. No local HTTP server, no browser opening, no TUI integration.
  #   2. We use the same CLIENT_ID (9d1c250a-e61b-44d9-88ed-5944d1962f5e) as opencode-anthropic-auth plugin — this is a well-known public OAuth client ID for Claude CLI tools
  #
  # ========================================
  Background: User Story
    As a developer
    I want to use Anthropic OAuth core primitives to build Claude subscription authentication
    So that the same PKCE, token exchange, and refresh patterns proven with Codex OAuth can be reused for Claude Pro/Max subscriptions

  @core
  Scenario: PKCE code verifier meets RFC 7636 requirements
    Given the Anthropic OAuth module is available
    When I generate a PKCE code challenge pair
    Then the verifier should be at least 43 characters long
    And the verifier should contain only unreserved URI characters
    And the challenge method should be "S256"
    And the challenge should be the Base64URL-encoded SHA-256 hash of the verifier

  @core
  Scenario: PKCE challenge is deterministic for a given verifier
    Given a known PKCE verifier string "test_verifier_abc"
    When I compute the S256 challenge twice
    Then both challenges should be identical

  @core
  Scenario: Authorize URL contains all required parameters for Max mode
    Given a PKCE challenge pair has been generated
    When I build the authorize URL for "max" mode
    Then the URL base should be "https://claude.ai/oauth/authorize"
    And the URL should contain parameter "code" with value "true"
    And the URL should contain parameter "client_id" with value "9d1c250a-e61b-44d9-88ed-5944d1962f5e"
    And the URL should contain parameter "response_type" with value "code"
    And the URL should contain parameter "redirect_uri" with value "https://console.anthropic.com/oauth/code/callback"
    And the URL should contain parameter "scope" with value "org:create_api_key user:profile user:inference"
    And the URL should contain parameter "code_challenge" matching the PKCE challenge
    And the URL should contain parameter "code_challenge_method" with value "S256"
    And the URL should contain parameter "state" matching the PKCE verifier

  @core
  Scenario: Authorization code in code-hash-state format is parsed correctly
    Given an authorization response "l0pnTslNFOmT#FgE6g_6khGKF"
    When the code is parsed
    Then the extracted code should be "l0pnTslNFOmT"
    And the extracted state should be "FgE6g_6khGKF"

  @core
  Scenario: Authorization code without hash separator is used as-is
    Given an authorization response "abc123"
    When the code is parsed
    Then the extracted code should be "abc123"
    And the extracted state should be empty

  @core
  Scenario: Authorization code exchanged for tokens at token endpoint
    Given a valid authorization code "test_code" with state "test_state"
    And a PKCE verifier "test_verifier"
    When the code is exchanged for tokens at the token endpoint
    Then the exchange request should be a JSON POST to "https://console.anthropic.com/v1/oauth/token"
    And the request Content-Type should be "application/json"
    And the request body should contain "grant_type" as "authorization_code"
    And the request body should contain "code" as "test_code"
    And the request body should contain "state" as "test_state"
    And the request body should contain "client_id" as "9d1c250a-e61b-44d9-88ed-5944d1962f5e"
    And the request body should contain "redirect_uri" as "https://console.anthropic.com/oauth/code/callback"
    And the request body should contain "code_verifier" as "test_verifier"
    And the response should contain access_token, refresh_token, and expires_in

  @error
  Scenario: Code exchange fails with invalid authorization code
    Given an invalid authorization code "bad_code"
    When the code is exchanged for tokens at the token endpoint
    Then the exchange should fail with an error containing the HTTP status
    And the error should contain the response body

  @core
  Scenario: Token refresh using refresh_token grant
    Given a valid refresh token "existing_refresh_token"
    When the token is refreshed
    Then the refresh request should be a JSON POST to "https://console.anthropic.com/v1/oauth/token"
    And the request body should contain "grant_type" as "refresh_token"
    And the request body should contain "refresh_token" as "existing_refresh_token"
    And the request body should contain "client_id" as "9d1c250a-e61b-44d9-88ed-5944d1962f5e"
    And the response should contain a new access_token, refresh_token, and expires_in

  @core
  Scenario: OAuth headers built with required beta headers
    Given an access token "test_access_token"
    When OAuth headers are built for an API request
    Then the Authorization header should be "Bearer test_access_token"
    And the anthropic-beta header should contain "oauth-2025-04-20"
    And the anthropic-beta header should contain "interleaved-thinking-2025-05-14"
    And the user-agent header should be "claude-cli/2.1.3 (external, cli)"
    And the x-api-key header should be removed

  @core
  Scenario: OAuth headers preserve existing beta headers
    Given an access token "test_access_token"
    And existing beta headers "prompt-caching-2024-07-31"
    When OAuth headers are built for an API request
    Then the anthropic-beta header should contain "oauth-2025-04-20"
    And the anthropic-beta header should contain "interleaved-thinking-2025-05-14"
    And the anthropic-beta header should contain "prompt-caching-2024-07-31"

  # Architecture: mcp_ prefixing is a parity reference — codelet uses native tools
  # (not MCP), so prefixing is not applied in the production request path. These
  # functions exist for parity verification against opencode and future MCP support.
  @core
  Scenario: Tool names prefixed with mcp_ in OAuth mode
    Given a tool named "Bash"
    When the tool name is prefixed for OAuth mode
    Then the prefixed name should be "mcp_Bash"

  @core
  Scenario: Tool names stripped of mcp_ prefix from response
    Given a response tool name "mcp_Bash"
    When the prefix is stripped from the response
    Then the resulting name should be "Bash"

  @core
  Scenario: Messages URL rewritten with beta query parameter
    Given a request URL "https://api.anthropic.com/v1/messages"
    When the URL is rewritten for OAuth mode
    Then the URL should be "https://api.anthropic.com/v1/messages?beta=true"

  @core
  Scenario: Messages URL with existing query parameters gets beta appended
    Given a request URL "https://api.anthropic.com/v1/messages?stream=true"
    When the URL is rewritten for OAuth mode
    Then the URL should contain "beta=true"
    And the URL should preserve "stream=true"

  @core
  Scenario: Non-messages URL is not rewritten
    Given a request URL "https://api.anthropic.com/v1/models"
    When the URL is checked for OAuth rewriting
    Then the URL should remain unchanged

  @core
  Scenario: Token expiry calculated from expires_in seconds
    Given a token response with expires_in of 3600
    When the expiry timestamp is calculated
    Then the expiry should be approximately current time plus 3600 seconds
