@done
@providers
@authentication
@PROV-023
Feature: Anthropic token refresh client and resilient request auth

  """
  New file: codelet/providers/src/claude_refreshing_client.rs — Implements RefreshingClaudeClient struct that wraps reqwest::Client and implements rig's HttpClientExt trait. Contains ClaudeTokenState struct with access_token, refresh_token, token_endpoint_base, and expires_at (Instant). Arc<tokio::sync::RwLock<ClaudeTokenState>> for shared mutable state. ClaudeTokenMode enum: OAuth{token_state} and ApiKey. ~200 lines, mirrors codex/refreshing_client.rs structure.
  Key structural difference from Codex RefreshingClient: Claude's RefreshingClaudeClient is SIMPLER — no URL rewriting needed (rig AnthropicExt::build_uri does ?beta=true), no extra headers like ChatGPT-Account-Id or originator. Only handles Authorization: Bearer header replacement and token refresh. Static headers (anthropic-beta, anthropic-version, user-agent, x-app) remain set at rig client build time.
  ClaudeProvider modification: claude.rs struct fields change from CompletionModel (defaulting to reqwest::Client) to CompletionModel<RefreshingClaudeClient> and from Client to Client<RefreshingClaudeClient>. from_api_key_with_mode_and_model() constructs RefreshingClaudeClient and passes via .http_client() to rig anthropic::ClientBuilder. OAuth mode: new_oauth(), API key mode: new_api_key(). Agent builder's client() type changes correspondingly.
  Token persistence uses async write_claude_auth() from claude_auth.rs (PROV-021). Since HttpClientExt methods return non-async futures (impl Future), persistence is spawned as a fire-and-forget tokio::spawn task — same pattern as codex refreshing_client.rs persist_tokens() using std::thread::spawn for synchronous write_codex_auth(). For Claude, write_claude_auth is async so we use tokio::spawn directly.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Implement HttpClientExt trait for RefreshingClaudeClient that wraps reqwest::Client, intercepting every HTTP request in OAuth mode to check token expiry, refresh if needed, and inject the current Authorization: Bearer header
  #   2. Token expiry tracked using tokio::time::Instant with a 30-second buffer — if now + 30s >= expires_at, token is considered expired and must be refreshed before the request
  #   3. Token refresh uses existing refresh_access_token_at() from claude_oauth.rs (PROV-020) with configurable base URL for testability (production: console.anthropic.com, tests: wiremock)
  #   4. After refresh, updated tokens persisted to claude_auth.json via write_claude_auth() from claude_auth.rs (PROV-021) — persistence is best-effort (errors logged, don't fail the request)
  #   5. RefreshingClaudeClient strips any existing Authorization header from the rig request and injects Authorization: Bearer {current_access_token} — rig's static headers may contain a stale token
  #   6. RefreshingClaudeClient holds token state in Arc<tokio::sync::RwLock<ClaudeTokenState>> for thread-safe interior mutability — tokio RwLock required because refresh is async and guards must be held across .await points
  #   7. Concurrent refresh uses double-check locking: read lock → check expired → drop read → write lock → RE-CHECK expired → refresh only if still expired. Prevents redundant refresh calls when multiple requests detect expiry simultaneously
  #   8. If token refresh fails (network error, invalid refresh_token), the original request is NOT sent — the error propagates up to the caller as rig::http_client::Error
  #   9. RefreshingClaudeClient is used for ALL ClaudeProvider modes (not just OAuth). In API key mode, it operates in pass-through mode (no refresh, no header injection, forwards to reqwest unchanged). This gives one consistent type: CompletionModel<RefreshingClaudeClient>
  #   10. URL rewriting (?beta=true) is NOT done by RefreshingClaudeClient — the rig AnthropicExt::build_uri() already handles this via AnthropicKey::is_oauth_token() detection. RefreshingClaudeClient only handles auth headers and token refresh
  #   11. Static headers (anthropic-beta, anthropic-version, user-agent, x-app) remain set at rig client build time and are NOT modified by RefreshingClaudeClient — only Authorization changes dynamically
  #   12. ClaudeProvider struct fields become CompletionModel<RefreshingClaudeClient> and Client<RefreshingClaudeClient> (anthropic::Client parameterized with custom HTTP backend). ClaudeProvider::from_api_key_with_mode_and_model() constructs the RefreshingClaudeClient and passes it via .http_client()
  #   13. Tokens loaded from disk (via read_claude_auth from PROV-021) should use Some(0) for expires_in_secs to force immediate refresh on first API request — the persisted token may be expired
  #
  # EXAMPLES:
  #   1. Happy path (valid token): ClaudeProvider makes API call → RefreshingClaudeClient reads token state → token has 30min remaining → skips refresh → strips any existing Authorization header → injects Authorization: Bearer {access_token} → forwards to reqwest → response returned
  #   2. Token expired mid-session: ClaudeProvider makes API call → RefreshingClaudeClient detects token expired → calls claude_oauth::refresh_access_token_at() with stored refresh_token → gets new ClaudeTokenResponse → updates in-memory state → persists to claude_auth.json (best-effort) → injects fresh Bearer token → forwards request → response returned
  #   3. Token refresh fails (invalid refresh_token): RefreshingClaudeClient detects expiry → calls refresh_access_token_at() → gets 401 → returns rig::http_client::Error → original API request NOT sent → user sees auth error
  #   4. Streaming request with expired token: RefreshingClaudeClient refreshes token via send_streaming() path → fresh Bearer injected → SSE stream returned with fresh credentials
  #   5. API key mode pass-through: ClaudeProvider::from_api_key_with_mode_and_model() creates RefreshingClaudeClient in ApiKey mode → requests flow through to reqwest unchanged — no token refresh, no header modification, rig's static headers preserved
  #   6. Token within expiry buffer: token expires in 20 seconds, buffer is 30 seconds → now + 30s >= expires_at → proactive refresh triggered → request uses refreshed token
  #   7. Concurrent requests during refresh: two requests arrive while token is expired → first acquires write lock → refreshes → second finds fresh token on re-check → both proceed with same refreshed token → only one HTTP refresh call made
  #   8. Tokens loaded from disk: OAuth tokens exist in claude_auth.json from previous session → ClaudeProvider passes Some(0) for expires_in_secs → token immediately expired → first API request triggers refresh before sending
  #   9. ClaudeProvider integration: from_api_key_with_mode_and_model() in OAuth mode creates RefreshingClaudeClient with initial tokens → passes as .http_client() to rig anthropic::ClientBuilder → rig client type becomes Client<RefreshingClaudeClient> → CompletionModel<RefreshingClaudeClient> → provider methods use it transparently
  #
  # ASSUMPTIONS:
  #   1. This card does NOT implement: tool name prefixing/stripping (mcp_ prefix — tracked as part of the request body transformation which may be handled at a higher layer or in a future card), NAPI bindings (PROV-024), TUI integration (PROV-025), or model routing (PROV-026). Purely the HTTP middleware layer for token refresh and auth header injection.
  #   2. Reuses from PROV-020 (claude_oauth.rs): refresh_access_token_at(), ClaudeTokenResponse. Reuses from PROV-021 (claude_auth.rs): write_claude_auth(), ClaudeAuthJson, calculate_expiry(). No new crate dependencies.
  #
  # ========================================

  Background: User Story
    As a developer using Claude with OAuth subscription
    I want to have my Claude API requests automatically refresh expired tokens and route with correct auth headers
    So that long-running sessions don't break when access tokens expire mid-conversation

  @happy-path
  Scenario: Request with valid token injects Bearer header without refresh
    Given a RefreshingClaudeClient in OAuth mode with a valid access token expiring in 30 minutes
    When the client sends a request to "https://api.anthropic.com/v1/messages"
    Then the Authorization header should be "Bearer {access_token}"
    And no token refresh should occur
    And the request should be forwarded to reqwest successfully

  @happy-path
  Scenario: Expired token is automatically refreshed before request
    Given a RefreshingClaudeClient in OAuth mode with an expired access token
    And a valid refresh token
    When the client sends a request to "https://api.anthropic.com/v1/messages"
    Then the client should refresh the access token via claude_oauth refresh_access_token_at()
    And the refreshed tokens should be persisted to claude_auth.json
    And the request should proceed with the new access token
    And the response should be returned successfully

  @error
  Scenario: Token refresh failure propagates error without sending request
    Given a RefreshingClaudeClient in OAuth mode with an expired access token
    And an invalid refresh token that returns a 401 error
    When the client sends a request to "https://api.anthropic.com/v1/messages"
    Then the client should attempt to refresh the access token
    And the refresh should fail with an authentication error
    And the original API request should NOT be sent
    And the error should propagate to the caller

  @happy-path
  Scenario: Streaming request with expired token refreshes before streaming
    Given a RefreshingClaudeClient in OAuth mode with an expired access token
    And a valid refresh token
    When the client sends a streaming request to "https://api.anthropic.com/v1/messages"
    Then the client should refresh the access token before streaming
    And the streaming response should use the refreshed credentials
    And the SSE stream should be returned successfully

  Scenario: API key mode passes requests through unchanged
    Given a RefreshingClaudeClient in ApiKey mode
    When the client sends a request to "https://api.anthropic.com/v1/messages"
    Then no token refresh should occur
    And the original headers from rig should be preserved
    And the request should be forwarded to reqwest as-is

  Scenario: Token refresh within expiry buffer triggers proactive refresh
    Given a RefreshingClaudeClient in OAuth mode with a token expiring in 20 seconds
    And the expiry buffer is 30 seconds
    When the client sends a request
    Then the client should proactively refresh the token
    And the request should use the refreshed token

  Scenario: Existing Authorization header is replaced with current token
    Given a RefreshingClaudeClient in OAuth mode with a valid access token
    And the original request has a stale Authorization header "Bearer old-stale-token" set by rig
    When the client sends the request
    Then the stale Authorization header should be stripped
    And replaced with "Bearer {current_access_token}"

  Scenario: Static headers are preserved and not modified
    Given a RefreshingClaudeClient in OAuth mode with a valid access token
    And the request has static headers including anthropic-beta and user-agent set by rig
    When the client sends the request
    Then the anthropic-beta header should be preserved unchanged
    And the user-agent header should be preserved unchanged
    And only the Authorization header should be modified

  @integration
  Scenario: ClaudeProvider uses RefreshingClaudeClient for OAuth mode
    Given OAuth tokens with access_token and refresh_token
    When ClaudeProvider::from_api_key_with_mode_and_model() is called in OAuth mode
    Then a RefreshingClaudeClient should be created with OAuth ClaudeTokenMode
    And it should be passed as the HTTP client to rig anthropic::Client<RefreshingClaudeClient>
    And the provider should be able to construct a rig Agent

  Scenario: Tokens loaded from disk trigger immediate refresh via Some(0)
    Given OAuth tokens exist in claude_auth.json from a previous session
    When ClaudeProvider passes Some(0) for expires_in_secs to RefreshingClaudeClient
    Then the token is immediately considered expired
    Given the access token may be expired
    Then the first API request triggers a token refresh before sending

  Scenario: Token persistence is best-effort and does not fail requests
    Given a RefreshingClaudeClient in OAuth mode with an expired access token
    And token persistence to claude_auth.json will fail due to filesystem error
    When the client sends a request that triggers a token refresh
    Then the refresh should succeed and tokens should be updated in memory
    And the persistence failure should be logged
    And the request should still proceed with the refreshed token

  Scenario: Claude auth persistence writes correct JSON structure
    Given a RefreshingClaudeClient in OAuth mode with an expired access token
    And a valid refresh token
    When a token refresh occurs
    Then claude_auth.json should contain access_token from the refresh response
    And claude_auth.json should contain refresh_token from the refresh response
    And claude_auth.json should contain expires calculated from expires_in
